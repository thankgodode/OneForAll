use crate::context::AppContext;
use crate::network::send::stream_from_disk;
use crate::network::{
    control::handle_control_connection,
    receive::{receive_incoming_transfer, resume_incoming_transfer},
};
use crate::protocol::{recv_msg, ConnectionKind};
use crate::state::TransferHandle;
use std::net::SocketAddr;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

/// Entry point for every connection accepted on the hosting socket. Reads
/// the first frame to decide whether this is a long-lived Control
/// connection, a fresh file Transfer, or a ResumeTransfer continuing an
/// interrupted one, then hands off accordingly.
pub async fn handle_connection(mut socket: TcpStream, addr: SocketAddr, ctx: AppContext, app: AppHandle) {
    let kind: ConnectionKind = match recv_msg(&mut socket).await {
        Ok(k) => k,
        Err(_) => return,
    };

    match kind {
        ConnectionKind::Control => {
            handle_control_connection(socket, addr, ctx.session, ctx.device, app).await;
        }
        ConnectionKind::Transfer { peer_id, transfer_id, filename, size } => {
            // The membership check happens here, before a single byte of
            // file data is read. This is the actual enforcement point —
            // the UI only offering joined peers as recipients is what
            // makes it hard to *try* sending to a non-member; this is
            // what makes it impossible to *succeed* even against a
            // hand-crafted connection that skips the UI entirely.
            let Some(from_peer) = ctx.session.peer_info(&peer_id) else { return };
            receive_incoming_transfer(
                socket, transfer_id, filename, size, from_peer,
                ctx.settings, ctx.pending_incoming, ctx.tcp, ctx.db, app,
            ).await;
        }
        ConnectionKind::ResumeTransfer { peer_id, transfer_id } => {
            // Same membership check as a fresh Transfer — a resume is
            // still just a peer sending you bytes, and still needs to be
            // someone currently in the session, not just someone who
            // once was.
            let Some(from_peer) = ctx.session.peer_info(&peer_id) else { return };
            resume_incoming_transfer(socket, transfer_id, from_peer, ctx.tcp, ctx.db, app).await;
        }
        ConnectionKind::TriggerResend{file_path, transfer_id, size, starting_sent} => {
            println!("Resend triggered.");

            let _ = app.emit("transfer-resumed", (&transfer_id, starting_sent));

            println!("Path: {:?}", file_path);

            let cancel = CancellationToken::new();
            let db = ctx.db;
            let task = tokio::spawn(stream_from_disk(
                socket, file_path, transfer_id.clone(), size, starting_sent, app.clone(), cancel.clone(), db.clone(),
            ));

            let tcp_state = ctx.tcp;
            tcp_state.active_transfers.lock().unwrap().insert(transfer_id, TransferHandle { task, cancel });
        }
    }
}
