use crate::{db::Db, state::TcpState};
use std::sync::Arc;
use tauri::State;

/// Shared by both directions — a transfer_id is a transfer_id whether
/// it's an outgoing send or an incoming receive, so one command covers
/// the Cancel button for both.
#[tauri::command]
pub fn cancel_transfer(tcp_state: State<'_, Arc<TcpState>>, transfer_id: String) {
    if let Some(handle) = tcp_state.active_transfers.lock().unwrap().get(&transfer_id) {
        handle.cancel.cancel();
    }
}

#[tauri::command]
pub fn remove_incomplete_files(transfer_id:&str,db:State<'_,Arc<Db>>){
    db.inner().remove_incomplete_transfer(&transfer_id).unwrap();
}