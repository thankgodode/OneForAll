use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// User-configurable preferences — the Rust-side mirror of the toggles in
/// the frontend's Settings view. Read by the transfer code path to decide
/// whether to prompt before accepting an incoming file.
pub struct AppSettings {
    pub download_dir: Mutex<PathBuf>,
    pub confirm_incoming_files: Mutex<bool>,
}

impl AppSettings {
    pub fn new(download_dir: PathBuf) -> Self {
        Self {
            download_dir: Mutex::new(download_dir),
            confirm_incoming_files: Mutex::new(true),
        }
    }

    pub fn download_dir(&self) -> PathBuf {
        self.download_dir.lock().unwrap().clone()
    }

    pub fn require_confirmation(&self) -> bool {
        *self.confirm_incoming_files.lock().unwrap()
    }
}

#[tauri::command]
pub fn set_confirm_incoming_files(state: tauri::State<'_, Arc<AppSettings>>, value: bool) {
    *state.confirm_incoming_files.lock().unwrap() = value;
}

#[tauri::command]
pub fn set_download_dir(state: tauri::State<'_, Arc<AppSettings>>, path: String) {
    *state.download_dir.lock().unwrap() = PathBuf::from(path);
}
