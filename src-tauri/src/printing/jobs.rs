use crate::printing::printer::{list_local_printers, print_file};
use crate::protocol::{recv_msg, send_msg, ConnectionKind, PeerInfo, PrintJobResponse};
use crate::state::{PendingPrintJobs, SessionState};
use crate::util::sanitize_filename;
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PrintJobUpdate {
    job_id: String,
    filename: String,
    peer_id: String,
    peer_name: String,
    printer_id: String,
    printer_name: String,
    direction: &'static str,
    status: &'static str,
    error: Option<String>,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct IncomingPrintJob {
    job_id: String,
    filename: String,
    size: u64,
    peer_id: String,
    from: PeerInfo,
    printer_name: String,
}

fn emit_status(
    app: &AppHandle,
    job_id: &str,
    filename: &str,
    peer: &PeerInfo,
    printer_id: &str,
    printer_name: &str,
    direction: &'static str,
    status: &'static str,
    error: Option<String>,
) {
    let _ = app.emit(
        "print-job-status",
        PrintJobUpdate {
            job_id: job_id.to_string(),
            filename: filename.to_string(),
            peer_id: peer.id.clone(),
            peer_name: peer.display_name.clone(),
            printer_id: printer_id.to_string(),
            printer_name: printer_name.to_string(),
            direction,
            status,
            error,
        },
    );
}

#[tauri::command]
pub async fn send_print_job(
    app: AppHandle,
    session: State<'_, Arc<SessionState>>,
    my_peer_id: String,
    file_path: String,
    target_peer_id: String,
    printer_id: String,
) -> Result<(), String> {
    let target = session
        .peer_info(&target_peer_id)
        .ok_or("The selected peer is no longer connected")?;
    let (owner, printer) = session
        .find_printer_owner(&printer_id)
        .ok_or("The selected printer is no longer available")?;
    if owner.id != target_peer_id {
        return Err("The selected printer does not belong to this peer".into());
    }

    let path = Path::new(&file_path);
    let metadata = tokio::fs::metadata(path).await.map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("Select a regular file to print".into());
    }
    let filename = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .ok_or("The selected file has no name")?;
    let job_id = uuid::Uuid::new_v4().to_string();
    emit_status(
        &app,
        &job_id,
        &filename,
        &target,
        &printer_id,
        &printer.name,
        "send",
        "pending",
        None,
    );

    tokio::spawn(async move {
        send_print_job_to_peer(
            app,
            target,
            printer_id,
            printer.name,
            filename,
            file_path,
            metadata.len(),
            my_peer_id,
            job_id,
        )
        .await;
    });
    Ok(())
}

