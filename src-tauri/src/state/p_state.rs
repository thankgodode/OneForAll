use crate::state::SessionState;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePrinter {
    pub peer_id: String,
    pub peer_display_name: String,
    pub peer_device_name: String,
    pub printer_id: String,
    pub printer_name: String,
    pub is_default: bool,
}

#[tauri::command]
pub fn get_remote_printers(
    session: tauri::State<'_, Arc<SessionState>>,
) -> Vec<RemotePrinter> {
    let peers = session.peers.lock().unwrap();

    let mut result = Vec::new();

    for peer in peers.values() {
        let printers = peer.printers.lock().unwrap();

        for printer in printers.iter() {
            result.push(RemotePrinter {
                peer_id: peer.info.id.clone(),
                peer_display_name: peer.info.display_name.clone(),
                peer_device_name: peer.info.device_name.clone(),
                printer_id: printer.id.clone(),
                printer_name: printer.name.clone(),
                is_default: printer.is_default,
            });
        }
    }

    result
}