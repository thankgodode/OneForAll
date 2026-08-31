use commands::{start_service,stop_advertising, browse_service, stop_browsing, start_hosting, stop_hosting, MdnsState,TcpState};

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(MdnsState::new())
        .manage(TcpState::new())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_service,stop_advertising, browse_service, stop_browsing, start_hosting, stop_hosting])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
