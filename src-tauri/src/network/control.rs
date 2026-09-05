use crate::protocol::{recv_msg, send_msg, ControlMessage, PeerInfo};
use crate::state::{DeviceInfo, PendingJoin, SessionState};
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

/// Handles one Control connection end-to-end: reads the join request,
/// admits immediately if auto-accept or a matching code applies,
/// otherwise queues it for the frontend to approve — then relays roster
/// events for as long as the peer stays connected. Returning means the
/// peer is gone, whether that's a clean `Leave` or the socket just
/// dying. `device` is THIS device's own identity — it's what lets
/// `JoinAccepted` actually tell the new peer who they connected to,
/// rather than leaving them with nothing but the session name.
pub async fn handle_control_connection(mut socket: TcpStream, addr: SocketAddr, session: Arc<SessionState>, device: Arc<DeviceInfo>, app: AppHandle) {
    let msg: ControlMessage = match recv_msg(&mut socket).await {
        Ok(m) => m,
        Err(_) => return,
    };

    let ControlMessage::JoinRequest { display_name, device_name, code, listen_port } = msg else {
        return; // first message on a Control connection must be a join request
    };

    let request_id = uuid::Uuid::new_v4().to_string();
    // Use the peer's OWN reported listening port, not the source port of
    // this incoming connection — that source port is ephemeral and
    // nothing is listening on it. The IP is still trustworthy (it's
    // where the bytes actually came from); only the port needs replacing.
    let dialable_addr = format!("{}:{}", addr.ip(), listen_port);
    let info = PeerInfo { id: request_id.clone(), display_name, device_name, addr: dialable_addr };

    let auto_accept = *session.auto_accept.lock().unwrap();
    let admitted = if auto_accept || session.code_matches(&code) {
        true
    } else {
        let _ = send_msg(&mut socket, &ControlMessage::JoinPending).await;
        let (tx, rx) = oneshot::channel();
        session.pending.lock().unwrap().insert(request_id.clone(), PendingJoin { info: info.clone(), responder: tx });
        let _ = app.emit("join-requested", &info);
        // waits here — possibly a while — for a human to click Accept or
        // Reject in the UI. Cheap to hold open: this is async I/O, not a
        // blocked thread.
        rx.await.unwrap_or(false)
    };
    session.pending.lock().unwrap().remove(&request_id);

    if !admitted {
        let _ = send_msg(&mut socket, &ControlMessage::JoinRejected { reason: "Declined by host".into() }).await;
        return;
    }

    // Snapshot the roster and notify EXISTING peers before inserting the
    // newcomer — both of these have to happen while the newcomer is still
    // absent from `session.peers`, or two things go wrong: the roster
    // handed back in JoinAccepted below would include the newcomer's own
    // entry (since `session.roster()` reads the live map), and
    // `broadcast` would reach the newcomer's own outbox — which only
    // gets created a few lines below — echoing PeerJoined(info) back
    // down their own connection describing themselves.
    let roster = session.roster();
    session.broadcast(ControlMessage::PeerJoined(info.clone()));

    let (outbox_tx, mut outbox_rx) = mpsc::unbounded_channel();
    session.insert_peer(info.clone(), Some(outbox_tx));

    // Extracted into a local BEFORE the `.await` below on purpose: a
    // `MutexGuard` built inline inside an awaited call's arguments lives
    // until the end of the whole statement (not just the `.clone()`),
    // which spans the `.await` point and makes the guard part of the
    // future's captured state. `std::sync::MutexGuard` isn't `Send`, so
    // that future — and everything that spawns it, all the way out to
    // `hosting.rs`'s `tokio::spawn` — stops being `Send` too. Binding to
    // a local first ends the guard's scope at the `let`, well before the
    // `.await` is ever reached.
    let session_name = session.session_name.lock().unwrap().clone();
    let _ = send_msg(&mut socket, &ControlMessage::JoinAccepted {
        session_name,
        peer_id: info.id.clone(),
        host_device_name: device.name.clone(),
        roster,
    }).await;

    let _ = app.emit("peer-joined", &info);

    let (mut read_half, mut write_half) = socket.into_split();

    // writer: forwards anything queued for this peer (roster updates from
    // OTHER peers joining/leaving, a Kick) down this peer's own socket
    let writer = tokio::spawn(async move {
        while let Some(msg) = outbox_rx.recv().await {
            if send_msg(&mut write_half, &msg).await.is_err() {
                break;
            }
        }
    });

    // reader: this connection staying open IS this peer's membership —
    // its only job is noticing when that stops being true
    loop {
        match recv_msg::<ControlMessage, _>(&mut read_half).await {
            Ok(ControlMessage::Leave) | Err(_) => break,
            Ok(ControlMessage::Ping) => { /* liveness only, no reply required over TCP */ }
            _ => {}
        }
    }

    writer.abort();
    session.remove_peer(&info.id);
    session.broadcast(ControlMessage::PeerLeft { peer_id: info.id.clone() });
    let _ = app.emit("peer-left", &info.id);
}
