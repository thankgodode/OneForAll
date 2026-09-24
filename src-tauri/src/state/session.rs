use crate::protocol::{ControlMessage, PeerInfo};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::printing::printer::{PrinterInfo};

/// A peer this device considers a member of the current session.
/// `outbox` is only `Some` when this device is the one that ACCEPTED
/// this peer's join — i.e. this device is acting as host for that
/// connection, and can push messages down it. When a joined CLIENT
/// mirrors its own copy of the roster (see `network::join`), it has no
/// direct connection to those other peers, so `outbox` is `None` — the
/// entry exists purely so `is_member`/`peer_info` work identically
/// whether this device is the host or a joined client.
///
/// `last_seen` and `kicked` back the heartbeat mechanism (see
/// control.rs / join.rs): the connection-handling loop updates
/// `last_seen` on every message it receives, and checks `kicked` at
/// disconnect time to report an honest `DisconnectReason` instead of
/// treating every disconnect as an unexplained timeout.
pub struct PeerHandle {
    pub info: PeerInfo,
    pub outbox: Option<UnboundedSender<ControlMessage>>,
    pub printers: Mutex<Vec<PrinterInfo>>,
    pub last_seen: Mutex<Instant>,
    pub kicked: AtomicBool,
}

/// A join request sent to the frontend for approval but not yet decided.
/// Resolving `responder` is what lets the connection task waiting on it
/// proceed — see `network::control::handle_control_connection`.
pub struct PendingJoin {
    pub info: PeerInfo,
    pub responder: oneshot::Sender<bool>,
}

pub struct SessionState {
    pub code: Mutex<Option<String>>,
    pub auto_accept: Mutex<bool>,
    pub session_name: Mutex<String>,
    pub peers: Mutex<HashMap<String, PeerHandle>>,
    pub pending: Mutex<HashMap<String, PendingJoin>>,
}

impl SessionState {
    pub fn new(session_name: impl Into<String>) -> Self {
        Self {
            code: Mutex::new(None),
            auto_accept: Mutex::new(false),
            session_name: Mutex::new(session_name.into()),
            peers: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
        }
    }

    pub fn roster(&self) -> Vec<PeerInfo> {
        self.peers.lock().unwrap().values().map(|p| p.info.clone()).collect()
    }

    pub fn is_member(&self, peer_id: &str) -> bool {
        self.peers.lock().unwrap().contains_key(peer_id)
    }

    pub fn peer_info(&self, peer_id: &str) -> Option<PeerInfo> {
        self.peers.lock().unwrap().get(peer_id).map(|p| p.info.clone())
    }

    /// Finds a currently-connected peer by device_name rather than
    /// peer_id — used for resuming a transfer, where the persisted
    /// record's peer_id may be stale (a peer that reconnected gets a
    /// fresh one every join; see db::IncompleteTransfer). Shared by both
    /// resume directions: the sender resuming a send, and the receiver
    /// requesting the rest of a receive.
    pub fn find_peer_by_device_name(&self, device_name: &str) -> Option<PeerInfo> {
        self.peers.lock().unwrap().values().map(|p| &p.info).find(|p| p.device_name == device_name).cloned()
    }

    pub fn insert_peer(&self, info: PeerInfo, outbox: Option<UnboundedSender<ControlMessage>>) {
        self.peers.lock().unwrap().insert(info.id.clone(), PeerHandle {
            printers: Mutex::new(Vec::new()),
            info, outbox, last_seen: Mutex::new(Instant::now()), kicked: AtomicBool::new(false),
        });
    }

    /// Convenience for the mirrored-roster case described on `PeerHandle`
    /// — used by a joined client, never by a host accepting a real join.
    pub fn insert_peer_mirror(&self, info: PeerInfo) {
        self.insert_peer(info, None);
    }

    pub fn remove_peer(&self, peer_id: &str) {
        self.peers.lock().unwrap().remove(peer_id);
    }

    /// Used when THIS device's own membership ends (kicked, disconnected,
    /// or left) — clears the whole mirrored roster at once, as opposed to
    /// `remove_peer` which drops a single other peer.
    pub fn clear_all_peers(&self) {
        self.peers.lock().unwrap().clear();
    }

    pub fn broadcast(&self, msg: ControlMessage) {
        for peer in self.peers.lock().unwrap().values() {
            if let Some(outbox) = &peer.outbox {
                let _ = outbox.send(msg.clone());
            }
        }
    }

