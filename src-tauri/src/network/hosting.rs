use crate::mdns;
use crate::protocol::ControlMessage;
use crate::state::{MdnsState, SessionState, TcpState};
use std::sync::Arc;
use tauri::State;

/// Advertises the ALREADY-bound listener (see `listener::start_listening`,
/// which every participant calls at startup regardless of role) as a
/// discoverable, named session. Deliberately can't bind its own port —
/// requiring one to already exist in `TcpState` makes "advertise before
/// anything is listening" structurally impossible rather than just a
/// convention to remember.
#[tauri::command]
pub async fn start_hosting(
    mdns_state: State<'_, Arc<MdnsState>>,
    tcp_state: State<'_, Arc<TcpState>>,
    session_state: State<'_, Arc<SessionState>>,
    session_name: String,
    instance_name: String,
) -> Result<u16, String> {
    let bound_port = *tcp_state.bound_port.lock().unwrap();
    let port = bound_port.ok_or_else(|| "Not listening yet — call start_listening first".to_string())?;

    *session_state.session_name.lock().unwrap() = session_name;

    let mdns = mdns_state.inner().clone();
    mdns::advertise::advertise(&mdns, port, &instance_name).await?;
    Ok(port)
}

/// Closes the session: kicks every joined peer and stops advertising.
/// Deliberately does NOT tear down the listener — that stays up for the
/// device's whole lifetime (see `listener::start_listening`), since other
/// peers may still want to dial this device directly even after it stops
/// being a named, joinable session.
#[tauri::command]
pub async fn stop_hosting(
    mdns_state: State<'_, Arc<MdnsState>>,
    session_state: State<'_, Arc<SessionState>>,
) -> Result<(), String> {
    session_state.broadcast(ControlMessage::Kick { reason: "Host stopped the session".into() });

    let mdns = mdns_state.inner().clone();
    mdns::advertise::stop(&mdns).await
}
