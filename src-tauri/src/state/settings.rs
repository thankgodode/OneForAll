use crate::db::Db;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// User-configurable preferences — the Rust-side mirror of the toggles in
/// the frontend's Settings view. Read by the transfer code path to decide
/// whether to prompt before accepting an incoming file.
///
/// This is an in-memory CACHE over the DB, not a replacement for it —
/// every field here is loaded from `Db` once at startup (`AppSettings::load`)
/// and every setter below writes through to `Db` immediately. Reads stay
/// fast (no DB round-trip on the hot path — `require_confirmation()` is
/// checked on every incoming file), while writes stay durable.
pub struct AppSettings {
    pub display_name: Mutex<String>,
    pub download_dir: Mutex<PathBuf>,
    pub confirm_incoming_files: Mutex<bool>,
    pub avatar_color: Mutex<String>,
}

impl AppSettings {
    /// Loads persisted values from `db`, falling back to the given
    /// defaults only if the DB read itself fails (e.g. a corrupted file)
    /// — NOT on every fresh install, since `Db::open` already seeds the
    /// settings row with sane defaults the first time it creates the
    /// database.
    pub fn load(db: &Db, fallback_display_name: &str, fallback_download_dir: PathBuf) -> Self {
        match db.load_settings() {
            Ok(s) => Self {
                display_name: Mutex::new(s.display_name),
                download_dir: Mutex::new(PathBuf::from(s.download_dir)),
                confirm_incoming_files: Mutex::new(s.confirm_incoming_files),
                avatar_color: Mutex::new(s.avatar_color),
            },
            Err(_) => Self {
                display_name: Mutex::new(fallback_display_name.to_string()),
                download_dir: Mutex::new(fallback_download_dir),
                confirm_incoming_files: Mutex::new(true),
                avatar_color: Mutex::new("#3FDCC4".into()),
            },
        }
    }

    pub fn download_dir(&self) -> PathBuf {
        self.download_dir.lock().unwrap().clone()
    }

    pub fn require_confirmation(&self) -> bool {
        *self.confirm_incoming_files.lock().unwrap()
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub display_name: String,
    pub download_dir: String,
    pub confirm_incoming_files: bool,
    pub avatar_color: String,
}

/// Called once at frontend startup to restore whatever was saved last
/// session — without this, display name/avatar/download dir/confirm
/// preference would all silently reset to defaults on every launch,
/// which was true of the whole app before this feature existed.
#[tauri::command]
pub fn get_settings(settings: tauri::State<'_, Arc<AppSettings>>) -> SettingsSnapshot {
    SettingsSnapshot {
        display_name: settings.display_name.lock().unwrap().clone(),
        download_dir: settings.download_dir().to_string_lossy().into_owned(),
        confirm_incoming_files: settings.require_confirmation(),
        avatar_color: settings.avatar_color.lock().unwrap().clone(),
    }
}

#[tauri::command]
pub fn set_display_name(settings: tauri::State<'_, Arc<AppSettings>>, db: tauri::State<'_, Arc<Db>>, name: String) -> Result<(), String> {
    *settings.display_name.lock().unwrap() = name.clone();
    db.save_display_name(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_confirm_incoming_files(settings: tauri::State<'_, Arc<AppSettings>>, db: tauri::State<'_, Arc<Db>>, value: bool) -> Result<(), String> {
    *settings.confirm_incoming_files.lock().unwrap() = value;
    db.save_confirm_incoming_files(value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_download_dir(settings: tauri::State<'_, Arc<AppSettings>>, db: tauri::State<'_, Arc<Db>>, path: String) -> Result<(), String> {
    *settings.download_dir.lock().unwrap() = PathBuf::from(&path);
    db.save_download_dir(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_avatar_color(settings: tauri::State<'_, Arc<AppSettings>>, db: tauri::State<'_, Arc<Db>>, color: String) -> Result<(), String> {
    *settings.avatar_color.lock().unwrap() = color.clone();
    db.save_avatar_color(&color).map_err(|e| e.to_string())
}
