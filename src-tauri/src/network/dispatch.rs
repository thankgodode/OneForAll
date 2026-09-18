use crate::context::AppContext;
use crate::network::{
    control::handle_control_connection,
    receive::{receive_incoming_transfer, resume_incoming_transfer},
    send::serve_resume_request,
};
use crate::protocol::{recv_msg, ConnectionKind};
use std::net::SocketAddr;
use tauri::AppHandle;
use tokio::net::TcpStream;

/// Entry point for every connection accepted on the hosting socket. Reads
/// the first frame to decide whether this is a long-lived Control
/// connection, a fresh file Transfer, or one of the two resume
/// directions (sender-initiated ResumeTransfer, or receiver-initiated
/// RequestResume), then hands off accordingly.
pub async fn handle_connection(mut socket: TcpStream, addr: SocketAddr, ctx: AppContext, app: AppHandle) {
    let kind: ConnectionKind = match recv_msg(&mut socket).await {
        Ok(k) => k,
        Err(_) => return,
    };

    match kind {
        ConnectionKind::Control => {
            handle_control_connection(socket, addr, ctx.session, ctx.device, ctx.tcp, app).await;
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
        ConnectionKind::RequestResume { peer_id, transfer_id, bytes_already_have } => {
            // The receiver-initiated direction: WE are the original
            // sender here, being asked to continue from an offset the
            // other side already computed. Same membership check as
            // everything else — being asked to resume doesn't skip it.
            let Some(from_peer) = ctx.session.peer_info(&peer_id) else { return };
            serve_resume_request(socket, transfer_id, bytes_already_have, from_peer, ctx.tcp, ctx.db, app).await;
        }
    }
}