    /// A code only ever *grants* admission — it never gates it. If the
    /// host hasn't set one, this returns false (no bypass available), NOT
    /// true. Approval is the default path; a code is an opt-in shortcut
    /// around it, not the other way round.
    pub fn code_matches(&self, supplied: &Option<String>) -> bool {
        match (&*self.code.lock().unwrap(), supplied) {
            (Some(expected), Some(given)) => expected == given,
            _ => false,
        }
    }

    /// Records that we just heard from `peer_id` — called on every
    /// successful receive in the connection-handling loop, not just on
    /// Ping. Any traffic at all counts as "still alive."
    pub fn touch_peer(&self, peer_id: &str) {
        if let Some(peer) = self.peers.lock().unwrap().get(peer_id) {
            *peer.last_seen.lock().unwrap() = Instant::now();
        }
    }

    /// How long since we last heard anything from `peer_id`, in whole
    /// seconds — `None` if they're not currently a member at all.
    pub fn seconds_since_seen(&self, peer_id: &str) -> Option<u64> {
        self.peers.lock().unwrap().get(peer_id).map(|p| p.last_seen.lock().unwrap().elapsed().as_secs())
    }

    /// Marks a peer as explicitly kicked, BEFORE the Kick message is even
    /// sent — read later, once the connection actually ends, to report
    /// `DisconnectReason::Kicked` instead of `TimedOut`. Without this,
    /// a kick just looks like an ordinary timeout once the connection
    /// eventually dies, even though the host caused it on purpose.
    pub fn mark_kicked(&self, peer_id: &str) {
        if let Some(peer) = self.peers.lock().unwrap().get(peer_id) {
            peer.kicked.store(true, Ordering::Relaxed);
        }
    }

    pub fn was_kicked(&self, peer_id: &str) -> bool {
        self.peers.lock().unwrap().get(peer_id).map(|p| p.kicked.load(Ordering::Relaxed)).unwrap_or(false)
    }

    pub fn set_peer_printers(
        &self,
        peer_id: &str,
        printers: Vec<PrinterInfo>,
    ) {
        if let Some(peer) = self.peers.lock().unwrap().get(peer_id) {
            *peer.printers.lock().unwrap() = printers;
        }
    }

    pub fn peer_printers(
        &self,
        peer_id: &str,
    ) -> Option<Vec<PrinterInfo>> {
        self.peers
            .lock()
            .unwrap()
            .get(peer_id)
            .map(|peer| peer.printers.lock().unwrap().clone())
    }

    pub fn find_printer_owner(
        &self,
        printer_id: &str,
    ) -> Option<(PeerInfo, PrinterInfo)> {
        let peers = self.peers.lock().unwrap();

        for peer in peers.values() {
            let printers = peer.printers.lock().unwrap();

            if let Some(printer) = printers.iter().find(|p| p.id == printer_id) {
                return Some((peer.info.clone(), printer.clone()));
            }
        }

        None
    }

    pub fn broadcast_except(
        &self,
        except_peer_id: &str,
        msg: ControlMessage,
    ) {
        for (peer_id, peer) in self.peers.lock().unwrap().iter() {
            if peer_id == except_peer_id {
                continue;
            }

            if let Some(outbox) = &peer.outbox {
                let _ = outbox.send(msg.clone());
            }
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerLiveness {
    pub peer_id: String,
    pub seconds_since_seen: u64,
}

/// Lets the frontend show liveness ahead of the full disconnect —
/// e.g. graying out a device once it's gone quiet for a few seconds,
/// well before HEARTBEAT_TIMEOUT actually removes them from the roster.
#[tauri::command]
pub fn get_peer_liveness(session: tauri::State<'_, Arc<SessionState>>) -> Vec<PeerLiveness> {
    session.peers.lock().unwrap().iter()
        .map(|(id, p)| PeerLiveness { peer_id: id.clone(), seconds_since_seen: p.last_seen.lock().unwrap().elapsed().as_secs() })
        .collect()
}

/// Backs the "Require approval" toggle and optional join code in the
/// frontend's Network view. Both just mutate `SessionState` directly —
/// `control::handle_control_connection` reads them fresh on every join
/// attempt, so a change here takes effect on the very next connection.
#[tauri::command]
pub fn set_join_code(session: tauri::State<'_, Arc<SessionState>>, code: Option<String>) {
    *session.code.lock().unwrap() = code;
}

#[tauri::command]
pub fn set_auto_accept(session: tauri::State<'_, Arc<SessionState>>, value: bool) {
    *session.auto_accept.lock().unwrap() = value;
}