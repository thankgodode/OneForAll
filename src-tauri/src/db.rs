//! Persistence layer — one SQLite file for everything durable: profile
//! settings and the incomplete-transfers ledger that makes resume
//! possible. Deliberately synchronous (plain `rusqlite`, not an async
//! driver): every call here is small, local, and rare — settings change
//! on user action, and transfer records are written exactly twice per
//! transfer (once at start, once at completion/removal), never on a
//! progress tick. That low a write volume doesn't justify the extra
//! moving parts of an async database driver.
//!
//! Caveat worth knowing: calling `Connection` methods directly inside an
//! async fn technically blocks the tokio worker thread for the (very
//! short) duration of the query. For this app's write volume that's a
//! non-issue in practice; if this ever needs hardening, wrap call sites
//! in `tokio::task::spawn_blocking` rather than changing this module.

use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

pub struct Db {
    conn: Mutex<Connection>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSettings {
    pub display_name: String,
    pub download_dir: String,
    pub confirm_incoming_files: bool,
    pub confirm_incoming_printing: bool,
    pub avatar_color: String,
}

/// One row per transfer that's currently interrupted (connection lost,
/// not explicitly cancelled) — enough to offer "resume" later, without
/// ever needing to trust a separately-tracked progress counter. On
/// resume, the receiver's actual on-disk file size IS the offset; this
/// record just needs to say WHICH transfer, WHICH peer, and WHERE.
///
/// Deliberately keyed on `peer_device_name`, not `peer_id`, for matching
/// purposes — `peer_id` is regenerated fresh every time a device joins a
/// session (see control.rs), so after exactly the kind of reconnect this
/// feature exists for, the OLD peer_id in this record won't match the
/// NEW one the same physical device gets assigned. `device_name` is a
/// far more stable (if not perfectly unique) signal across rejoins.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncompleteTransfer {
    pub transfer_id: String,
    pub peer_display_name: String,
    pub peer_device_name: String,
    pub filename: String,
    pub file_path: String,
    pub total_size: u64,
    pub direction: String,        // "send" | "receive"
    pub created_at: i64,          // unix seconds — when the transfer STARTED
    pub interrupted_at: Option<i64>, // unix seconds — when it last stopped mid-stream; None while actively streaming
}

