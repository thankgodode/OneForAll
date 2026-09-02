//! Small standalone helpers with no dependency on app state.

use std::path::{Path, PathBuf};

/// The local machine's hostname. Call once at startup and cache it
/// (see `state::device::DeviceInfo`) — it's a syscall, not something to
/// redo on every join or send.
pub fn device_name() -> String {
    gethostname::gethostname().to_string_lossy().into_owned()
}

/// Strips any path components from a peer-supplied filename so it can't
/// be used to write outside the download directory — e.g. a peer sending
/// `filename: "../../.bashrc"` ends up writing to `.bashrc` in the
/// download dir, not overwriting your actual dotfile.
pub fn sanitize_filename(name: &str) -> String {
    Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .unwrap_or("unnamed_file")
        .to_string()
}

/// Returns a path in `dir` for `filename` that doesn't already exist,
/// appending " (1)", " (2)", etc. on collision — never silently
/// overwrites an existing file.
pub async fn unique_path(dir: &Path, filename: &str) -> std::io::Result<PathBuf> {
    let path = dir.join(filename);
    if !path.exists() {
        return Ok(path);
    }
    let stem = Path::new(filename).file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = Path::new(filename).extension().and_then(|e| e.to_str());

    for i in 1..1000 {
        let candidate = match ext {
            Some(ext) => dir.join(format!("{stem} ({i}).{ext}")),
            None => dir.join(format!("{stem} ({i})")),
        };
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "too many filename collisions"))
}
