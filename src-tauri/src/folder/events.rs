//! Payload types emitted to the frontend via `AppHandle::emit`. Centralized
//! here so `network::send` and `network::receive` share one definition
//! instead of two structurally-identical structs drifting apart over time.

use crate::protocol::PeerInfo;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct TransferProgress {
    pub transfer_id: String,
    pub bytes_done: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: f64,
}

#[derive(Serialize, Clone)]
pub struct IncomingFileRequest {
    pub transfer_id: String,
    pub filename: String,
    pub size: u64,
    pub from: PeerInfo,
}
