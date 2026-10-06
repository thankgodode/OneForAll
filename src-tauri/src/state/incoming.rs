use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;

/// Incoming file transfers awaiting the user's accept/reject decision,
/// keyed by transfer_id. Same "hold a oneshot sender until the frontend
/// responds" pattern as `SessionState::pending` for joins.
pub struct PendingIncoming {
    pub requests: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

impl PendingIncoming {
    pub fn new() -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
        }
    }
}

pub struct PendingPrintJobs {
    pub requests: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

impl PendingPrintJobs {
    pub fn new() -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
        }
    }
}
