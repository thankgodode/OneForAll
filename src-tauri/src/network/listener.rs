use crate::context::AppContext;
use crate::network::dispatch::handle_connection;
use crate::state::{AppSettings, DeviceInfo, PendingIncoming, SessionState, TcpState};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::net::TcpListener;

/// Binds a listener and spawns its accept loop. Used by BOTH roles:
/// hosting a discoverable session and simply being reachable so other
/// peers can dial you directly for a transfer. Every participant needs
/// this — not just whoever is "hosting" — which is why it's split out
/// from `network::hosting::start_hosting` rather than being part of it.
pub async fn bind_and_serve(ctx: AppContext, app: AppHandle) -> Result<u16, String> {
    let listener = TcpListener::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let tcp_state = ctx.tcp.clone();
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
    Ok(port)
}

/// Call once, at app startup, regardless of whether this device ends up
/// hosting or joining — both roles need a real listening port before
/// they do anything else. Hosting reuses this port for its mDNS
/// advertisement (see `hosting::start_hosting`); joining reports it to
/// the host as part of the join request, so the roster entry the host
/// hands out for THIS device is actually dialable rather than pointing
/// at the ephemeral source port of its outbound control connection.
#[tauri::command]
pub async fn start_listening(
    app: AppHandle,
    tcp_state: State<'_, Arc<TcpState>>,
    session_state: State<'_, Arc<SessionState>>,
    settings_state: State<'_, Arc<AppSettings>>,
    pending_state: State<'_, Arc<PendingIncoming>>,
    device_state: State<'_, Arc<DeviceInfo>>,
) -> Result<u16, String> {
    let ctx = AppContext {
        session: session_state.inner().clone(),
        tcp: tcp_state.inner().clone(),
        settings: settings_state.inner().clone(),
        pending_incoming: pending_state.inner().clone(),
        device: device_state.inner().clone(),
    };
    bind_and_serve(ctx, app).await
}
