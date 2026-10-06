//! Payload types emitted to the frontend via `AppHandle::emit`. Centralized
//! here so `network::send` and `network::receive` share one definition
//! instead of two structurally-identical structs drifting apart over time.

use crate::protocol::PeerInfo;
use serde::Serialize;

// Same reasoning as PeerInfo in protocol.rs: these get serde_json'd
// straight into event payloads with no camelCase conversion unless told
// to, so every struct here needs the attribute or its fields arrive in
// JS as snake_case regardless of what the JS code assumes.

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub transfer_id: String,
    pub bytes_done: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IncomingFileRequest {
    pub transfer_id: String,
    pub filename: String,
    pub size: u64,
    pub from: PeerInfo,
    pub printer_name: Option<String>,
}

/// Fired exactly once per transfer, on both directions, right before
/// streaming begins — send.rs after resolving the file's size but before
/// connecting, receive.rs after any confirmation gate passes. This is
/// the ONLY way the frontend learns a transfer_id exists at all: the id
/// is generated deep inside Rust (`uuid::Uuid::new_v4()`), and
/// `send_file_to_peers` itself returns nothing per-recipient — without
/// this event there'd be no way to know which transfer_id belongs to
/// which (file, peer) pair well enough to render a progress row for it.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransferStarted {
    pub transfer_id: String,
    pub peer_id: String,
    pub peer_name: String,
    pub filename: String,
    pub size: u64,
    pub direction: &'static str, // "send" | "receive"
    pub printer_name: Option<String>,
}
