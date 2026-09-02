use crate::protocol::{ControlMessage, PeerInfo};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

/// A joined peer's live connection handle. `outbox` lets other parts of
/// the app (kick_peer, roster broadcasts) push a message down this
/// specific peer's control connection — the connection's own writer task
/// (see `network::control`) owns the actual socket half.
pub struct PeerHandle {
    pub info: PeerInfo,
    pub outbox: UnboundedSender<ControlMessage>,
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

    pub fn insert_peer(&self, info: PeerInfo, outbox: UnboundedSender<ControlMessage>) {
        self.peers.lock().unwrap().insert(info.id.clone(), PeerHandle { info, outbox });
    }

    pub fn remove_peer(&self, peer_id: &str) {
        self.peers.lock().unwrap().remove(peer_id);
    }

    pub fn broadcast(&self, msg: ControlMessage) {
        for peer in self.peers.lock().unwrap().values() {
            let _ = peer.outbox.send(msg.clone());
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
