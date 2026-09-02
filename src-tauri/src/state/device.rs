use crate::util::device_name;
use std::sync::Arc;

/// This machine's identity — computed once at startup since a hostname
/// lookup is a syscall, not something to redo per join or send.
pub struct DeviceInfo {
    pub name: String,
}

impl DeviceInfo {
    pub fn new() -> Self {
        Self { name: device_name() }
    }
}

/// Backs the "Device name" row in the frontend's Settings view.
#[tauri::command]
pub fn get_device_name(state: tauri::State<'_, Arc<DeviceInfo>>) -> String {
    state.name.clone()
}
