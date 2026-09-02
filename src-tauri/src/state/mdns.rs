use mdns_sd::ServiceDaemon;
use std::sync::Mutex;
use tokio::task::JoinHandle;

pub struct MdnsState {
    pub daemon: ServiceDaemon,
    pub browse_task: Mutex<Option<JoinHandle<()>>>,
    pub service_fullname: Mutex<Option<String>>,
}

impl MdnsState {
    pub fn new() -> Self {
        Self {
            daemon: ServiceDaemon::new().expect("Failed to create mDNS daemon"),
            browse_task: Mutex::new(None),
            service_fullname: Mutex::new(None),
        }
    }
}
