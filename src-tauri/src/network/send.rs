use crate::events::{TransferProgress, TransferStarted};
use crate::protocol::{send_msg, ConnectionKind, PeerInfo};
use crate::state::{SessionState, TcpState, TransferHandle};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

const CHUNK_SIZE: usize = 64 * 1024;
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

        joins.push(tokio::spawn(async move {
            send_one_file(app, tcp_state, file_path, target, my_peer_id).await;
        }));
    }

    futures::future::join_all(joins).await;
    Ok(())
}

async fn send_one_file(app: AppHandle, tcp_state: Arc<TcpState>, file_path: Arc<String>, target: PeerInfo, my_peer_id: String) {
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
        peer_id: my_peer_id, transfer_id: transfer_id.clone(), filename, size,
    }).await;
    if sent_header.is_err() {
        let _ = app.emit("transfer-failed", (&transfer_id, "Failed to start transfer"));
        return;
    }

    let cancel = CancellationToken::new();
    let task = tokio::spawn(stream_from_disk(socket, (*file_path).clone(), transfer_id.clone(), size, app.clone(), cancel.clone()));
    tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { task, cancel });
}

async fn stream_from_disk(mut socket: TcpStream, file_path: String, transfer_id: String, size: u64, app: AppHandle, cancel: CancellationToken) {
    let Ok(mut file) = File::open(&file_path).await else {
        let _ = app.emit("transfer-failed", (&transfer_id, "Could not reopen file"));
        return;
    };

    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut sent: u64 = 0;
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
                    speed_bytes_per_sec: sent as f64 / elapsed,
                });
                last_emit = tokio::time::Instant::now();
            }
        }
        socket.flush().await
    }.await;

    match result {
        Ok(()) if sent == size => {
            let _ = app.emit("transfer-completed", (&transfer_id, ()));
        }
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            let _ = app.emit("transfer-cancelled", &transfer_id);
        }
        _ => {
            let _ = app.emit("transfer-failed", (&transfer_id, "Connection lost mid-transfer"));
        }
    }
}
