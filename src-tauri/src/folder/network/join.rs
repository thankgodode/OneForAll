use crate::protocol::{recv_msg, send_msg, ConnectionKind, ControlMessage, PeerInfo};
use crate::state::{DeviceInfo, SessionState};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::net::TcpStream;

/// Called when the user clicks a discovered host, or connects to one
/// entered manually. Sends a join request, then waits — possibly a
/// while, if the host requires approval — for a decision. Every new
/// connection must open with a `ConnectionKind` frame before anything
/// else, including this one: it's what lets `dispatch::handle_connection`
/// on the host side tell this apart from an incoming file transfer.
#[tauri::command]
pub async fn join_network(
    app: AppHandle,
    device: State<'_, Arc<DeviceInfo>>,
    host_addr: String,
    display_name: String,
) -> Result<PeerInfo, String> {
    let mut socket = TcpStream::connect(&host_addr).await.map_err(|e| e.to_string())?;
    send_msg(&mut socket, &ConnectionKind::Control).await.map_err(|e| e.to_string())?;
    send_msg(&mut socket, &ControlMessage::JoinRequest {
        display_name,
        device_name: device.name.clone(),
        code: None,
    }).await.map_err(|e| e.to_string())?;

    loop {
        match recv_msg::<ControlMessage, _>(&mut socket).await {
            Ok(ControlMessage::JoinPending) => {
                let _ = app.emit("join-pending", ());
            }
            Ok(ControlMessage::JoinAccepted { session_name, peer_id, roster }) => {
                let _ = app.emit("joined-network", (&session_name, &roster));
                let me = PeerInfo { id: peer_id, display_name: "You".into(), device_name: device.name.clone(), addr: host_addr };
                tokio::spawn(listen_for_roster_updates(socket, app));
                return Ok(me);
            }
            Ok(ControlMessage::JoinRejected { reason }) => return Err(reason),
            _ => return Err("Connection closed before a decision was made".into()),
        }
    }
}

/// Runs for as long as this client stays joined — the connection itself
/// IS the membership. If it closes for any reason, the host has already
/// cleaned up its side (see `control::handle_control_connection`); this
/// task's only job is telling the frontend so the UI can update.
async fn listen_for_roster_updates(mut socket: TcpStream, app: AppHandle) {
    loop {
        match recv_msg::<ControlMessage, _>(&mut socket).await {
            Ok(ControlMessage::PeerJoined(p)) => { let _ = app.emit("peer-joined", p); }
            Ok(ControlMessage::PeerLeft { peer_id }) => { let _ = app.emit("peer-left", peer_id); }
            Ok(ControlMessage::Kick { reason }) => { let _ = app.emit("kicked", reason); break; }
            Err(_) => { let _ = app.emit("disconnected-from-network", ()); break; }
            _ => {}
        }
    }
}

/// Host side: resolves a pending join request the frontend showed the
/// user (the "join-requested" event fired from `control.rs`).
#[tauri::command]
pub fn respond_to_join(session: State<'_, Arc<SessionState>>, request_id: String, accept: bool) {
    if let Some(pending) = session.pending.lock().unwrap().remove(&request_id) {
        let _ = pending.responder.send(accept);
    }
}

/// Host side: forcibly disconnects a joined peer. Pushing `Kick` down
/// their outbox and letting them close their own end is enough — no need
/// to reach into the socket from here.
#[tauri::command]
pub fn kick_peer(session: State<'_, Arc<SessionState>>, peer_id: String) {
    if let Some(peer) = session.peers.lock().unwrap().get(&peer_id) {
        let _ = peer.outbox.send(ControlMessage::Kick { reason: "Removed by host".into() });
    }
}