impl Db {
    pub fn open(path: &Path, default_display_name: &str, default_download_dir: &str) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                display_name TEXT NOT NULL,
                download_dir TEXT NOT NULL,
                confirm_incoming_files INTEGER NOT NULL DEFAULT 1,
                confirm_incoming_printing INTEGER NOT NULL DEFAULT 1,
                avatar_color TEXT NOT NULL DEFAULT '#3FDCC4'
            );
            CREATE TABLE IF NOT EXISTS incomplete_transfers (
                transfer_id TEXT PRIMARY KEY,
                peer_display_name TEXT NOT NULL,
                peer_device_name TEXT NOT NULL,
                filename TEXT NOT NULL,
                file_path TEXT NOT NULL,
                total_size INTEGER NOT NULL,
                direction TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                interrupted_at INTEGER
            );"
        )?;
        // CREATE TABLE IF NOT EXISTS above only defines the shape for a
        // BRAND NEW database file — it does nothing to a table that
        // already existed from a previous version without this column.
        // This covers that case; it fails harmlessly with "duplicate
        // column" on every run where the column is already present
        // (which includes every fresh install, since CREATE TABLE just
        // added it above) — the error is expected and ignored on purpose.
        let _ = conn.execute("ALTER TABLE incomplete_transfers ADD COLUMN interrupted_at INTEGER", []);
        // CHECK (id = 1) above enforces a singleton row; this only fires
        // the very first time the app runs against a fresh database file.
        conn.execute(
            "INSERT OR IGNORE INTO settings (id, display_name, download_dir) VALUES (1, ?1, ?2)",
            params![default_display_name, default_download_dir],
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn load_settings(&self) -> rusqlite::Result<PersistedSettings> {
        self.conn.lock().unwrap().query_row(
            "SELECT display_name, download_dir, confirm_incoming_files, avatar_color FROM settings WHERE id = 1",
            [],
            |row| Ok(PersistedSettings {
                display_name: row.get(0)?,
                download_dir: row.get(1)?,
                confirm_incoming_files: row.get::<_, i64>(2)? != 0,
                confirm_incoming_printing: row.get::<_, i64>(2)? !=0,
                avatar_color: row.get(3)?,
            }),
        )
    }

    pub fn save_display_name(&self, name: &str) -> rusqlite::Result<()> {
        self.conn.lock().unwrap().execute("UPDATE settings SET display_name = ?1 WHERE id = 1", params![name]).map(|_| ())
    }
    pub fn save_download_dir(&self, dir: &str) -> rusqlite::Result<()> {
        self.conn.lock().unwrap().execute("UPDATE settings SET download_dir = ?1 WHERE id = 1", params![dir]).map(|_| ())
    }
    pub fn save_confirm_incoming_files(&self, value: bool) -> rusqlite::Result<()> {
        self.conn.lock().unwrap().execute("UPDATE settings SET confirm_incoming_files = ?1 WHERE id = 1", params![value as i64]).map(|_| ())
    }
    pub fn save_avatar_color(&self, color: &str) -> rusqlite::Result<()> {
        self.conn.lock().unwrap().execute("UPDATE settings SET avatar_color = ?1 WHERE id = 1", params![color]).map(|_| ())
    }

    pub fn save_incomplete_transfer(&self, t: &IncompleteTransfer) -> rusqlite::Result<()> {
        self.conn.lock().unwrap().execute(
            "INSERT OR REPLACE INTO incomplete_transfers
             (transfer_id, peer_display_name, peer_device_name, filename, file_path, total_size, direction, created_at, interrupted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![t.transfer_id, t.peer_display_name, t.peer_device_name, t.filename, t.file_path, t.total_size as i64, t.direction, t.created_at, t.interrupted_at],
        ).map(|_| ())
    }

    /// Stamps WHEN a transfer stopped mid-stream — called from the
    /// "keep resumable" branches in receive.rs/send.rs, every time
    /// (peer left, timed out, or was explicitly paused), not just once.
    /// A transfer that's dropped and reconnected several times keeps
    /// getting a FRESH timestamp here, so "interrupted X ago" always
    /// reflects the MOST RECENT gap, not the first one.
    pub fn mark_transfer_interrupted(&self, transfer_id: &str, at: i64) -> rusqlite::Result<()> {
        self.conn.lock().unwrap().execute(
            "UPDATE incomplete_transfers SET interrupted_at = ?1 WHERE transfer_id = ?2",
            params![at, transfer_id],
        ).map(|_| ())
    }

    pub fn remove_incomplete_transfer(&self, transfer_id: &str) -> rusqlite::Result<()> {
        self.conn.lock().unwrap().execute("DELETE FROM incomplete_transfers WHERE transfer_id = ?1", params![transfer_id]).map(|_| ())
    }

    pub fn get_incomplete_transfer(&self, transfer_id: &str) -> rusqlite::Result<Option<IncompleteTransfer>> {
        self.conn.lock().unwrap().query_row(
            "SELECT transfer_id, peer_display_name, peer_device_name, filename, file_path, total_size, direction, created_at, interrupted_at
             FROM incomplete_transfers WHERE transfer_id = ?1",
            params![transfer_id],
            Self::row_to_transfer,
        ).map(Some).or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            e => Err(e),
        })
    }

    pub fn list_incomplete_transfers(&self) -> rusqlite::Result<Vec<IncompleteTransfer>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT transfer_id, peer_display_name, peer_device_name, filename, file_path, total_size, direction, created_at, interrupted_at
             FROM incomplete_transfers ORDER BY created_at DESC"
        )?;
        let x = stmt.query_map([], Self::row_to_transfer)?.collect();

        x
    }

    fn row_to_transfer(row: &rusqlite::Row) -> rusqlite::Result<IncompleteTransfer> {
        Ok(IncompleteTransfer {
            transfer_id: row.get(0)?,
            peer_display_name: row.get(1)?,
            peer_device_name: row.get(2)?,
            filename: row.get(3)?,
            file_path: row.get(4)?,
            total_size: row.get::<_, i64>(5)? as u64,
            direction: row.get(6)?,
            created_at: row.get(7)?,
            interrupted_at: row.get(8)?,
        })
    }
}
