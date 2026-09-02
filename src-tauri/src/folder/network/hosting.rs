use crate::context::AppContext;
use crate::mdns;
use crate::network::dispatch::handle_connection;
use crate::state::{AppSettings, MdnsState, PendingIncoming, SessionState, TcpState};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::net::TcpListener;

/// Binds a listener, THEN advertises it over mDNS — never the other way
/// round. If a peer resolved the service before the port was live, it
/// would dial straight into connection-refused. Binding to port 0 lets
/// the OS pick a free port; we read the real port back before telling
/// anyone about it.
#[tauri::command]
pub async fn start_hosting(
    app: AppHandle,
    tcp_state: State<'_, Arc<TcpState>>,
    mdns_state: State<'_, Arc<MdnsState>>,
    session_state: State<'_, Arc<SessionState>>,
    settings_state: State<'_, Arc<AppSettings>>,
    pending_state: State<'_, Arc<PendingIncoming>>,
    session_name: String,
    instance_name: String,
) -> Result<u16, String> {
    let listener = TcpListener::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    *session_state.session_name.lock().unwrap() = session_name;

    let ctx = AppContext {
        session: session_state.inner().clone(),
        tcp: tcp_state.inner().clone(),
        settings: settings_state.inner().clone(),
        pending_incoming: pending_state.inner().clone(),
    };

    let app_for_loop = app.clone();
    let handle = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    let ctx = ctx.clone();
                    let app = app_for_loop.clone();
                    tokio::spawn(async move {
                        handle_connection(socket, addr, ctx, app).await;
                    });
                }
                Err(e) => {
                    eprintln!("accept error: {e}");
                    break;
                }
            }
        }
    });

    *tcp_state.listener_task.lock().unwrap() = Some(handle);
    *tcp_state.bound_port.lock().unwrap() = Some(port);

    let mdns = mdns_state.inner().clone();
    mdns::advertise::advertise(&mdns, port, &instance_name).await?;

    Ok(port)
}

/// Stops accepting new connections, cancels every in-flight transfer
/// (cooperatively — see `TransferHandle`), and unregisters the mDNS
/// advertisement. Order matters less here than at startup: nothing bad
/// happens if the advertisement briefly outlives the listener by a few
/// milliseconds, unlike the reverse race at startup.
#[tauri::command]
pub async fn stop_hosting(
    tcp_state: State<'_, Arc<TcpState>>,
    mdns_state: State<'_, Arc<MdnsState>>,
) -> Result<(), String> {
    if let Some(handle) = tcp_state.listener_task.lock().unwrap().take() {
        handle.abort();
    }
    for (_, handle) in tcp_state.active_transfers.lock().unwrap().drain() {
        handle.cancel.cancel();
    }
    *tcp_state.bound_port.lock().unwrap() = None;

    let mdns = mdns_state.inner().clone();
    mdns::advertise::stop(&mdns).await
}
