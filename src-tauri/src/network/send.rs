use crate::db::{Db, IncompleteTransfer};
use crate::events::{TransferProgress, TransferStarted};
use crate::network::TRANSFER_CHUNK_SIZE;
use crate::network::receive::resume_incoming_transfer;
use crate::protocol::{recv_msg, send_msg, ConnectionKind, PeerInfo, ResumeOffset};
use crate::state::{SessionState, TcpState, TransferHandle};
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(250);

/// Sends `file_path` to every id in `target_peer_ids`, concurrently — one
/// TCP connection and one transfer_id per recipient, so a slow or
/// failing peer never blocks delivery to the others. This is the "send
/// to all / send to selected" fan-out: the caller passes one id for a
/// single recipient, or the full roster for "send to all."
#[tauri::command]
pub async fn send_file_to_peers(
    app: AppHandle,
    session: State<'_, Arc<SessionState>>,
    tcp_state: State<'_, Arc<TcpState>>,
    db: State<'_, Arc<Db>>,
    my_peer_id: String,
    file_path: String,
    target_peer_ids: Vec<String>,
) -> Result<(), String> {
    let roster = session.roster();
    let file_path = Arc::new(file_path);

    let mut joins = Vec::new();
    for target_id in target_peer_ids {
        let Some(target) = roster.iter().find(|p| p.id == target_id).cloned() else { continue };
        let app = app.clone();
        let file_path = file_path.clone();
        let my_peer_id = my_peer_id.clone();
        let tcp_state = tcp_state.inner().clone();
        let db = db.inner().clone();

        joins.push(tokio::spawn(async move {
            send_one_file(app, tcp_state, db, file_path, target, my_peer_id).await;
        }));
    }

    futures::future::join_all(joins).await;
    Ok(())
}

async fn send_one_file(app: AppHandle, tcp_state: Arc<TcpState>, db: Arc<Db>, file_path: Arc<String>, target: PeerInfo, my_peer_id: String) {
    let transfer_id = uuid::Uuid::new_v4().to_string();

    let Ok(metadata) = tokio::fs::metadata(&*file_path).await else {
        let _ = app.emit("transfer-failed", (&transfer_id, "File no longer exists"));
        return;
    };
    let filename = Path::new(&*file_path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let size = metadata.len();

    // Emitted before connecting, on purpose — the frontend has no other
    // way to learn this transfer_id exists (see the note on
    // `events::TransferStarted`). If the connection below fails, the row
    // this creates is what `transfer-failed` then updates, rather than
    // there being no row to update at all.
    let _ = app.emit("transfer-started", TransferStarted {
        transfer_id: transfer_id.clone(),
        peer_id: target.id.clone(),
        peer_name: target.display_name.clone(),
        filename: filename.clone(),
        size,
        direction: "send",
    });

    let Ok(mut socket) = TcpStream::connect(&target.addr).await else {
        let _ = app.emit("transfer-failed", (&transfer_id, "Connection failed"));
        return;
    };

    let sent_header = send_msg(&mut socket, &ConnectionKind::Transfer {
        peer_id: my_peer_id, transfer_id: transfer_id.clone(), filename: filename.clone(), size,
    }).await;
    if sent_header.is_err() {
        let _ = app.emit("transfer-failed", (&transfer_id, "Failed to start transfer"));
        return;
    }

    // Persisted only once the connection genuinely succeeded and
    // streaming is about to begin — a peer simply being unreachable
    // isn't "interrupted mid-transfer," it's just a failed attempt with
    // nothing sent yet, so it isn't kept as a resumable record.
    let _ = db.save_incomplete_transfer(&IncompleteTransfer {
        transfer_id: transfer_id.clone(),
        peer_display_name: target.display_name.clone(),
        peer_device_name: target.device_name.clone(),
        filename,
        file_path: (*file_path).clone(),
        total_size: size,
        direction: "send".into(),
        created_at: now_unix(),
    });

    let cancel = CancellationToken::new();
    let task = tokio::spawn(stream_from_disk(socket, (*file_path).clone(), transfer_id.clone(), size, 0, app.clone(), cancel.clone(), db.clone()));
    tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { task, cancel });
}

