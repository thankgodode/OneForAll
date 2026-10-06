use crate::util::device_name;
use std::{process::Command, sync::Arc};

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
    #[cfg(target_os = "windows")]
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", "Get-Printer | Select-Object -ExpandProperty Name"])
        .output();
    #[cfg(not(target_os = "windows"))]
    let output = Command::new("lpstat").args(["-p"]).output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    let printers = String::from_utf8_lossy(&output.stdout);
    #[cfg(target_os = "windows")]
    let names = printers.lines().map(str::trim).filter(|name| !name.is_empty()).map(str::to_owned);
    #[cfg(not(target_os = "windows"))]
    let names = printers.lines().filter_map(|line| {
        let mut fields = line.split_whitespace();
        (fields.next() == Some("printer")).then(|| fields.next().map(str::to_owned)).flatten()
    });
    names.collect()
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
