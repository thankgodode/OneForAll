mod context;
mod events;
mod mdns;
mod network;
mod protocol;
mod state;
mod util;

use state::{AppSettings, DeviceInfo, MdnsState, PendingIncoming, SessionState, TcpState};
use std::sync::Arc;

fn default_download_dir() -> std::path::PathBuf {
    dirs::download_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
}

pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(DeviceInfo::new()))
        .manage(Arc::new(MdnsState::new()))
        .manage(Arc::new(SessionState::new("My Network")))
        .manage(Arc::new(TcpState::new()))
        .manage(Arc::new(AppSettings::new(default_download_dir())))
        .manage(Arc::new(PendingIncoming::new()))
        .invoke_handler(tauri::generate_handler![
            // device
            state::device::get_device_name,
            // settings
            state::settings::set_confirm_incoming_files,
            state::settings::set_download_dir,
            // mdns
            mdns::advertise::advertise_service,
            mdns::advertise::stop_advertising,
            mdns::browse::browse_service,
            mdns::browse::stop_browsing,
            // hosting (bind + advertise together)
            network::hosting::start_hosting,
            network::hosting::stop_hosting,
            // join / roster management
            network::join::join_network,
            network::join::respond_to_join,
            network::join::kick_peer,
            // transfers
            network::send::send_file_to_peers,
            network::receive::respond_to_incoming_file,
            network::transfers::cancel_transfer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
