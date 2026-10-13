use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;

pub struct PendingIncomingRequest {
    pub responder: oneshot::Sender<bool>,
    pub is_print_job: bool,
}

/// Incoming file transfers awaiting the user's accept/reject decision,
/// keyed by transfer_id. Same "hold a oneshot sender until the frontend
/// responds" pattern as `SessionState::pending` for joins.
pub struct PendingIncoming {
    pub requests: Mutex<HashMap<String, PendingIncomingRequest>>,
}

impl PendingIncoming {
    pub fn new() -> Self {
        Self { requests: Mutex::new(HashMap::new()) }
    }

    pub fn accept_pending_print_jobs(&self) -> Vec<String> {
        let mut requests = self.requests.lock().unwrap();
        let transfer_ids = requests
            .iter()
            .filter(|(_, request)| request.is_print_job)
            .map(|(transfer_id, _)| transfer_id.clone())
            .collect::<Vec<_>>();

        for transfer_id in &transfer_ids {
            if let Some(request) = requests.remove(transfer_id) {
                let _ = request.responder.send(true);
            }
        }

        transfer_ids
    }
}

#[cfg(test)]
mod tests {
    use super::{PendingIncoming, PendingIncomingRequest};
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn accepting_pending_print_jobs_leaves_file_requests_pending() {
        let pending = PendingIncoming::new();
        let (print_tx, print_rx) = oneshot::channel();
        let (file_tx, _file_rx) = oneshot::channel();
        {
            let mut requests = pending.requests.lock().unwrap();
            requests.insert("print".into(), PendingIncomingRequest {
                responder: print_tx,
                is_print_job: true,
            });
            requests.insert("file".into(), PendingIncomingRequest {
                responder: file_tx,
                is_print_job: false,
            });
        }

        assert_eq!(pending.accept_pending_print_jobs(), vec!["print"]);
        assert_eq!(print_rx.await, Ok(true));
        assert!(pending.requests.lock().unwrap().contains_key("file"));
    }
}
