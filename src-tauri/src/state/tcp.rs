use std::collections::HashMap;
use std::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// A running transfer (send or receive). Two SEPARATE cancellation
/// signals, deliberately not one, because they mean different things to
/// the streaming loop that checks them:
///
/// - `cancel` — an explicit user "Cancel" click. True abandonment: the
///   streaming loop deletes the partial file and the resume-ledger record.
/// - `peer_gone` — the OTHER party left the session (they disconnected,
///   were kicked, or WE did/were). This is exactly the "interrupted, not
///   abandoned" case the resume feature exists for — the loop stops
///   streaming but keeps both the partial file and the DB record, same
///   as an ordinary connection-lost failure.
///
/// Collapsing these into one signal would mean a peer leaving the
/// network silently deletes any in-flight transfer with them, which
/// defeats the resume feature for exactly the scenario it's meant to
/// handle.
#[derive(Debug)]
pub struct TransferHandle {
    pub peer_id: String,
    pub task: JoinHandle<()>,
    pub cancel: CancellationToken,
    pub peer_gone: CancellationToken,
}

pub struct TcpState {
    pub listener_task: Mutex<Option<JoinHandle<()>>>,
    pub bound_port: Mutex<Option<u16>>,
    pub active_transfers: Mutex<HashMap<String, TransferHandle>>,
}

impl TcpState {
    pub fn new() -> Self {
        Self {
            listener_task: Mutex::new(None),
            bound_port: Mutex::new(None),
            active_transfers: Mutex::new(HashMap::new()),
        }
    }

    /// Stops any active transfer with `peer_id`, because THEY left the
    /// session — not an explicit cancel, so the transfer stays resumable.
    /// Called for a single departing peer (see control.rs / join.rs).
    pub fn stop_transfers_with_peer(&self, peer_id: &str) {
        for handle in self.active_transfers.lock().unwrap().values() {
            if handle.peer_id == peer_id {
                handle.peer_gone.cancel();
            }
        }
    }

    /// Stops every active transfer regardless of peer — used when THIS
    /// device is the one leaving (kicked, or its own connection to the
    /// host was lost), since every transfer it's part of is moot either way.
    pub fn stop_all_transfers(&self) {
        for handle in self.active_transfers.lock().unwrap().values() {
            handle.peer_gone.cancel();
        }
    }
}
