// Mirrors backend/src/protocol.rs::HOST_PEER_ID exactly — if that constant
// ever changes on the Rust side, this is the one line that needs to match.
// Used as `myPeerId` when sending a file while hosting (the host has no
// "real" peer_id of its own, since it never goes through JoinRequest).
export const HOST_PEER_ID = "host";
