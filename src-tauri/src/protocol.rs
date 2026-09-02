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

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PeerInfo {
    pub id: String,
    pub display_name: String,
    pub device_name: String,
    pub addr: String, // "ip:port" — used to dial this peer directly for transfers
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ConnectionKind {
    Control,
    Transfer {
        peer_id: String,
        transfer_id: String,
        filename: String,
        size: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ControlMessage {
    JoinRequest { display_name: String, device_name: String, code: Option<String> },
    JoinPending,
    JoinAccepted { session_name: String, peer_id: String, roster: Vec<PeerInfo> },
    JoinRejected { reason: String },
    PeerJoined(PeerInfo),
    PeerLeft { peer_id: String },
    Kick { reason: String },
    Leave,
    Ping,
    Pong,
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
