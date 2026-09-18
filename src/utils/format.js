// Small formatting helpers used across the app.

export function fmtBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

export function initials(name) {
  return (name || "?").slice(0, 2).toUpperCase();
}

export function fmtSpeed(bytesPerSec) {
  return `${fmtBytes(bytesPerSec)}/s`;
}

// unixSeconds is the backend's interrupted_at/created_at — a plain unix
// timestamp in seconds, not milliseconds, so it needs *1000 for Date math.
export function fmtTimeAgo(unixSeconds) {
  const seconds = Math.max(0, Math.floor(Date.now() / 1000 - unixSeconds));
  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

// The backend reports raw bytes-done/total/speed, not a ready-made ETA —
// this is where "how long is left" actually gets computed, client-side.
export function fmtEta(bytesDone, totalBytes, bytesPerSec) {
  if (bytesPerSec <= 0) return "—";
  const remaining = totalBytes - bytesDone;
  if (remaining <= 0) return "0s";
  const seconds = Math.ceil(remaining / bytesPerSec);
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.ceil(seconds / 60)}m`;
  return `${Math.ceil(seconds / 3600)}h`;
}
