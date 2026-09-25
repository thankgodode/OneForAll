use crate::protocol::{recv_msg, send_msg, ConnectionKind, ControlMessage, DisconnectReason, PeerInfo, HOST_PEER_ID};
use crate::state::{ClientState, DeviceInfo, SessionState, TcpState};
use crate::printing::printer;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);

/// Called when the user clicks a discovered host, or connects to one
/// entered manually. `my_listen_port` MUST come from an already-completed
/// `listener::start_listening` call — it's what lets the host hand out a
/// dialable address for THIS device to everyone else in the roster,
/// instead of the ephemeral source port of this very connection.
#[tauri::command]
pub async fn join_network(
    app: AppHandle,
    device: State<'_, Arc<DeviceInfo>>,
    client: State<'_, Arc<ClientState>>,
    session: State<'_, Arc<SessionState>>,
    host_addr: String,
    display_name: String,
    my_listen_port: u16,
    tcp_state: State<'_, Arc<TcpState>>,
) -> Result<PeerInfo, String> {
    let mut socket = TcpStream::connect(&host_addr).await.map_err(|e| e.to_string())?;
    send_msg(&mut socket, &ConnectionKind::Control).await.map_err(|e| e.to_string())?;
    send_msg(&mut socket, &ControlMessage::JoinRequest {
        display_name,
        device_name: device.name.clone(),
        code: None,
        listen_port: my_listen_port,
    }).await.map_err(|e| e.to_string())?;

    loop {
        match recv_msg::<ControlMessage, _>(&mut socket).await {
            Ok(ControlMessage::JoinPending) => {
                let _ = app.emit("join-pending", ());
            }
            Ok(ControlMessage::JoinAccepted { session_name, peer_id, host_device_name, roster }) => {
                let host_info = PeerInfo {
                    id: HOST_PEER_ID.into(),
                    display_name: "Host".into(),
                    device_name: host_device_name,
                    addr: host_addr.clone(),
                };
                // The frontend gets the host's real info here — this is
                // the only point in the whole flow where it does. Without
                // it, a joined client's device list would show every
                // other peer but never the host it's actually connected
                // to, since `roster` itself never includes the host (see
                // control.rs — the host never inserts itself as a member
                // of its own session).
                let _ = app.emit("joined-network", (&session_name, &host_info, &roster));
                let me = PeerInfo {
                    id: peer_id.clone(),
                    display_name: "You".into(),
                    device_name: device.name.clone(),
                    addr: format!("127.0.0.1:{my_listen_port}"), // not used for dialing self; kept for shape consistency
                };

                // Mirror the roster into this device's own SessionState so
                // `dispatch::handle_connection`'s membership check works
                // the same way here as it does on the host — including
                // the host itself under HOST_PEER_ID, since the host
                // never goes through JoinRequest to get a "real" id.
                session.insert_peer_mirror(host_info);
                for peer in &roster {
                    session.insert_peer_mirror(peer.clone());
                }

                // Split so the connection can be read from (roster updates)
                // and written to (a future Leave) independently. The write
                // half lives behind an mpsc channel — same "outbox" shape
                // as the host side's PeerHandle — so `leave_network` can
                // send through it without this function needing to keep
                // running or expose the raw socket.
                let (read_half, mut write_half) = socket.into_split();
                let (tx, mut rx) = mpsc::unbounded_channel();

                let local_printers = printer::list_local_printers();
                
                let _ = tx.send(ControlMessage::PrinterList {
                    peer_id: peer_id.clone(),
                    printers: local_printers.clone(),
                });
                
                let heartbeat_tx = tx.clone(); // cloned BEFORE the move below
                let reply_tx = tx.clone(); // cloned for the SAME reason — see the Ping arm in listen_for_roster_updates
                *client.outbox.lock().unwrap() = Some(tx);
                *client.my_peer_id.lock().unwrap() = Some(peer_id.clone());

                tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        if send_msg(&mut write_half, &msg).await.is_err() {
                            break;
                        }
                    }
                });

                // Same heartbeat as the host side (see control.rs) —
                // keeps the connection looking alive to a plain TCP read
                // during idle periods, and detects the host silently
                // vanishing (no FIN, no RST — cable pulled, laptop
                // closed) within HEARTBEAT_TIMEOUT instead of hanging.
                let heartbeat = tokio::spawn(async move {
                    let mut ticker = tokio::time::interval(HEARTBEAT_INTERVAL);
                    loop {
                        ticker.tick().await;
                        if heartbeat_tx.send(ControlMessage::Ping).is_err() { break; }
                    }
                });

                let client_for_cleanup = client.inner().clone();
                let session_for_updates = session.inner().clone();
                let tcp = tcp_state.inner().clone();
                tokio::spawn(async move {
                    listen_for_roster_updates(read_half, app, client_for_cleanup, session_for_updates, reply_tx,tcp).await;
                    heartbeat.abort();
                });

                return Ok(me);
            }
            Ok(ControlMessage::JoinRejected { reason }) => return Err(reason),
            _ => return Err("Connection closed before a decision was made".into()),
        }
    }
}

