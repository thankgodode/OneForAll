use crate::db::{Db, IncompleteTransfer};
use crate::events::{TransferProgress, TransferStarted};
use crate::network::TRANSFER_CHUNK_SIZE;
use crate::protocol::{recv_msg, send_msg, ConnectionKind, PeerInfo, PrintJobResult, ResumeOffset};
use crate::state::{SessionState, TcpState, TransferHandle};
use std::io::SeekFrom;
use std::path::Path;
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
            send_one_file(app, tcp_state, db, file_path, target, my_peer_id, None, None, None).await;
        }));
    }

    futures::future::join_all(joins).await;
    Ok(())
}

#[tauri::command]
pub async fn send_print_job(
    app: AppHandle,
    session: State<'_, Arc<SessionState>>,
    tcp_state: State<'_, Arc<TcpState>>,
    db: State<'_, Arc<Db>>,
    my_peer_id: String,
    file_path: String,
    target_peer_id: String,
    printer_name: String,
    copies: u32,
    transfer_id: String,
) -> Result<String, String> {
    if copies == 0 {
        return Err("Number of copies must be at least 1".into());
    }
    let target = session.roster().into_iter()
        .find(|peer| peer.id == target_peer_id)
        .ok_or("That device is no longer connected")?;
    if !target.printers.iter().any(|printer| printer == &printer_name) {
        return Err("That printer is no longer available on the selected device".into());
    }

    tokio::spawn(send_one_file(
        app,
        tcp_state.inner().clone(),
        db.inner().clone(),
        Arc::new(file_path),
        target,
        my_peer_id,
        Some(printer_name),
        Some(copies),
        Some(transfer_id.clone()),
    ));
    Ok(transfer_id)
}

async fn send_one_file(
    app: AppHandle,
    tcp_state: Arc<TcpState>,
    db: Arc<Db>,
    file_path: Arc<String>,
    target: PeerInfo,
    my_peer_id: String,
    printer_name: Option<String>,
    copies: Option<u32>,
    supplied_transfer_id: Option<String>,
) {
    let transfer_id = supplied_transfer_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let Ok(metadata) = tokio::fs::metadata(&*file_path).await else {
        emit_send_failure(&app, &transfer_id, printer_name.is_some(), "File no longer exists");
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
    if printer_name.is_none() {
        let _ = app.emit("transfer-started", TransferStarted {
            transfer_id: transfer_id.clone(),
            peer_id: target.id.clone(),
            peer_name: target.display_name.clone(),
            filename: filename.clone(),
            size,
            direction: "send",
            printer_name: None,
        });
    }

    let Ok(mut socket) = TcpStream::connect(&target.addr).await else {
        emit_send_failure(&app, &transfer_id, printer_name.is_some(), "Connection failed");
        return;
    };

    let sent_header = send_msg(&mut socket, &ConnectionKind::Transfer {
        peer_id: my_peer_id, transfer_id: transfer_id.clone(), filename: filename.clone(), size,
        printer_name: printer_name.clone(),
        copies,
    }).await;
    if sent_header.is_err() {
        emit_send_failure(&app, &transfer_id, printer_name.is_some(), "Failed to start transfer");
        return;
    }

    if printer_name.is_some() {
        match recv_msg::<bool, _>(&mut socket).await {
            Ok(true) => {}
            Ok(false) => {
                emit_print_status(&app, &transfer_id, "rejected", None);
                return;
            }
            Err(_) => {
                emit_print_status(&app, &transfer_id, "failed", Some("Print job approval connection was lost".into()));
                return;
            }
        }
        emit_print_status(&app, &transfer_id, "sending", None);
    }

    // Persisted only once the connection genuinely succeeded and
    // streaming is about to begin — a peer simply being unreachable
    // isn't "interrupted mid-transfer," it's just a failed attempt with
    // nothing sent yet, so it isn't kept as a resumable record.
    if printer_name.is_none() {
        let _ = db.save_incomplete_transfer(&IncompleteTransfer {
        transfer_id: transfer_id.clone(),
        peer_display_name: target.display_name.clone(),
        peer_device_name: target.device_name.clone(),
        filename,
        file_path: (*file_path).clone(),
        total_size: size,
        direction: "send".into(),
        created_at: now_unix(),
        interrupted_at: None,
        });
    }

    let cancel = CancellationToken::new();
    let peer_gone = CancellationToken::new();
    let task = tokio::spawn(stream_from_disk(
        socket, (*file_path).clone(), transfer_id.clone(), size, 0, app.clone(), cancel.clone(), peer_gone.clone(), db.clone(), printer_name.is_some(),
    ));
    tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { peer_id: target.id, task, cancel, peer_gone });
}

