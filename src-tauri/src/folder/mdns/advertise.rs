use super::SERVICE_TYPE;
use crate::state::MdnsState;
use mdns_sd::ServiceInfo;
use std::sync::Arc;
use std::time::Duration;
use tauri::State;

/// Core logic — deliberately takes a plain `&MdnsState` rather than a
/// Tauri `State<...>` so `network::hosting::start_hosting` can call it
/// directly right after binding a listener, without going through the
/// command layer below. No join code or session details go in the TXT
/// record: anyone browsing the network can read TXT properties, so
/// authorization has to happen over the control connection instead
/// (see `network::control`), not via what mDNS advertises.
pub async fn advertise(state: &MdnsState, port: u16, instance_name: &str) -> Result<(), String> {
    let host_name = format!("{}.local.", instance_name.to_lowercase().replace(' ', "-"));
    let properties: [(&str, &str); 0] = [];

    let service = ServiceInfo::new(SERVICE_TYPE, instance_name, &host_name, "", port, &properties[..])
        .map_err(|e| e.to_string())?
        .enable_addr_auto();

    let fullname = service.get_fullname().to_string();
    state.daemon.register(service).map_err(|e| e.to_string())?;
    *state.service_fullname.lock().unwrap() = Some(fullname);
    Ok(())
}

pub async fn stop(state: &MdnsState) -> Result<(), String> {
    let fullname = state.service_fullname.lock().unwrap().take();
    if let Some(name) = fullname {
        let rx = state.daemon.unregister(&name).map_err(|e| e.to_string())?;
        let _ = tokio::time::timeout(Duration::from_secs(2), rx.recv_async()).await;
    }
    Ok(())
}

/// Thin command wrappers for calling these directly from the frontend
/// (manual testing, or advertising outside the normal start_hosting flow).
#[tauri::command]
pub async fn advertise_service(state: State<'_, Arc<MdnsState>>, port: u16, instance_name: String) -> Result<(), String> {
    advertise(&state, port, &instance_name).await
}

#[tauri::command]
pub async fn stop_advertising(state: State<'_, Arc<MdnsState>>) -> Result<(), String> {
    stop(&state).await
}
