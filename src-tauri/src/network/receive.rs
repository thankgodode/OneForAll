use crate::db::{Db, IncompleteTransfer};
use crate::events::{IncomingFileRequest, TransferProgress, TransferStarted};
use crate::network::TRANSFER_CHUNK_SIZE;
use crate::protocol::{send_msg, ConnectionKind, PeerInfo, PrintJobResult, ResumeOffset};
use crate::state::{AppSettings, PendingIncoming, SessionState, TcpState, TransferHandle};
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
    printer_name: Option<String>,
    copies: Option<u32>,
    app: AppHandle,
) {
    if printer_name.is_some() && !matches!(copies, Some(1..)) {
        crate::network::send::emit_print_status(&app, &transfer_id, "failed", Some("Invalid number of copies".into()));
        return;
    }

    let require_confirmation = if printer_name.is_some() {
        settings.require_confirm_printing()
    } else {
        settings.require_confirmation()
    };
    let mut confirmation_rx = None;
    let mut awaiting_confirmation = false;
    if require_confirmation {
        let (tx, rx) = oneshot::channel();
        {
            let mut requests = pending_incoming.requests.lock().unwrap();
            let confirmation_still_required = if printer_name.is_some() {
                settings.require_confirm_printing()
            } else {
                settings.require_confirmation()
            };

            if confirmation_still_required {
                requests.insert(transfer_id.clone(), crate::state::incoming::PendingIncomingRequest {
                    responder: tx,
                    is_print_job: printer_name.is_some(),
                });
                confirmation_rx = Some(rx);
                awaiting_confirmation = true;
            } else {
                let _ = tx.send(true);
            }
        }
    }

    if printer_name.is_some() || awaiting_confirmation {
        let _ = app.emit("incoming-file-request", IncomingFileRequest {
            transfer_id: transfer_id.clone(),
            filename: filename.clone(),
            size,
            from: from_peer.clone(),
            printer_name: printer_name.clone(),
            copies,
            requires_confirmation: awaiting_confirmation,
        });
    }

    if let Some(rx) = confirmation_rx {
        match rx.await {
            Ok(true) => {}
            _ => {
                if printer_name.is_some() {
                    let _ = crate::protocol::send_msg(&mut socket, &false).await;
                    crate::network::send::emit_print_status(&app, &transfer_id, "rejected", None);
                } else {
                    let _ = app.emit("transfer-rejected", &transfer_id);
                }
                return; // socket drops here; sender sees the connection close
            }
        }
    }

    // Fires exactly once, right before this transfer actually starts —
    // whether or not confirmation was required, this is always the event
    // that creates the row on the frontend (see the note on
    // `events::TransferStarted`).
    let safe_name = sanitize_filename(&filename);
    let dest_path = if printer_name.is_some() {
        std::env::temp_dir().join(format!("oneforall-print-{}-{safe_name}", uuid::Uuid::new_v4()))
    } else {
        let download_dir = settings.download_dir();
        match unique_path(&download_dir, &safe_name).await {
            Ok(path) => path,
            Err(error) => {
                let _ = app.emit("transfer-failed", (&transfer_id, error.to_string()));
                return;
            }
        }
    };

    let file = match tokio::fs::OpenOptions::new().write(true).create_new(true).open(&dest_path).await {
        Ok(file) => file,
        Err(e) => {
            if printer_name.is_some() {
                crate::network::send::emit_print_status(&app, &transfer_id, "failed", Some(format!("Couldn't create temporary print file: {e}")));
            } else {
                let _ = app.emit("transfer-failed", (&transfer_id, e.to_string()));
            }
            return;
        }
    };

    if printer_name.is_some() && crate::protocol::send_msg(&mut socket, &true).await.is_err() {
        let _ = tokio::fs::remove_file(&dest_path).await;
        crate::network::send::emit_print_status(&app, &transfer_id, "failed", Some("Couldn't approve the print job connection".into()));
        return;
    }

    if printer_name.is_some() {
        crate::network::send::emit_print_status(&app, &transfer_id, "receiving", None);
    } else {
        let _ = app.emit("transfer-started", TransferStarted {
            transfer_id: transfer_id.clone(),
            peer_id: from_peer.id.clone(),
            peer_name: from_peer.display_name.clone(),
            filename: filename.clone(),
            size,
            direction: "receive",
            printer_name: None,
        });
    }

    // Persisted BEFORE streaming starts, not after it fails — this is
    // what survives an app restart, not just a connection drop while
    // still running. Removed on completion or explicit cancel; kept on
    // any other failure so a later `ResumeTransfer` connection has
    // something to look up.
    if printer_name.is_none() {
        let _ = db.save_incomplete_transfer(&IncompleteTransfer {
            transfer_id: transfer_id.clone(),
            peer_display_name: from_peer.display_name.clone(),
            peer_device_name: from_peer.device_name.clone(),
            filename: filename.clone(),
            file_path: dest_path.to_string_lossy().into_owned(),
            total_size: size,
            direction: "receive".into(),
            created_at: now_unix(),
            interrupted_at: None,
        });
    }

    let cancel = CancellationToken::new();
    let peer_gone = CancellationToken::new();
    let task = tokio::spawn(stream_to_disk(
        socket, file, transfer_id.clone(), size, 0, app.clone(), dest_path, cancel.clone(), peer_gone.clone(), db.clone(), printer_name, copies,
    ));
    tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { peer_id: from_peer.id, task, cancel, peer_gone });
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
) {
    let record = match db.get_incomplete_transfer(&transfer_id) {
        Ok(Some(r)) => r,
        _ => {
            let _ = app.emit("transfer-failed", (&transfer_id, "No record of this transfer to resume"));
            return;
        }
    };

    // Matched on device_name, not peer_id — a peer that had to rejoin
    // (exactly the scenario this feature handles) gets a brand new
    // peer_id every time (see control.rs), so comparing peer_id here
    // would reject the most common legitimate resume case. device_name
    // is a far more stable signal across a rejoin, if not a perfectly
    // unique one — see the note on IncompleteTransfer in db.rs.
    if record.peer_device_name != from_peer.device_name {
        let _ = app.emit("transfer-failed", (&transfer_id, "This transfer belongs to a different device"));
        return;
    }

    let dest_path = PathBuf::from(&record.file_path);
    let already_have = match tokio::fs::metadata(&dest_path).await {
        Ok(meta) => meta.len(),
        Err(_) => 0, // partial file is gone somehow — resume from scratch rather than fail outright
    };

    if send_msg(&mut socket, &ResumeOffset { bytes_received: already_have }).await.is_err() {
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
    let peer_gone = CancellationToken::new();
    let task = tokio::spawn(stream_to_disk(
        socket, file, transfer_id.clone(), record.total_size, already_have, app.clone(), dest_path, cancel.clone(), peer_gone.clone(), db.clone(), None, None,
    ));
    tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { peer_id: from_peer.id, task, cancel, peer_gone });
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
    peer_gone: CancellationToken,
    db: Arc<Db>,
    printer_name: Option<String>,
    copies: Option<u32>,
) {
    // Explicit capacity, matching TRANSFER_CHUNK_SIZE — without this,
    // BufWriter defaults to an internal 8KB buffer, decoupled from
    // whatever we just read from the socket. Every write_all call here
    // is already TRANSFER_CHUNK_SIZE bytes; giving BufWriter the same
    // capacity keeps its own buffering from doing anything counter to
    // what our chunking already intends, rather than relying on an
    // internal bypass-when-larger-than-buffer behavior to make the
    // mismatch harmless.
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
                _ = peer_gone.cancelled() => {
                    // Deliberately a DIFFERENT ErrorKind than the explicit
                    // cancel above, but handled by the SAME catch-all
                    // branch below as an ordinary connection-lost failure
                    // — the peer leaving the session is exactly the
                    // "interrupted, not abandoned" case resume exists for.
                    return Err(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "peer left the session"));
                }
                n = socket.read(&mut buf) => n?,
            };
            if n == 0 { break; } // peer closed the connection
            writer.write_all(&buf[..n]).await?;
            received += n as u64;

            if printer_name.is_none() && (last_emit.elapsed() >= PROGRESS_EMIT_INTERVAL || received >= size) {
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
    drop(writer);

    match result {
        Ok(()) if received == size => {
            if let Some(printer_name) = printer_name.as_deref() {
                crate::network::send::emit_print_status(&app, &transfer_id, "printing", None);
                let print_result = crate::printer::print_file(&dest_path, printer_name, copies.unwrap_or(1)).await;
                let cleanup_result = tokio::fs::remove_file(&dest_path).await;
                let result = match (print_result, cleanup_result) {
                    (Ok(()), Ok(())) => PrintJobResult { success: true, error: None },
                    (Err(error), _) => PrintJobResult { success: false, error: Some(error) },
                    (Ok(()), Err(error)) => PrintJobResult {
                        success: false,
                        error: Some(format!("Print submitted, but temporary file cleanup failed: {error}")),
                    },
                };
                let _ = send_msg(&mut socket, &result).await;
                if result.success {
                    crate::network::send::emit_print_status(&app, &transfer_id, "completed", None);
                } else {
                    crate::network::send::emit_print_status(&app, &transfer_id, "failed", result.error);
                }
            } else {
                let _ = db.remove_incomplete_transfer(&transfer_id);
                let _ = app.emit("transfer-completed", (&transfer_id, dest_path.to_string_lossy().into_owned()));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            // Explicit user cancel — true abandonment, not a "resume
            // later" situation. Clean up fully.
            if printer_name.is_some() {
                let error = "Print job cancelled".to_string();
                let _ = send_msg(&mut socket, &PrintJobResult { success: false, error: Some(error.clone()) }).await;
                crate::network::send::emit_print_status(&app, &transfer_id, "failed", Some(error));
            } else {
                let _ = db.remove_incomplete_transfer(&transfer_id);
                let _ = app.emit("transfer-cancelled", &transfer_id);
            }
            let _ = tokio::fs::remove_file(&dest_path).await;
        }
        _ => {
            // Covers both an ordinary connection-lost failure AND the
            // peer_gone case above — both are "the other party isn't
            // reachable via this session anymore," and both keep the
            // partial file and DB record so resume stays possible.
            // Stamping interrupted_at here (not at transfer start) is
            // what lets the UI show "interrupted X ago" using the MOST
            // RECENT gap, even across several reconnect attempts.
            if printer_name.is_some() {
                let _ = tokio::fs::remove_file(&dest_path).await;
                let error = "Connection lost while receiving the print job".to_string();
                let _ = send_msg(&mut socket, &PrintJobResult { success: false, error: Some(error.clone()) }).await;
                crate::network::send::emit_print_status(&app, &transfer_id, "failed", Some(error));
            } else {
                let _ = db.mark_transfer_interrupted(&transfer_id, now_unix());
                let _ = app.emit("transfer-failed", (&transfer_id, "Connection lost — the transfer can be resumed once reconnected"));
            }
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
    if let Some(request) = pending.requests.lock().unwrap().remove(&transfer_id) {
        let _ = request.responder.send(accept);
    }
}

/// Lists every transfer this device knows is interrupted — receiving OR
/// sending — so the frontend can render a "resume?" prompt that survives
/// even a full app restart, not just a still-running reconnect.
#[tauri::command]
pub fn list_incomplete_transfers(db: State<'_, Arc<Db>>) -> Vec<IncompleteTransfer> {
    db.list_incomplete_transfers().unwrap_or_default()
}

/// The receiver-initiated resume path — the receiver already has the
/// partial file, so it's the one in the best position to notice "this
/// is incomplete" and reach out, rather than waiting for the original
/// sender to remember and reconnect. Computes the resume offset from
/// the partial file's ACTUAL size on disk (same principle as the
/// sender-initiated path in send.rs — never a separately tracked count),
/// then sends that offset upfront as part of the connection itself, so
/// no back-and-forth is needed before streaming can begin.
#[tauri::command]
pub async fn request_resume(
    app: AppHandle,
    session: State<'_, Arc<SessionState>>,
    tcp_state: State<'_, Arc<TcpState>>,
    db: State<'_, Arc<Db>>,
    my_peer_id: String,
    transfer_id: String,
) -> Result<(), String> {
    let record = db.get_incomplete_transfer(&transfer_id).map_err(|e| e.to_string())?
        .ok_or("No record of this transfer")?;
    if record.direction != "receive" {
        return Err("This transfer was outgoing, not incoming".into());
    }

    let sender = session.find_peer_by_device_name(&record.peer_device_name)
        .ok_or("That device isn't currently connected — rejoin the network first")?;

    let bytes_already_have = tokio::fs::metadata(&record.file_path).await.map(|m| m.len()).unwrap_or(0);

    let tcp_state = tcp_state.inner().clone();
    let db = db.inner().clone();
    tokio::spawn(async move {
        request_resume_from_sender(app, tcp_state, db, record, sender, my_peer_id, bytes_already_have).await;
    });
    Ok(())
}

async fn request_resume_from_sender(
    app: AppHandle,
    tcp_state: Arc<TcpState>,
    db: Arc<Db>,
    record: IncompleteTransfer,
    sender: PeerInfo,
    my_peer_id: String,
    bytes_already_have: u64,
) {
    let transfer_id = record.transfer_id.clone();

    let Ok(mut socket) = TcpStream::connect(&sender.addr).await else {
        let _ = app.emit("transfer-failed", (&transfer_id, "Connection failed"));
        return;
    };

    let sent_header = send_msg(&mut socket, &ConnectionKind::RequestResume {
        peer_id: my_peer_id, transfer_id: transfer_id.clone(), bytes_already_have,
    }).await;
    if sent_header.is_err() {
        let _ = app.emit("transfer-failed", (&transfer_id, "Failed to reconnect"));
        return;
    }

    let dest_path = PathBuf::from(&record.file_path);
    let file = match tokio::fs::OpenOptions::new().append(true).open(&dest_path).await {
        Ok(f) => f,
        Err(e) => {
            let _ = app.emit("transfer-failed", (&transfer_id, e.to_string()));
            return;
        }
    };

    let _ = app.emit("transfer-resumed", (&transfer_id, bytes_already_have));

    let cancel = CancellationToken::new();
    let peer_gone = CancellationToken::new();
    let task = tokio::spawn(stream_to_disk(
        socket, file, transfer_id.clone(), record.total_size, bytes_already_have, app.clone(), dest_path, cancel.clone(), peer_gone.clone(), db.clone(), None, None,
    ));
    tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { peer_id: sender.id, task, cancel, peer_gone });
}