/// Reconnects to continue a transfer that was interrupted. Looked up by
/// `transfer_id` in the local ledger; the target peer is re-resolved by
/// CURRENT roster membership (matched on device_name — see the note on
/// `db::IncompleteTransfer` — since the peer's session peer_id may well
/// have changed if they had to rejoin in between).
#[tauri::command]
pub async fn resume_transfer(
    app: AppHandle,
    session: State<'_, Arc<SessionState>>,
    tcp_state: State<'_, Arc<TcpState>>,
    db: State<'_, Arc<Db>>,
    my_peer_id: String,
    transfer_id: String,
) -> Result<(), String> {
    let record = db.get_incomplete_transfer(&transfer_id).map_err(|e| e.to_string())?
        .ok_or("No record of this transfer")?;
    if record.direction == "send" {
        return Err("This transfer was incoming, not outgoing".into());
    }

    let roster = session.roster();
    let target = roster.iter().find(|p| p.device_name == record.peer_device_name).cloned()
        .ok_or("That device isn't currently connected — rejoin the network first")?;

    let tcp_state = tcp_state.inner().clone();
    let db = db.inner().clone();
    tokio::spawn(async move {
        resume_one_file(app, tcp_state, db, record, target).await;
    });
    Ok(())
}

async fn resume_one_file(app: AppHandle, tcp_state: Arc<TcpState>, db: Arc<Db>, record: IncompleteTransfer, target: PeerInfo) {
    let transfer_id = record.transfer_id.clone();

    let Ok(mut socket) = TcpStream::connect(&target.addr).await else {
        let _ = app.emit("transfer-failed", (&transfer_id, "Connection failed"));
        return;
    };

    let dest_path = PathBuf::from(&record.file_path);
    let already_have = match tokio::fs::metadata(&dest_path).await {
        Ok(meta) => meta.len(),
        Err(_) => 0, // partial file is gone somehow — resume from scratch rather than fail outright
    };
    
    let sent_header = send_msg(&mut socket, &ConnectionKind::ResumeTransfer {
        transfer_id: transfer_id.clone(), starting_sent:already_have, 
    }).await;
    
    if sent_header.is_err() {
        let _ = app.emit("transfer-failed", (&transfer_id, "Failed to reconnect"));
        return;
    }

    let _ = app.emit("transfer-resumed", (&transfer_id, already_have));

    let cancel = CancellationToken::new();
    let task = tokio::spawn(resume_incoming_transfer(socket, transfer_id.clone(), target, tcp_state.clone(), db, app,dest_path,already_have));

    tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { task, cancel });
}

pub async fn stream_from_disk(
    mut socket: TcpStream,
    file_path: String,
    transfer_id: String,
    size: u64,
    starting_sent: u64,
    app: AppHandle,
    cancel: CancellationToken,
    db: Arc<Db>,
) {
    let Ok(mut file) = File::open(&file_path).await else {
        let _ = app.emit("transfer-failed", (&transfer_id, "Could not reopen file"));
        return;
    };
    if starting_sent > 0 {
        if let Err(e) = file.seek(SeekFrom::Start(starting_sent)).await {
            let _ = app.emit("transfer-failed", (&transfer_id, e.to_string()));
            return;
        }
    }

    let mut buf = vec![0u8; TRANSFER_CHUNK_SIZE];
    let mut sent: u64 = starting_sent;
    let start = tokio::time::Instant::now();
    let mut last_emit = start;

    let result: std::io::Result<()> = async {
        loop {
            let n = tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "cancelled"));
                }
                n = file.read(&mut buf) => n?,
            };
            if n == 0 { break; }
            socket.write_all(&buf[..n]).await?;
            sent += n as u64;

            if last_emit.elapsed() >= PROGRESS_EMIT_INTERVAL || sent >= size {
                let elapsed = start.elapsed().as_secs_f64().max(0.001);
                let _ = app.emit("transfer-progress", TransferProgress {
                    transfer_id: transfer_id.clone(),
                    bytes_done: sent,
                    total_bytes: size,
                    speed_bytes_per_sec: (sent - starting_sent) as f64 / elapsed,
                });
                last_emit = tokio::time::Instant::now();
            }
        }
        socket.flush().await
    }.await;

    match result {
        Ok(()) if sent == size => {
            let _ = db.remove_incomplete_transfer(&transfer_id);
            let _ = app.emit("transfer-completed", (&transfer_id, ()));
        }
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            // Explicit user cancel — true abandonment.
            let _ = db.remove_incomplete_transfer(&transfer_id);
            let _ = app.emit("transfer-cancelled", &transfer_id);
        }
        _ => {
            // Connection lost, not explicitly cancelled — kept resumable
            // on purpose; see the matching note in receive.rs.
            let _ = app.emit("transfer-failed", (&transfer_id, "Connection lost — the transfer can be resumed once reconnected"));
        }
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
