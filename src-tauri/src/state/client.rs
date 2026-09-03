use crate::protocol::ControlMessage;
use std::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;

/// Tracks THIS device's own membership when it has joined someone else's
/// session — the mirror of `SessionState`, which tracks OTHER devices'
/// membership when this device is the host. `outbox` is how
/// `leave_network` sends a clean `Leave` down the still-open control
/// connection without `network::join` needing to expose the raw socket.
pub struct ClientState {
    pub outbox: Mutex<Option<UnboundedSender<ControlMessage>>>,
    pub my_peer_id: Mutex<Option<String>>,
}

impl ClientState {
    pub fn new() -> Self {
        Self { outbox: Mutex::new(None), my_peer_id: Mutex::new(None) }
    }

    pub fn clear(&self) {
        *self.outbox.lock().unwrap() = None;
        *self.my_peer_id.lock().unwrap() = None;
    }
}
