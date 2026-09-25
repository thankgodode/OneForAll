use crate::printing::printer;
use crate::protocol::{recv_msg, send_msg, ControlMessage, DisconnectReason, PeerInfo};
use crate::state::{DeviceInfo, PendingJoin, SessionState, TcpState};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);

/// Handles one Control connection end-to-end: reads the join request,
/// admits immediately if auto-accept or a matching code applies,
/// otherwise queues it for the frontend to approve — then relays roster
/// events for as long as the peer stays connected. Returning means the
/// peer is gone, whether that's a clean `Leave` or the socket just
/// dying. `device` is THIS device's own identity — it's what lets
/// `JoinAccepted` actually tell the new peer who they connected to,
/// rather than leaving them with nothing but the session name. `tcp` is
/// what lets a departing peer's active transfers get stopped (kept
/// resumable, not abandoned) rather than silently continuing to stream
/// to someone no longer in the session.
pub async fn handle_control_connection(mut socket: TcpStream, addr: SocketAddr, session: Arc<SessionState>, device: Arc<DeviceInfo>, tcp: Arc<TcpState>, app: AppHandle) {
    let msg: ControlMessage = match recv_msg(&mut socket).await {
        Ok(m) => m,
        Err(_) => return,
    };

    let local_printers = printer::list_local_printers();

    let ControlMessage::JoinRequest { display_name, device_name, code, listen_port , printers} = msg else {
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
    let heartbeat_tx = outbox_tx.clone(); // cloned BEFORE the move below — see the ticker task further down
    let reply_tx = outbox_tx.clone(); // cloned for the SAME reason — see the Ping arm in the read loop below
    session.insert_peer(info.clone(), Some(outbox_tx), printers.clone());

    println!("Joined peer printer: {:?}", printers);
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
        printers: local_printers
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

    // Periodic Ping keeps the connection looking "alive" to a plain TCP
    // read even when nothing else is happening — without this, a joined
    // peer who's just idle (not sending anything) would eventually trip
    // the timeout below on the OTHER side even though nothing's actually
    // wrong. Piggybacks on the same outbox/writer above, no new plumbing.
    let heartbeat = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(HEARTBEAT_INTERVAL);
        loop {
            ticker.tick().await;
            if heartbeat_tx.send(ControlMessage::Ping).is_err() { break; }
        }
    });

    // reader: this connection staying open IS this peer's membership —
    // its only job is noticing when that stops being true. Wrapped in a
    // timeout so a silently-dead connection (cable pulled, laptop lid
    // closed — no FIN, no RST, just nothing ever arriving again) is
    // caught within HEARTBEAT_TIMEOUT instead of hanging indefinitely on
    // a read that TCP alone may never fail on its own. `touch_peer` on
    // every successful receive (not just Ping) is what backs
    // `seconds_since_seen` for anyone querying this peer's liveness.
    let disconnect_reason = loop {
        match tokio::time::timeout(HEARTBEAT_TIMEOUT, recv_msg::<ControlMessage, _>(&mut read_half)).await {
            Ok(Ok(ControlMessage::Leave)) => {
                break DisconnectReason::Left
            },
            Ok(Ok(ControlMessage::Ping)) => {
                // Reply immediately, on the SAME outbox the periodic
                // heartbeat task already writes through — this is what
                // makes Pong an actual acknowledgment of a specific Ping
                // rather than the dead, never-constructed variant it was
                // before. Not load-bearing for liveness detection itself
                // (any traffic already counts, via touch_peer below), but
                // it's what an RTT measurement or a stricter "did they
                // specifically ack" check would hook into later.
                session.touch_peer(&info.id);
                let _ = reply_tx.send(ControlMessage::Pong);
            }
            Ok(Ok(_)) => session.touch_peer(&info.id), // Pong, or anything else — any traffic at all counts as "still alive"
            Ok(Err(_)) | Err(_) => {
                // A real error or a timeout both mean the same thing here:
                // no orderly Leave arrived. The only way to tell "the host
                // kicked us" apart from a genuine silent drop is the flag
                // set (if at all) by kick_peer BEFORE this loop ever saw
                // anything wrong — nothing about the connection itself
                // distinguishes the two.
                break if session.was_kicked(&info.id) { DisconnectReason::Kicked } else { DisconnectReason::TimedOut };
            }
        }
    };
    heartbeat.abort();

    writer.abort();
    session.remove_peer(&info.id);
    // Any transfer this peer had in flight — with the host, or with a
    // THIRD peer (direct peer-to-peer connections don't route through
    // this control connection at all, so they wouldn't otherwise notice
    // this peer just left) — stops here. Kept resumable, not deleted:
    // see the peer_gone vs cancel distinction on TransferHandle.
    tcp.stop_transfers_with_peer(&info.id);
    session.broadcast(ControlMessage::PeerLeft { peer_id: info.id.clone(), reason: disconnect_reason.clone() });
    let _ = app.emit("peer-left", (&info.id, &disconnect_reason));
}
