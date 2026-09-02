use crate::context::AppContext;
use crate::network::{control::handle_control_connection, receive::receive_incoming_transfer};
use crate::protocol::{recv_msg, ConnectionKind};
use std::net::SocketAddr;
use tauri::AppHandle;
use tokio::net::TcpStream;

/// Entry point for every connection accepted on the hosting socket. Reads
/// the first frame to decide whether this is a long-lived Control
/// connection or a one-shot file Transfer, then hands off accordingly.
pub async fn handle_connection(mut socket: TcpStream, addr: SocketAddr, ctx: AppContext, app: AppHandle) {
    let kind: ConnectionKind = match recv_msg(&mut socket).await {
        Ok(k) => k,
        Err(_) => return,
    };

    match kind {
        ConnectionKind::Control => {
            handle_control_connection(socket, addr, ctx.session, app).await;
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
                ctx.settings, ctx.pending_incoming, ctx.tcp, app,
            ).await;
        }
    }
}
