use crate::db::{Db, IncompleteTransfer};
use crate::events::{IncomingFileRequest, TransferProgress, TransferStarted};
use crate::network::TRANSFER_CHUNK_SIZE;
use crate::protocol::{send_msg, PeerInfo, ResumeOffset};
use crate::state::{AppSettings, PendingIncoming, TcpState, TransferHandle};
use crate::util::{sanitize_filename, unique_path};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(250);

/// Receives one incoming file. Called from `dispatch::handle_connection`
/// AFTER the sender's membership has already been verified — this
/// function trusts its caller on that front and focuses purely on the
/// transfer: optional confirmation, filename safety, and streaming to
/// disk without buffering the whole file in memory.
pub async fn receive_incoming_transfer(
    mut socket: TcpStream,
    transfer_id: String,
    filename: String,
    size: u64,
    from_peer: PeerInfo,
    settings: Arc<AppSettings>,
    pending_incoming: Arc<PendingIncoming>,
    tcp_state: Arc<TcpState>,
    db: Arc<Db>,
    app: AppHandle,
) {
    if settings.require_confirmation() {
        let (tx, rx) = oneshot::channel();
        pending_incoming.requests.lock().unwrap().insert(transfer_id.clone(), tx);
        let _ = app.emit("incoming-file-request", IncomingFileRequest {
            transfer_id: transfer_id.clone(),
            filename: filename.clone(),
            size,
            from: from_peer.clone(), // cloned — still needed below for transfer-started
        });

        match rx.await {
            Ok(true) => {}
            _ => {
                let _ = app.emit("transfer-rejected", &transfer_id);
                return; // socket drops here; sender sees the connection close
            }
        }
    }

    // Fires exactly once, right before this transfer actually starts —
    // whether or not confirmation was required, this is always the event
    // that creates the row on the frontend (see the note on
    // `events::TransferStarted`).
    let _ = app.emit("transfer-started", TransferStarted {
        transfer_id: transfer_id.clone(),
        peer_id: from_peer.id.clone(),
        peer_name: from_peer.display_name.clone(),
        filename: filename.clone(),
        size,
        direction: "receive",
    });

    let safe_name = sanitize_filename(&filename);
    let download_dir = settings.download_dir();
    let dest_path = match unique_path(&download_dir, &safe_name).await {
        Ok(p) => p,
        Err(e) => {
            let _ = app.emit("transfer-failed", (&transfer_id, e.to_string()));
            return;
        }
    };

    let file = match File::create(&dest_path).await {
        Ok(f) => f,
        Err(e) => {
            let _ = app.emit("transfer-failed", (&transfer_id, e.to_string()));
            return;
        }
    };

    // Persisted BEFORE streaming starts, not after it fails — this is
    // what survives an app restart, not just a connection drop while
    // still running. Removed on completion or explicit cancel; kept on
    // any other failure so a later `ResumeTransfer` connection has
    // something to look up.
    let _ = db.save_incomplete_transfer(&IncompleteTransfer {
        transfer_id: transfer_id.clone(),
        peer_display_name: from_peer.display_name.clone(),
        peer_device_name: from_peer.device_name.clone(),
        filename: filename.clone(),
        file_path: dest_path.to_string_lossy().into_owned(),
        total_size: size,
        direction: "receive".into(),
        created_at: now_unix(),
    });

    let cancel = CancellationToken::new();
    let task = tokio::spawn(stream_to_disk(
        socket, file, transfer_id.clone(), size, 0, app.clone(), dest_path, cancel.clone(), db.clone(),
    ));
    tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { task, cancel });
}