/// Runs for as long as this client stays joined — the connection itself
/// IS the membership. Keeps the local mirrored roster (`session`) in
/// sync as peers come and go, and clears `ClientState` on exit so
/// `leave_network` doesn't try to send through a connection that's
/// already gone.
async fn listen_for_roster_updates(mut read_half: OwnedReadHalf, app: AppHandle, client: Arc<ClientState>, session: Arc<SessionState>, reply_tx: mpsc::UnboundedSender<ControlMessage>,tcp:Arc<TcpState>) {
    loop {
        let received = tokio::time::timeout(HEARTBEAT_TIMEOUT, recv_msg::<ControlMessage, _>(&mut read_half)).await;
        match received {
            Ok(Ok(ControlMessage::PeerJoined(p))) => {
                session.touch_peer(HOST_PEER_ID);
                session.insert_peer_mirror(p.clone());
                let _ = app.emit("peer-joined", p);
            }
            Ok(Ok(ControlMessage::PeerLeft { peer_id, reason })) => {
                session.touch_peer(HOST_PEER_ID);
                session.remove_peer(&peer_id);
                let _ = app.emit("peer-left", (peer_id, reason));
            }
            Ok(Ok(ControlMessage::Kick { reason })) => { let _ = app.emit("kicked", reason); break; }
            Ok(Ok(ControlMessage::Ping)) => {
                // Same as the host side — reply on the same outbox the
                // periodic heartbeat task already writes through, making
                // Pong an actual acknowledgment rather than a variant
                // that's declared but never sent.
                session.touch_peer(HOST_PEER_ID);
                let _ = reply_tx.send(ControlMessage::Pong);
            }
            Ok(Ok(ControlMessage::PrinterList {
                peer_id,
                printers,
            })) => {
                session.touch_peer(HOST_PEER_ID);

                session.set_peer_printers(&peer_id, printers.clone());

                let _ = app.emit(
                    "printers-updated",
                    (peer_id, printers),
                );

                println!("Printer list arm");
            }
            Ok(Ok(_)) => session.touch_peer(HOST_PEER_ID), // Pong, or anything else — any traffic counts as "still alive"
            Ok(Err(_)) | Err(_) => {
                // Same reasoning as the host side: a real error and a
                // timeout both just mean "no orderly Leave arrived" — the
                // host going silent (crash, network drop, no FIN/RST at
                // all) is exactly the case DisconnectReason::TimedOut is
                // for, distinct from the explicit Kick handled above.
                let _ = app.emit("disconnected-from-network", DisconnectReason::TimedOut);
                break;
            }
        }
    }

    tcp.stop_all_transfers();
    client.clear();
    session.clear_all_peers();
}

/// Sends a clean `Leave` down the still-open control connection, then
/// drops the sender — the writer task in `join_network` ends naturally
/// once the channel closes after forwarding it. Optimistic on the local
/// side: `ClientState` clears immediately rather than waiting for the
/// host to acknowledge, since there's nothing meaningful to wait for.
#[tauri::command]
pub fn leave_network(client: State<'_, Arc<ClientState>>) {
    if let Some(tx) = client.outbox.lock().unwrap().take() {
        let _ = tx.send(ControlMessage::Leave);
    }
    *client.my_peer_id.lock().unwrap() = None;
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
/// to reach into the socket from here. Marks `kicked` BEFORE sending, so
/// that once the connection actually ends, `handle_control_connection`
/// can report `DisconnectReason::Kicked` instead of it looking like an
/// ordinary timeout.
#[tauri::command]
pub fn kick_peer(session: State<'_, Arc<SessionState>>, peer_id: String) {
    session.mark_kicked(&peer_id);
    if let Some(peer) = session.peers.lock().unwrap().get(&peer_id) {
        if let Some(outbox) = &peer.outbox {
            let _ = outbox.send(ControlMessage::Kick { reason: "Removed by host".into() });
        }
    }
}