fn emit_send_failure(app: &AppHandle, transfer_id: &str, is_print_job: bool, reason: &str) {
    if is_print_job {
        emit_print_status(app, transfer_id, "failed", Some(reason.to_owned()));
    } else {
        let _ = app.emit("transfer-failed", (transfer_id, reason));
    }
}

pub(crate) fn emit_print_status(app: &AppHandle, transfer_id: &str, status: &'static str, error: Option<String>) {
    let _ = app.emit("print-job-status", crate::events::PrintJobStatus {
        transfer_id: transfer_id.to_owned(),
        status,
        error,
    });
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
        return Err("This transfer was outgoing, not incoming".into());
    }

    let target = session.find_peer_by_device_name(&record.peer_device_name)
        .ok_or("That device isn't currently connected — rejoin the network first")?;

    let tcp_state = tcp_state.inner().clone();
    let db = db.inner().clone();
    tokio::spawn(async move {
        resume_one_file(app, tcp_state, db, record, target, my_peer_id).await;
    });
    Ok(())
}

async fn resume_one_file(app: AppHandle, tcp_state: Arc<TcpState>, db: Arc<Db>, record: IncompleteTransfer, target: PeerInfo, my_peer_id: String) {
    let transfer_id = record.transfer_id.clone();

    let Ok(mut socket) = TcpStream::connect(&target.addr).await else {
        let _ = app.emit("transfer-failed", (&transfer_id, "Connection failed"));
        return;
    };

    let sent_header = send_msg(&mut socket, &ConnectionKind::ResumeTransfer {
        peer_id: my_peer_id, transfer_id: transfer_id.clone(),
    }).await;
    if sent_header.is_err() {
        let _ = app.emit("transfer-failed", (&transfer_id, "Failed to reconnect"));
        return;
    }

    let offset: ResumeOffset = match recv_msg(&mut socket).await {
        Ok(o) => o,
        Err(_) => {
            let _ = app.emit("transfer-failed", (&transfer_id, "Host couldn't resume this transfer"));
            return;
        }
    };

    let _ = app.emit("transfer-resumed", (&transfer_id, offset.bytes_received));

    let cancel = CancellationToken::new();
    let peer_gone = CancellationToken::new();
    let task = tokio::spawn(stream_from_disk(
        socket, record.file_path, transfer_id.clone(), record.total_size, offset.bytes_received, app.clone(), cancel.clone(), peer_gone.clone(), db.clone(), false,
    ));
    tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { peer_id: target.id, task, cancel, peer_gone });
}

