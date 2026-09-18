import { invoke } from "@tauri-apps/api/core";

// Thin, one-to-one wrappers around every #[tauri::command] the backend
// exposes. Nothing here holds state or makes decisions — that's App.jsx's
// job. Keeping these as named functions (rather than calling invoke()
// directly from components) means the backend's argument names only need
// to be right in ONE place if they ever change.

// --- device / settings ---
export const getDeviceName = () => invoke("get_device_name");
// getSettings() restores whatever was saved last session — call this
// once at startup instead of trusting default/local React state, since
// display name, avatar color, download dir, and the confirm-files
// preference are now all persisted on the Rust side (see nearby-backend/src/db.rs).
export const getSettings = () => invoke("get_settings");
export const setDisplayName = (name) => invoke("set_display_name", { name });
export const setConfirmIncomingFiles = (value) => invoke("set_confirm_incoming_files", { value });
export const setDownloadDir = (path) => invoke("set_download_dir", { path });
export const setAvatarColor = (color) => invoke("set_avatar_color", { color });

// --- session access control (host side) ---
export const setJoinCode = (code) => invoke("set_join_code", { code });
export const setAutoAccept = (value) => invoke("set_auto_accept", { value });
// Seconds since each currently-connected peer was last heard from —
// lets the UI show liveness ahead of the full ~15s heartbeat timeout,
// e.g. graying out a device that's gone a few seconds quiet. Not polled
// automatically anywhere yet; call this from a UI that wants it.
export const getPeerLiveness = () => invoke("get_peer_liveness");

// --- mdns ---
export const browseService = () => invoke("browse_service");
export const stopBrowsing = () => invoke("stop_browsing");
export const advertiseService = (port, instanceName) => invoke("advertise_service", { port, instanceName });
export const stopAdvertising = () => invoke("stop_advertising");

// --- listening (every participant, host or not, does this once at startup) ---
export const startListening = () => invoke("start_listening");

// --- hosting (advertises the ALREADY-bound listener from startListening) ---
export const startHosting = (sessionName, instanceName) =>
  invoke("start_hosting", { sessionName, instanceName });
export const stopHosting = () => invoke("stop_hosting");

// --- join / roster ---
// myListenPort must come from startListening(), called once at app boot —
// it's what lets the host hand out a dialable address for THIS device.
export const joinNetwork = (hostAddr, displayName, myListenPort) =>
  invoke("join_network", { hostAddr, displayName, myListenPort });
export const leaveNetwork = () => invoke("leave_network");
export const respondToJoin = (requestId, accept) => invoke("respond_to_join", { requestId, accept });
export const kickPeer = (peerId) => invoke("kick_peer", { peerId });

// --- transfers ---
export const sendFileToPeers = (myPeerId, filePath, targetPeerIds) =>
  invoke("send_file_to_peers", { myPeerId, filePath, targetPeerIds });
export const respondToIncomingFile = (transferId, accept) =>
  invoke("respond_to_incoming_file", { transferId, accept });
export const cancelTransfer = (transferId) => invoke("cancel_transfer", { transferId });
export const removeIncomplete = (transferId) => invoke("remove_incomplete_files", { transferId});


// --- resumable transfers ---
// listIncompleteTransfers() covers BOTH still-running-app reconnects AND
// a full app restart — the ledger backing it is on disk (nearby-backend/src/db.rs),
// not in memory, so it survives either. Both resumeTransfer() and
// requestResume() re-resolve the peer by device name (not the old,
// possibly-stale peer_id).
//
// Two different directions, not interchangeable:
// - resumeTransfer(): call this when WE were the original SENDER —
//   we're the one reconnecting and asking the receiver where it left off.
// - requestResume(): call this when WE were the original RECEIVER —
//   we already know our own partial file's size, so we tell the sender
//   directly rather than asking. This is the direction to prefer in the
//   UI: the receiver, holding the partial file, is in the best position
//   to notice a transfer is incomplete and reach out, rather than
//   waiting for the original sender to remember.
export const listIncompleteTransfers = () => invoke("list_incomplete_transfers");
export const resumeTransfer = (myPeerId, transferId) =>
  invoke("resume_transfer", { myPeerId, transferId });
export const requestResume = (myPeerId, transferId) =>
  invoke("request_resume", { myPeerId, transferId });
