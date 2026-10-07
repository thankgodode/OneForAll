use crate::util::device_name;
use std::sync::Arc;

/// This machine's identity — computed once at startup since a hostname
/// lookup is a syscall, not something to redo per join or send.
pub struct DeviceInfo {
    pub name: String,
    pub printers: Vec<String>,
}

impl DeviceInfo {
    pub fn new() -> Self {
        Self { name: device_name(), printers: detect_printers() }
    }
}

fn detect_printers() -> Vec<String> {
    printers::get_printers()
        .into_iter()
        .map(|printer| printer.name)
        .collect()
}

/// Backs the "Device name" row in the frontend's Settings view.
#[tauri::command]
pub fn get_device_name(state: tauri::State<'_, Arc<DeviceInfo>>) -> String {
    state.name.clone()
}

#[tauri::command]
pub fn get_printers(state: tauri::State<'_, Arc<DeviceInfo>>) -> Vec<String> {
    state.printers.clone()
}