async fn send_print_job_to_peer(
    app: AppHandle,
    target: PeerInfo,
    printer_id: String,
    printer_name: String,
    filename: String,
    file_path: String,
    size: u64,
    my_peer_id: String,
    job_id: String,
) {
    let result = async {
        let mut file = tokio::fs::File::open(&file_path)
            .await
            .map_err(|e| e.to_string())?;
        let mut socket = TcpStream::connect(&target.addr)
            .await
            .map_err(|e| e.to_string())?;
        send_msg(
            &mut socket,
            &ConnectionKind::PrintJob {
                peer_id: my_peer_id,
                job_id: job_id.clone(),
                printer_id: printer_id.clone(),
                filename: filename.clone(),
                size,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

        match recv_msg::<PrintJobResponse, _>(&mut socket)
            .await
            .map_err(|e| e.to_string())?
        {
            PrintJobResponse::Accepted => {}
            PrintJobResponse::Rejected { reason } => {
                return Err(format!("PRINT_JOB_REJECTED:{reason}"))
            }
            PrintJobResponse::Failed { reason } => return Err(reason),
            _ => return Err("The recipient sent an invalid print-job response".into()),
        }

        emit_status(
            &app,
            &job_id,
            &filename,
            &target,
            &printer_id,
            &printer_name,
            "send",
            "sending",
            None,
        );
        let mut limited_file = (&mut file).take(size);
        let copied = tokio::io::copy(&mut limited_file, &mut socket)
            .await
            .map_err(|e| e.to_string())?;
        drop(limited_file);
        if copied != size {
            return Err("The selected file changed while it was being sent".into());
        }

        match recv_msg::<PrintJobResponse, _>(&mut socket)
            .await
            .map_err(|e| e.to_string())?
        {
            PrintJobResponse::Printing => {
                emit_status(
                    &app,
                    &job_id,
                    &filename,
                    &target,
                    &printer_id,
                    &printer_name,
                    "send",
                    "printing",
                    None,
                );
                match recv_msg::<PrintJobResponse, _>(&mut socket)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    PrintJobResponse::Completed => Ok(()),
                    PrintJobResponse::Failed { reason } => Err(reason),
                    _ => Err("The recipient sent an invalid print-job result".into()),
                }
            }
            PrintJobResponse::Failed { reason } => Err(reason),
            _ => Err("The recipient sent an invalid print-job result".into()),
        }
    }
    .await;

    match result {
        Ok(()) => emit_status(
            &app,
            &job_id,
            &filename,
            &target,
            &printer_id,
            &printer_name,
            "send",
            "completed",
            None,
        ),
        Err(reason) => {
            let (status, reason) = match reason.strip_prefix("PRINT_JOB_REJECTED:") {
                Some(reason) => ("rejected", reason.to_string()),
                None => ("failed", reason),
            };
            emit_status(
                &app,
                &job_id,
                &filename,
                &target,
                &printer_id,
                &printer_name,
                "send",
                status,
                Some(reason),
            );
        }
    }
}

pub async fn receive_print_job(
    mut socket: TcpStream,
    job_id: String,
    printer_id: String,
    filename: String,
    size: u64,
    from_peer: PeerInfo,
    pending_jobs: Arc<PendingPrintJobs>,
    app: AppHandle,
) {
    let Some(printer) = list_local_printers()
        .into_iter()
        .find(|p| p.id == printer_id)
    else {
        let reason = "The selected printer is no longer installed".to_string();
        let _ = send_msg(
            &mut socket,
            &PrintJobResponse::Rejected {
                reason: reason.clone(),
            },
        )
        .await;
        return;
    };

    let (tx, rx) = oneshot::channel();
    let duplicate = {
        let mut requests = pending_jobs.requests.lock().unwrap();
        if requests.contains_key(&job_id) {
            true
        } else {
            requests.insert(job_id.clone(), tx);
            false
        }
    };
    if duplicate {
        let reason = "A print job with this ID is already awaiting approval".to_string();
        let _ = send_msg(&mut socket, &PrintJobResponse::Rejected { reason }).await;
        return;
    }
    let _ = app.emit(
        "incoming-print-job",
        IncomingPrintJob {
            job_id: job_id.clone(),
            filename: filename.clone(),
            size,
            peer_id: from_peer.id.clone(),
            from: from_peer.clone(),
            printer_name: printer.name.clone(),
        },
    );
    emit_status(
        &app,
        &job_id,
        &filename,
        &from_peer,
        &printer_id,
        &printer.name,
        "receive",
        "pending",
        None,
    );

    if !matches!(rx.await, Ok(true)) {
        let reason = "Print job rejected by recipient".to_string();
        let _ = send_msg(
            &mut socket,
            &PrintJobResponse::Rejected {
                reason: reason.clone(),
            },
        )
        .await;
        emit_status(
            &app,
            &job_id,
            &filename,
            &from_peer,
            &printer_id,
            &printer.name,
            "receive",
            "rejected",
            Some(reason),
        );
        return;
    }

    let safe_name = sanitize_filename(&filename);
    let temp_id = uuid::Uuid::new_v4();
    let file_path = std::env::temp_dir().join(format!("oneforall-print-{temp_id}-{safe_name}"));
    let mut file_options = std::fs::OpenOptions::new();
    file_options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        file_options.mode(0o600);
    }
    let mut file = match file_options.open(&file_path) {
        Ok(file) => tokio::fs::File::from_std(file),
        Err(error) => {
            let reason = format!("Couldn't create temporary print file: {error}");
            emit_status(
                &app,
                &job_id,
                &filename,
                &from_peer,
                &printer_id,
                &printer.name,
                "receive",
                "failed",
                Some(reason.clone()),
            );
            let _ = send_msg(&mut socket, &PrintJobResponse::Failed { reason }).await;
            return;
        }
    };
    if send_msg(&mut socket, &PrintJobResponse::Accepted)
        .await
        .is_err()
    {
        let _ = tokio::fs::remove_file(&file_path).await;
        return;
    }
    emit_status(
        &app,
        &job_id,
        &filename,
        &from_peer,
        &printer_id,
        &printer.name,
        "receive",
        "sending",
        None,
    );
    let mut limited_socket = (&mut socket).take(size);
    let received = tokio::io::copy(&mut limited_socket, &mut file).await;
    drop(limited_socket);
    let receive_result = match received {
        Ok(received) if received == size => file.flush().await.map_err(|e| e.to_string()),
        Ok(_) => Err("The connection ended before the complete document arrived".into()),
        Err(error) => Err(error.to_string()),
    };
    drop(file);

    if let Err(reason) = receive_result {
        let _ = tokio::fs::remove_file(&file_path).await;
        finish_failed(
            &mut socket,
            &app,
            &job_id,
            &filename,
            &from_peer,
            &printer_id,
            &printer.name,
            reason,
        )
        .await;
        return;
    }

    if send_msg(&mut socket, &PrintJobResponse::Printing)
        .await
        .is_err()
    {
        let _ = tokio::fs::remove_file(&file_path).await;
        return;
    }
    emit_status(
        &app,
        &job_id,
        &filename,
        &from_peer,
        &printer_id,
        &printer.name,
        "receive",
        "printing",
        None,
    );

    let path = file_path.to_string_lossy().into_owned();
    let printer_name = printer.name.clone();
    let printed = tokio::task::spawn_blocking(move || print_file(&printer_name, &path, 1))
        .await
        .map_err(|e| e.to_string())
        .and_then(|result| result);
    let _ = tokio::fs::remove_file(&file_path).await;

    match printed {
        Ok(()) => {
            emit_status(
                &app,
                &job_id,
                &filename,
                &from_peer,
                &printer_id,
                &printer.name,
                "receive",
                "completed",
                None,
            );
            let _ = send_msg(&mut socket, &PrintJobResponse::Completed).await;
        }
        Err(reason) => {
            finish_failed(
                &mut socket,
                &app,
                &job_id,
                &filename,
                &from_peer,
                &printer_id,
                &printer.name,
                reason,
            )
            .await
        }
    }
}

async fn finish_failed(
    socket: &mut TcpStream,
    app: &AppHandle,
    job_id: &str,
    filename: &str,
    peer: &PeerInfo,
    printer_id: &str,
    printer_name: &str,
    reason: String,
) {
    emit_status(
        app,
        job_id,
        filename,
        peer,
        printer_id,
        printer_name,
        "receive",
        "failed",
        Some(reason.clone()),
    );
    let _ = send_msg(socket, &PrintJobResponse::Failed { reason }).await;
}

#[tauri::command]
pub fn respond_to_print_job(
    pending_jobs: State<'_, Arc<PendingPrintJobs>>,
    job_id: String,
    accept: bool,
) -> Result<(), String> {
    let sender = pending_jobs
        .requests
        .lock()
        .unwrap()
        .remove(&job_id)
        .ok_or("Print job is no longer awaiting a response")?;
    sender
        .send(accept)
        .map_err(|_| "Print job is no longer awaiting a response".into())
}