/// The other half of resume — this side is entirely passive. Called from
/// `dispatch::handle_connection` when a `ConnectionKind::RequestResume`
/// arrives: some peer we originally SENT this file to is now asking us
/// to continue from a byte offset THEY computed (their partial file's
/// real size — see the matching request_resume command in receive.rs).
/// No response needed before streaming begins; unlike sender-initiated
/// resume, the offset already arrived as part of the connection itself.
pub async fn serve_resume_request(
    socket: TcpStream,
    transfer_id: String,
    bytes_already_have: u64,
    from_peer: PeerInfo,
    tcp_state: Arc<TcpState>,
    db: Arc<Db>,
    app: AppHandle,
) {
    let record = match db.get_incomplete_transfer(&transfer_id) {
        Ok(Some(r)) => r,
        _ => {
            let _ = app.emit("transfer-failed", (&transfer_id, "No record of this transfer"));
            return;
        }
    };
    if record.direction != "send" {
        let _ = app.emit("transfer-failed", (&transfer_id, "This wasn't a transfer we sent"));
        return;
    }
    // Same identity check as ResumeTransfer, just from the sender's side
    // this time — device_name, not peer_id, for the reasons noted
    // throughout (a reconnected peer gets a fresh peer_id every join).
    if record.peer_device_name != from_peer.device_name {
        let _ = app.emit("transfer-failed", (&transfer_id, "This transfer belongs to a different device"));
        return;
    }
    if bytes_already_have > record.total_size {
        let _ = app.emit("transfer-failed", (&transfer_id, "Resume offset is larger than the file itself"));
        return;
    }

    let _ = app.emit("transfer-resumed", (&transfer_id, bytes_already_have));

    let cancel = CancellationToken::new();
    let peer_gone = CancellationToken::new();
    let task = tokio::spawn(stream_from_disk(
        socket, record.file_path, transfer_id.clone(), record.total_size, bytes_already_have, app.clone(), cancel.clone(), peer_gone.clone(), db.clone(), false,
    ));
    tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { peer_id: from_peer.id, task, cancel, peer_gone });
}

async fn stream_from_disk(
    mut socket: TcpStream,
    file_path: String,
    transfer_id: String,
    size: u64,
    starting_sent: u64,
    app: AppHandle,
    cancel: CancellationToken,
    peer_gone: CancellationToken,
    db: Arc<Db>,
    is_print_job: bool,
) {
    let Ok(mut file) = File::open(&file_path).await else {
        emit_send_failure(&app, &transfer_id, is_print_job, "Could not reopen file");
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
                _ = peer_gone.cancelled() => {
                    // Different ErrorKind than the explicit cancel above,
                    // but the same catch-all handling below — the peer
                    // leaving the session is "interrupted," not "abandoned."
                    return Err(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "peer left the session"));
                }
                n = file.read(&mut buf) => n?,
            };
            if n == 0 { break; }
            socket.write_all(&buf[..n]).await?;
            sent += n as u64;

            if !is_print_job && (last_emit.elapsed() >= PROGRESS_EMIT_INTERVAL || sent >= size) {
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
            if is_print_job {
                match recv_msg::<PrintJobResult, _>(&mut socket).await {
                    Ok(result) if result.success => emit_print_status(&app, &transfer_id, "completed", None),
                    Ok(result) => emit_print_status(
                        &app,
                        &transfer_id,
                        "failed",
                        result.error.or_else(|| Some("The printer rejected the job".into())),
                    ),
                    Err(error) => emit_print_status(
                        &app,
                        &transfer_id,
                        "failed",
                        Some(format!("Lost the printer's result: {error}")),
                    ),
                }
            } else {
                let _ = db.remove_incomplete_transfer(&transfer_id);
                let _ = app.emit("transfer-completed", (&transfer_id, ()));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            // Explicit user cancel — true abandonment.
            if is_print_job {
                emit_print_status(&app, &transfer_id, "failed", Some("Print job cancelled".into()));
            } else {
                let _ = db.remove_incomplete_transfer(&transfer_id);
                let _ = app.emit("transfer-cancelled", &transfer_id);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut =>{
            if is_print_job {
                emit_print_status(&app, &transfer_id, "failed", Some("Connection lost while sending print job".into()));
            } else {
                let _ = app.emit("transfer-cancelled", &transfer_id);
                let _ = app.emit("transfer-failed",(&transfer_id,"Connection lost — the transfer can be resumed once reconnected"));
            }
        }
        _ => {
            // Covers both an ordinary connection-lost failure AND the
            // peer_gone case above — kept resumable on purpose; see the
            // matching note in receive.rs.
            if is_print_job {
                emit_print_status(&app, &transfer_id, "failed", Some("Connection lost while sending print job".into()));
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
