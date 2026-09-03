use crate::events::{IncomingFileRequest, TransferProgress, TransferStarted};
use crate::protocol::PeerInfo;
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
use tokio_util::sync::CancellationToken;

const CHUNK_SIZE: usize = 64 * 1024;
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

    let cancel = CancellationToken::new();
    let task = tokio::spawn(stream_to_disk(
        socket, file, transfer_id.clone(), size, app.clone(), dest_path, cancel.clone(),
    ));
    tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { task, cancel });
}

async fn stream_to_disk(
    mut socket: TcpStream,
    file: File,
    transfer_id: String,
    size: u64,
    app: AppHandle,
    dest_path: PathBuf,
    cancel: CancellationToken,
) {
    let mut writer = BufWriter::new(file);
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut received: u64 = 0;
    let start = tokio::time::Instant::now();
    let mut last_emit = start;

    let result: std::io::Result<()> = async {
        loop {
            let n = tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "cancelled"));
                }
                n = socket.read(&mut buf) => n?,
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
                    speed_bytes_per_sec: received as f64 / elapsed,
                });
                last_emit = tokio::time::Instant::now();
            }
            if received >= size { break; }
        }
        writer.flush().await
    }.await;

    match result {
        Ok(()) if received == size => {
            let _ = app.emit("transfer-completed", (&transfer_id, dest_path.to_string_lossy().into_owned()));
        }
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            let _ = app.emit("transfer-cancelled", &transfer_id);
            let _ = tokio::fs::remove_file(&dest_path).await;
        }
        _ => {
            let _ = app.emit("transfer-failed", (&transfer_id, "Connection closed before transfer completed"));
            let _ = tokio::fs::remove_file(&dest_path).await; // don't leave a truncated file behind
        }
    }
}

/// The frontend's response to an "incoming-file-request" event.
#[tauri::command]
pub fn respond_to_incoming_file(pending: State<'_, Arc<PendingIncoming>>, transfer_id: String, accept: bool) {
    if let Some(tx) = pending.requests.lock().unwrap().remove(&transfer_id) {
        let _ = tx.send(accept);
    }
}
