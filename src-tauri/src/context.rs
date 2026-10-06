//! `AppContext` bundles the Arc-wrapped state a connection-handling task
//! needs. Without it, every function on the accept-loop path (dispatch,
//! control, receive) accumulates its own parameter for each new piece of
//! shared state a feature needs — this keeps that list to one.

use crate::db::Db;
use crate::state::{
    AppSettings, DeviceInfo, PendingIncoming, PendingPrintJobs, SessionState, TcpState,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppContext {
    pub session: Arc<SessionState>,
    pub tcp: Arc<TcpState>,
    pub settings: Arc<AppSettings>,
    pub pending_incoming: Arc<PendingIncoming>,
    pub pending_print_jobs: Arc<PendingPrintJobs>,
    pub device: Arc<DeviceInfo>,
    pub db: Arc<Db>,
}
