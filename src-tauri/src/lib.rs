mod context;
mod events;
mod mdns;
mod network;
mod protocol;
mod state;
mod util;
mod db;

use state::{AppSettings, DeviceInfo, MdnsState, PendingIncoming, SessionState, TcpState};
use tauri::Manager;
use std::sync::Arc;
use crate::db::Db;

use crate::state::ClientState;

fn default_download_dir() -> std::path::PathBuf {
    dirs::download_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let device_info = Arc::new(DeviceInfo::new());

    tauri::Builder::default()
        .manage(Arc::new(DeviceInfo::new()))
        .manage(Arc::new(ClientState::new()))
        .manage(Arc::new(MdnsState::new()))
        .manage(Arc::new(SessionState::new("My Network")))
        .manage(Arc::new(TcpState::new()))
        // .manage(Arc::new(AppSettings::new(default_download_dir())))
        .manage(Arc::new(PendingIncoming::new()))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(move |app| {
            let data_dir = app.path().app_data_dir().expect("no app data directory available");
            std::fs::create_dir_all(&data_dir).expect("failed to create app data directory");
            let db_path = data_dir.join("oneforall.db");
            println!("Data directory: {:?}", data_dir);
            println!("Database path: {:?}", db_path);

            let default_dir = default_download_dir();
            let db = Db::open(&db_path, &device_info.name, &default_dir.to_string_lossy())
                .expect("failed to open local database");
            let settings = AppSettings::load(&db, &device_info.name, default_dir);

            app.manage(Arc::new(db));
            app.manage(Arc::new(settings));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // device
            state::device::get_device_name,
            // settings
            state::settings::get_settings,
            state::settings::set_display_name,
            state::settings::set_confirm_incoming_files,
            state::settings::set_download_dir,
            state::settings::set_avatar_color,
            // session access control
            state::session::set_join_code,
            state::session::set_auto_accept,
            // mdns
            mdns::advertise::advertise_service,
            mdns::advertise::stop_advertising,
            mdns::browse::browse_service,
            mdns::browse::stop_browsing,
            // hosting (advertising an already-bound listener)
            network::listener::start_listening,
            network::hosting::start_hosting,
            network::hosting::stop_hosting,
            // join / roster management
            network::join::join_network,
            network::join::leave_network,
            network::join::respond_to_join,
            network::join::kick_peer,
            // transfers
            network::send::send_file_to_peers,
            network::send::resume_transfer,
            network::receive::respond_to_incoming_file,
            network::receive::list_incomplete_transfers,
            network::transfers::cancel_transfer,
        ])
        .run(tauri::generate_context!())        
        .expect("error while running tauri application");
}

