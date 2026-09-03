// The backend's PeerInfo ({ id, display_name, device_name, addr }) predates
// the UI's device shape ({ id, name, device, status, color, ... }) — this
// file is the one place that reconciles the two, so no view needs to know
// PeerInfo's field names directly.

const PALETTE = ["#3FDCC4", "#5B9FE0", "#E8A63D", "#B383E8", "#E86B95", "#6FCF97", "#F2994A"];

// Deterministic so the same peer always gets the same color across
// sessions and across every device's screen — there's no color field on
// PeerInfo itself, so it's derived from the id instead of chosen randomly.
export function peerColor(id) {
  let hash = 0;
  for (let i = 0; i < id.length; i++) {
    hash = (hash * 31 + id.charCodeAt(i)) >>> 0;
  }
  return PALETTE[hash % PALETTE.length];
}

export function toUiDevice(peer, { status = "online", isSelf = false } = {}) {
  return {
    id: peer.id,
    name: peer.display_name,
    device: peer.device_name,
    status,
    isSelf,
    hasPrinter: false, // printing isn't implemented on the backend yet — see PrintView
    color: peerColor(peer.id),
  };
}
