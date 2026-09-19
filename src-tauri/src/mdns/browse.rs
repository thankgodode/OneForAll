use super::SERVICE_TYPE;
use crate::state::MdnsState;
use mdns_sd::ServiceEvent;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Starts browsing and forwards resolved/removed peers to the frontend as
/// events. Emits the resolved address alongside the fullname so the UI
/// can wire a click straight into `network::join::join_network` without
/// a second discovery round trip.
#[tauri::command]
pub async fn browse_service(app: AppHandle, state: State<'_, Arc<MdnsState>>) -> Result<(), String> {
    let receiver = state.daemon.browse(SERVICE_TYPE).map_err(|e| e.to_string())?;

    let handle = tokio::spawn(async move {
        while let Ok(event) = receiver.recv_async().await {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    let addr = info
                        .get_addresses_v4()
                        .iter()
                        .next()
                        .map(|ip| format!("{}:{}", ip, info.get_port()));
                    let _ = app.emit("mdns-service-resolved", (info.get_fullname().to_string(), addr));
                }
                ServiceEvent::ServiceRemoved(_, fullname) => {
                    let _ = app.emit("mdns-service-removed", fullname);
                }
                _ => {}
            }
        }
    });

    *state.browse_task.lock().unwrap() = Some(handle);
    Ok(())
}

#[tauri::command]
pub fn stop_browsing(state: State<'_, Arc<MdnsState>>) -> Result<(), String> {
    if let Some(handle) = state.browse_task.lock().unwrap().take() {
        handle.abort(); // no cleanup needed here — it's just a read loop over discovery events
    }
    Ok(())
}
