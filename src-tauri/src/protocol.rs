//! Wire protocol: message types shared between host and peers, plus the
//! length-prefixed framing used to send/receive them over a TcpStream.
//!
//! Connection layout:
//! 1. Every new TCP connection starts with exactly one `ConnectionKind` frame.
//! 2. `ConnectionKind::Control` connections then exchange `ControlMessage`
//!    frames for as long as the peer stays joined to the session.
//! 3. `ConnectionKind::Transfer` connections send raw file bytes immediately
//!    after their `ConnectionKind` frame — no further framing needed since
//!    the size is already known from it.

use std::{net::TcpStream, sync::Arc};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

use crate::{db::Db, state::TcpState};
use crate::printing::printer::PrinterInfo;

// `rename_all = "camelCase"` on every struct that crosses the JS boundary
// (this one, and everything in events.rs) is deliberate and load-bearing:
// Tauri auto-converts camelCase<->snake_case for #[tauri::command]
// ARGUMENTS, but NOT for emitted event payloads or command return values
// — those are just serde_json'd as-is, using whatever the Rust struct's
// field names literally are. Without this attribute, PeerInfo would
// serialize as `display_name`/`device_name`, and every frontend spot
// that expects `.displayName`/`.deviceName` would silently get
// `undefined` instead of an error. One convention, enforced here, is
// what makes the frontend able to assume camelCase everywhere without
// checking case-by-case.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PeerInfo {
    pub id: String,
    pub display_name: String,
    pub device_name: String,
    pub addr: String, // "ip:port" — used to dial this peer directly for transfers
    pub printers: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ConnectionKind {
    Control,
    Transfer {
        peer_id: String,
        transfer_id: String,
        filename: String,
        size: u64,
        printer_name: Option<String>,
        copies: Option<u32>,
    },
    /// Reconnecting to continue a transfer that was interrupted, not
    /// started fresh — no filename/size needed, since the receiver
    /// already has both from the original attempt (see db::IncompleteTransfer).
    /// The receiver responds with exactly one `ResumeOffset` (via the
    /// same length-prefixed bincode framing as Control messages) before
    /// either side sends a single raw file byte — that response is what
    /// tells the sender where in its local file to seek to. This is the
    /// SENDER-initiated resume path — see `RequestResume` below for the
    /// receiver-initiated equivalent.
    ResumeTransfer {
        peer_id: String,
        transfer_id: String,
    },
    /// The RECEIVER-initiated resume path — sent by whoever is missing
    /// data, to whoever originally sent it. Unlike ResumeTransfer, the
    /// offset travels WITH this message rather than in a reply that
    /// follows it: the receiver already knows its own partial file's
    /// real size before it ever connects, so there's nothing to ask for
    /// — streaming can begin immediately once the sender validates the
    /// request.
    RequestResume {
        peer_id: String,
        transfer_id: String,
        bytes_already_have: u64,
    },
    // PrinterInfo{
    //     id: String,
    //     name: String,
    //     is_default: bool,
    // }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintRequest {
    pub job_id: String,
    pub printer_id: String,
    pub filename: String,
    pub copies: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PrintResponse {
    Accepted {
        job_id: String,
    },
    Rejected {
        job_id: String,
        reason: String,
    },
    Completed {
        job_id: String,
    },
    Failed {
        job_id: String,
        reason: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PrintJobResult {
    pub success: bool,
    pub error: Option<String>,
}

/// The receiver's answer to `ConnectionKind::ResumeTransfer` — how many
/// bytes it already has on disk. Deliberately derived from the partial
/// file's ACTUAL size on disk (`fs::metadata`) at the moment this is
/// sent, never from a separately-tracked progress counter that could
/// have drifted from what was really flushed before the disconnect.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResumeOffset {
    pub bytes_received: u64,
}

/// A stand-in peer_id representing "the host of the session I joined."
/// The host never goes through `JoinRequest` itself, so it has no id in
/// the same namespace as joined peers by default — using this constant
/// on both sides means `SessionState::is_member` can treat "sent by the
/// host" and "sent by a joined peer" identically, with no special case
/// in `dispatch::handle_connection`.
pub const HOST_PEER_ID: &str = "host";

/// Why a peer's Control connection ended — this is what lets both sides
/// give an honest, specific account instead of every disconnect looking
/// like the same generic "gone" event. `Left` and `Kicked` are both
/// orderly (someone chose this); `TimedOut` is the one that matters most
/// for resumable transfers, since it's the only one where "unreachable"
/// might just be temporary.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DisconnectReason {
    Left,     // sent an explicit Leave
    Kicked,   // host sent Kick before this connection ended
    TimedOut, // no traffic within HEARTBEAT_TIMEOUT, or the socket just died with no FIN/RST at all
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ControlMessage {
    JoinRequest { display_name: String, device_name: String, code: Option<String>, listen_port: u16, printers: Vec<PrinterInfo> },
    JoinPending,
    JoinAccepted { session_name: String, peer_id: String, host_device_name: String, roster: Vec<PeerInfo>, printers:Vec<PrinterInfo>},
    JoinRejected { reason: String },
    PeerJoined(PeerInfo),
    PeerLeft { peer_id: String, reason: DisconnectReason },
    Kick { reason: String },
    Leave,
    Ping,
    Pong,
    PrinterList {
        peer_id: String,
        printers: Vec<PrinterInfo>,
    },
    PrintRequest(PrintRequest),
    PrintResponse(PrintResponse),
}

const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

/// Writes one length-prefixed, bincode-encoded message. Generic over any
/// async writer so it works with a full `TcpStream` or an owned half
/// (`OwnedWriteHalf`) after `.into_split()`.
pub async fn send_msg<T, W>(stream: &mut W, msg: &T) -> std::io::Result<()>
where
    T: Serialize,
    W: AsyncWriteExt + Unpin,
{
    let bytes = bincode::serialize(msg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    stream.write_u32(bytes.len() as u32).await?;
    stream.write_all(&bytes).await
}

/// Reads one length-prefixed, bincode-encoded message. Rejects an
/// oversized length prefix up front so a malicious or buggy peer can't
/// force a huge allocation just by lying about a frame's size.
pub async fn recv_msg<T, R>(stream: &mut R) -> std::io::Result<T>
where
    T: for<'de> Deserialize<'de>,
    R: AsyncReadExt + Unpin,
{
    let len = stream.read_u32().await? as usize;
    if len > MAX_FRAME_BYTES {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "frame too large"));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    bincode::deserialize(&buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