/// Handles a `ConnectionKind::ResumeTransfer` connection — called from
/// `dispatch::handle_connection` after membership is verified, same as
/// a fresh transfer. The difference is entirely in what happens before
/// any bytes move: look up the interrupted record, trust the PARTIAL
/// FILE'S ACTUAL SIZE ON DISK as the resume point (not any separately
/// tracked byte count — that could have drifted from what was really
/// flushed), tell the sender where to seek to, then keep streaming into
/// the same file, appending rather than truncating.
pub async fn resume_incoming_transfer(
    mut socket: TcpStream,
    transfer_id: String,
    from_peer: PeerInfo,
    tcp_state: Arc<TcpState>,
    db: Arc<Db>,
    app: AppHandle,
    dest_path: PathBuf,
    already_have: u64   
) {
    let record = match db.get_incomplete_transfer(&transfer_id) {
        Ok(Some(r)) => r,
        _ => {
            let _ = app.emit("transfer-failed", (&transfer_id, "No record of this transfer to resume"));
            return;
        }
    };

    if record.peer_device_name != from_peer.device_name {
        let _ = app.emit("transfer-failed", (&transfer_id, "This transfer belongs to a different device"));
        return;
    }

    let file = match tokio::fs::OpenOptions::new().append(true).open(&dest_path).await {
        Ok(f) => f,
        Err(e) => {
            let _ = app.emit("transfer-failed", (&transfer_id, e.to_string()));
            return;
        }
    };

    let _ = app.emit("transfer-resumed", (&transfer_id, already_have));

    let cancel = CancellationToken::new();
    let task = tokio::spawn(stream_to_disk(
        socket, file, transfer_id.clone(), record.total_size, already_have, app.clone(), dest_path, cancel.clone(), db.clone(),
    ));
    tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { task, cancel });
}

async fn stream_to_disk(
    mut socket: TcpStream,
    file: File,
    transfer_id: String,
    size: u64,
    starting_received: u64,
    app: AppHandle,
    dest_path: PathBuf,
    cancel: CancellationToken,
    db: Arc<Db>,
) {
    let mut writer = BufWriter::with_capacity(TRANSFER_CHUNK_SIZE, file);
    let mut buf = vec![0u8; TRANSFER_CHUNK_SIZE];
    let mut received: u64 = starting_received;
    let start = tokio::time::Instant::now();
    let mut last_emit = start;

    let result: std::io::Result<()> = async {
        loop {
            let n = tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "cancelled"));
                }

                result = tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    socket.read(&mut buf),
                ) => {
                    match result {
                        Ok(n) => n?,
                        Err(_) => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "peer inactive",
                            ));
                        }
                    }
                },
            };

            if n == 0 { break; } // peer closed the connection
            writer.write_all(&buf[..n]).await?;
            received += n as u64;

            if last_emit.elapsed() >= PROGRESS_EMIT_INTERVAL || received >= size {
                let elapsed = start.elapsed().as_secs_f64().max(0.001);
                let _ = app.emit("transfer-progress", TransferProgress {
                    transfer_id: transfer_id.clone(),
                    bytes_done: received,
                    total_bytes: size,
                    speed_bytes_per_sec: (received - starting_received) as f64 / elapsed,
                });
                last_emit = tokio::time::Instant::now();
            }
            if received >= size { break; }
        }
        writer.flush().await
    }.await;

    match result {
        Ok(()) if received == size => {
            let _ = db.remove_incomplete_transfer(&transfer_id);
            let _ = app.emit("transfer-completed", (&transfer_id, dest_path.to_string_lossy().into_owned()));
        }
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            // Explicit user cancel — true abandonment, not a "resume
            // later" situation. Clean up fully.
            let _ = db.remove_incomplete_transfer(&transfer_id);
            let _ = app.emit("transfer-cancelled", &transfer_id);
            let _ = tokio::fs::remove_file(&dest_path).await;
        }

        Err(e) if e.kind() == std::io::ErrorKind::TimedOut =>{
            println!("Network disconnected :(");
            // let _ = app.emit("transfer-cancelled", &transfer_id);
            let _ = app.emit("transfer-failed",(&transfer_id,"Connection lost — the transfer can be resumed once reconnected"));
        }
        _ => {
            println!("connection lost :(");
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

/// The frontend's response to an "incoming-file-request" event.
#[tauri::command]
pub fn respond_to_incoming_file(pending: State<'_, Arc<PendingIncoming>>, transfer_id: String, accept: bool) {
    if let Some(tx) = pending.requests.lock().unwrap().remove(&transfer_id) {
        let _ = tx.send(accept);
    }
}

/// Lists every transfer this device knows is interrupted — receiving OR
/// sending — so the frontend can render a "resume?" prompt that survives
/// even a full app restart, not just a still-running reconnect.
#[tauri::command]
pub fn list_incomplete_transfers(db: State<'_, Arc<Db>>) -> Vec<IncompleteTransfer> {
    db.list_incomplete_transfers().unwrap_or_default()
}
