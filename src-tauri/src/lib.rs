use axum::{
    Router, extract::ws::{Message, WebSocket, WebSocketUpgrade}, response::Response, routing::{any, get},
};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use serde::{Deserialize, Serialize};

use std::{
    net::Ipv4Addr,
    sync::Arc,
};

use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "hello")]
    Hello {
        display_name: String,
    },

    #[serde(rename = "message")]
    Chat {
        message: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "connected")]
    Connected {
        user_id: String,
    },

    #[serde(rename = "user_joined")]
    UserJoined {
        user_id: String,
        display_name: String,
    },

    #[serde(rename = "message")]
    Chat {
        user_id: String,
        message: String,
    },
}

#[derive(Clone)]
struct AppState {
    events: broadcast::Sender<ServerMessage>,
}

#[tauri::command]
async fn app() {
    let port = 5000;

    // --------------------------------------------------
    // mDNS
    // --------------------------------------------------

    advertise_service(port);

    // --------------------------------------------------
    // Application state
    // --------------------------------------------------

    let (events, _) = broadcast::channel::<ServerMessage>(100);

    let state = Arc::new(AppState {
        events,
    });

    // --------------------------------------------------
    // HTTP / WebSocket server
    // --------------------------------------------------

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(state);

    let address = format!("0.0.0.0:{port}");

    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .unwrap();

    println!("LAN server running on {address}");

    axum::serve(listener, app)
        .await
        .unwrap();
}

#[tauri::command]
async fn browse_service(){
    let mdns = ServiceDaemon::new().expect("Faiiled to browse");

    let service_type = "_lanapp._tcp.local.";

    let receiver = mdns.browse(service_type).expect("Failed to browse service.");

    tokio::spawn(async move{
        while let Ok(event) = receiver.recv(){
            match event {
                ServiceEvent::ServiceResolved(resolved) =>{
                    println!("Resolved service: {:?}", resolved.fullname);
                }
                ServiceEvent::ServiceRemoved(service_type, fullname) =>{
                    println!("{}\n{} was removed", service_type, fullname);
                }
                other_event => {
                    println!("Received... {:?}", &other_event);
                }
            }
        }
    });

    
}

#[tauri::command]
fn advertise_service(port: u16) {
    let mdns = ServiceDaemon::new()
        .expect("Failed to create mDNS daemon");

    let service_type = "_lanapp._tcp.local.";

    let instance_name =
        "ThankGod-PC._lanapp._tcp.local.";

    let host_name =
        "thankgod-pc.local.";

    // let ip = Ipv4Addr::new(192, 168, 1, 10);

    let properties = [
        ("version", "1"),
        ("name", "ThankGod PC"),
        ("device", "desktop"),
    ];

    let service = ServiceInfo::new(
        service_type,
        instance_name,
        host_name,
        "",
        port,
        &properties[..],
    )
    .expect("Failed to create mDNS service").enable_addr_auto();

    mdns.register(service)
        .expect("Failed to register mDNS service");

    println!("mDNS service registered");

    // Keep daemon alive.
    std::mem::forget(mdns);
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(
    mut socket: WebSocket,
    state: Arc<AppState>,
) {
    let user_id = Uuid::new_v4().to_string();

    let mut events = state.events.subscribe();

    // Send initial connection message
    let connected = ServerMessage::Connected {
        user_id: user_id.clone(),
    };

    let json = serde_json::to_string(&connected).unwrap();

    if socket
        .send(Message::Text(json.into()))
        .await
        .is_err()
    {
        return;
    }

    loop {
        tokio::select! {

            // ------------------------------------------
            // Incoming client message
            // ------------------------------------------

            message = socket.recv() => {

                let Some(Ok(message)) = message else {
                    break;
                };

                match message {

                    Message::Text(text) => {

                        match serde_json::from_str::<ClientMessage>(&text) {

                            Ok(ClientMessage::Hello {
                                display_name
                            }) => {

                                println!(
                                    "{} connected as {}",
                                    user_id,
                                    display_name
                                );

                                let event =
                                    ServerMessage::UserJoined {
                                        user_id: user_id.clone(),
                                        display_name,
                                    };

                                let _ =
                                    state.events.send(event);
                            }

                            Ok(ClientMessage::Chat {
                                message
                            }) => {

                                let event =
                                    ServerMessage::Chat {
                                        user_id: user_id.clone(),
                                        message,
                                    };

                                let _ =
                                    state.events.send(event);
                            }

                            Err(error) => {
                                eprintln!(
                                    "Invalid message: {error}"
                                );
                            }
                        }
                    }

                    Message::Close(_) => {
                        break;
                    }

                    _ => {}
                }
            }

            // ------------------------------------------
            // Broadcast event
            // ------------------------------------------

            event = events.recv() => {

                if let Ok(event) = event {

                    let json =
                        serde_json::to_string(&event)
                            .unwrap();

                    if socket
                        .send(Message::Text(json.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    }

    println!("User disconnected: {user_id}");
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet,app,browse_service,advertise_service])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
