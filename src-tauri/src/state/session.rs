use crate::protocol::{ControlMessage, PeerInfo};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc::UnboundedSender, oneshot};

/// A peer this device considers a member of the current session.
/// `outbox` is only `Some` when this device is the one that ACCEPTED
/// this peer's join — i.e. this device is acting as host for that
/// connection, and can push messages down it. When a joined CLIENT
/// mirrors its own copy of the roster (see `network::join`), it has no
/// direct connection to those other peers, so `outbox` is `None` — the
/// entry exists purely so `is_member`/`peer_info` work identically
/// whether this device is the host or a joined client.
pub struct PeerHandle {
    pub info: PeerInfo,
    pub outbox: Option<UnboundedSender<ControlMessage>>,
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

    pub fn insert_peer(&self, info: PeerInfo, outbox: Option<UnboundedSender<ControlMessage>>) {
        self.peers.lock().unwrap().insert(info.id.clone(), PeerHandle { info, outbox });
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
