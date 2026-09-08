pub mod control;
pub mod dispatch;
pub mod hosting;
pub mod join;
pub mod listener;
pub mod receive;
pub mod send;
pub mod transfers;

/// Read/write buffer size for both directions of a file transfer (see
/// `send::stream_from_disk` and `receive::stream_to_disk`). One shared
/// constant so tuning it only ever means changing one line — it used to
/// be duplicated in both files.
///
/// This is a ceiling on how much moves per loop iteration, not a
/// protocol limit — raw file bytes aren't length-prefixed the way
/// ControlMessage/ConnectionKind frames are (see protocol::MAX_FRAME_BYTES,
/// which caps THOSE, not this). There's no hard max here; it's bounded in
/// practice by how much memory you're willing to give to ONE buffer PER
/// concurrent transfer (send_file_to_peers spawns one stream task per
/// recipient, each with its own buffer this size).
pub const TRANSFER_CHUNK_SIZE: usize = 256 * 1024; // 256 KB
