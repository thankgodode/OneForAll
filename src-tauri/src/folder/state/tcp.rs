use std::collections::HashMap;
use std::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// A running transfer (send or receive). `cancel` lets a "Cancel" button
/// in the UI stop the transfer *cooperatively* — the streaming loop checks
/// it every iteration and cleans up (removes the partial file, emits
/// transfer-cancelled) instead of just being killed mid-write by
/// `JoinHandle::abort()`, which would leave a truncated file behind with
/// no event ever telling the frontend what happened.
pub struct TransferHandle {
    pub task: JoinHandle<()>,
    pub cancel: CancellationToken,
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
}
