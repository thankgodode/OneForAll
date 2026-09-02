use tauri::{AppHandle, Emitter, State};
use tokio::net::TcpStream;
use tokio::sync::futures;
use uuid::Uuid;
use std::net::{SocketAddr};
use tokio::task::JoinSet;
use std::sync::{Mutex,Arc};
use std::collections::HashMap;
use tokio::{sync::{mpsc::UnboundedSender}};
use hostname;

use crate::channels::{recv_msg,send_msg};

#[derive(serde::Serialize,serde::Deserialize,Clone)]
pub struct PeerInfo{
    pub id: String,
    pub display_name: String,
    pub device_name: String,
    pub addr: String,
}

#[derive(serde::Serialize,serde::Deserialize,Clone)]
pub enum ControlMessage{
    JoinRequest{
        code: Option<String>, 
        display_name:String,
        device_name:String
    },
    JoinPending,
    JoinAccepted{
        session_name: String,
        peer_id: String,
        roster: Vec<PeerInfo>,
    },
    JoinRejected{
        reason:String,
    },
    PeerJoined(PeerInfo),
    PeerLeft{
        peer_id: String,
    },
    Kick{
        reason: String
    },
    Leave,
    Ping,
    Pong    
}

struct PendingJoin{
    info: PeerInfo,
    responder: tokio::sync::oneshot::Sender<bool>
}

pub struct PeerHandle{
    info: PeerInfo,
    outbox: UnboundedSender<ControlMessage>
}

pub struct SessionState{
    code: Mutex<Option<String>>,
    auto_accept: Mutex<bool>,
    session_name: Mutex<String>,
    peers: Mutex<HashMap<String,PeerHandle>>,
    pending: Mutex<HashMap<String,PeerHandle>>,
}

impl SessionState{
    fn new(session_name: String, code:Option<String>)-> Self{
        let self_id = "self".to_string();
        let mut peers = HashMap::new();

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        peers.insert(self_id.clone(),PeerHandle{
            info: PeerInfo { id: self_id, display_name: "YOU".into(), device_name: device_name(), addr: "self".into() },
            outbox:tx
        });

        Self{
            code: Mutex::new(code),
            auto_accept: Mutex::new(false),
            session_name: Mutex::new(session_name),
            peers: Mutex::new(peers),
            pending: Mutex::new(HashMap::new())
        }
    }

    fn roster(&self) -> Vec<PeerInfo>{
        self.peers.lock().unwrap().values().map(|p| p.info.clone()).collect()
    }

    fn is_member(&self, peer_id: &str) -> bool{
        self.peers.lock().unwrap().contains_key(peer_id)
    }

    fn broadcast(&self, msg: ControlMessage) where ControlMessage: Clone{
        for peer in self.peers.lock().unwrap().values(){
            let _ = peer.outbox.send(msg.clone());
        }
    }
}

pub struct ClientSessionState {
    my_peer_id: Mutex<Option<String>>,
    session_name: Mutex<Option<String>>,
    roster: Mutex<Vec<PeerInfo>>,
}

impl ClientSessionState {
    fn new() -> Self {
        Self { my_peer_id: Mutex::new(None), session_name: Mutex::new(None), roster: Mutex::new(Vec::new()) }
    }

    fn roster(&self) -> Vec<PeerInfo> {
        self.roster.lock().unwrap().clone()
    }

    fn my_peer_id(&self) -> String {
        self.my_peer_id.lock().unwrap().clone().unwrap_or_default()
    }

    fn set_joined(&self, session_name: String, peer_id: String, roster: Vec<PeerInfo>) {
        *self.session_name.lock().unwrap() = Some(session_name);
        *self.my_peer_id.lock().unwrap() = Some(peer_id);
        *self.roster.lock().unwrap() = roster;
    }

    fn add_peer(&self, peer: PeerInfo) {
        self.roster.lock().unwrap().push(peer);
    }

    fn remove_peer(&self, peer_id: &str) {
        self.roster.lock().unwrap().retain(|p| p.id != peer_id);
    }

    fn clear(&self) {
        *self.my_peer_id.lock().unwrap() = None;
        *self.session_name.lock().unwrap() = None;
        self.roster.lock().unwrap().clear();
    }
}

#[derive(serde::Serialize,serde::Deserialize)]
enum ConnectionKind{
    Control,
    Transfer{
        peer_id: String,
        transfer_id:String,
        filename:String,
        size:u64
    }
}

fn device_name() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "Unknown Device".into())
}

async fn handle_control_connection(
    mut socket: TcpStream,
    addr: SocketAddr,
    session: Arc<SessionState>,
    app: AppHandle,
){
    let msg = match recv_msg(&mut socket).await{
        Ok(msg) => msg,
        Err(_) => {
            return
        }
    };
    

    let (peer_id, display_name, device_name) =  match msg{
        ControlMessage::JoinRequest { code, display_name, device_name } => {
            if code != *session.code.lock().unwrap(){
                let _ = send_msg(&mut socket, &ControlMessage::JoinRejected{
                    reason:"Invalid code".into(),
                }).await;
    
                return
            }

            (uuid::Uuid::new_v4().to_string(),display_name, device_name)
        }
        _ => return
    };

    let info = PeerInfo { id: peer_id.clone(), display_name, device_name, addr: addr.to_string() };
    let (outbox_tx, mut outbox_rx) = tokio::sync::mpsc::unbounded_channel();

    session.peers.lock().unwrap().insert(peer_id.clone(), PeerHandle { info: info.clone(), outbox: outbox_tx });

    let _ = send_msg(&mut socket, &ControlMessage::JoinAccepted {
        session_name: session.session_name.lock().unwrap().clone(),
        peer_id: peer_id.clone(),
        roster: session.roster(),
    }).await;

    session.broadcast(ControlMessage::PeerJoined(info));
    let _ = app.emit("peer-joined", peer_id.clone());

    let (mut read_half, mut write_half) = socket.into_split();

    // writer: forward anything queued for this peer (roster updates, Kick) down the socket
    let writer = tokio::spawn(async move {
        while let Some(msg) = outbox_rx.recv().await {
            if send_msg(&mut write_half, &msg).await.is_err() { break; }
        }
    });

    // reader: mostly waiting for Leave or the connection dying
    loop {
        match recv_msg(&mut read_half).await {
            Ok(ControlMessage::Leave) | Err(_) => break,
            Ok(ControlMessage::Ping) => { /* update last_seen if you track it */ }
            _ => {}
        }
    }

    writer.abort();
    session.peers.lock().unwrap().remove(&peer_id);
    session.broadcast(ControlMessage::PeerLeft { peer_id: peer_id.clone() });
    let _ = app.emit("peer-left", peer_id);

}

pub async fn handle_connection(mut socket: TcpStream, addr: SocketAddr, session: Arc<SessionState>,app: AppHandle){
    let kind = match recv_msg(&mut socket).await{
        Ok(k) => k,
        Err(_) => return
    };

    match kind{
        ConnectionKind::Control => handle_control_connection(socket, addr, session, app).await,
        ConnectionKind::Transfer { peer_id, transfer_id, filename, size } =>{
            if !session.is_member(&peer_id){
                return;
            }

            // receive_file(socket, transfer_id,t, filename, size, app).await;
        }
    }
}

#[tauri::command]
async fn join_network(app: AppHandle, host_addr: String, display_name: String) -> Result<PeerInfo, String> {
    let mut socket = TcpStream::connect(&host_addr).await.map_err(|e| e.to_string())?;
    send_msg(&mut socket, &ControlMessage::JoinRequest {
        display_name, device_name: device_name(), code: None,
    }).await.map_err(|e| e.to_string())?;

    loop {
        match recv_msg(&mut socket).await {
            Ok(ControlMessage::JoinPending) => { let _ = app.emit("join-pending", ()); } // UI: "Waiting for host…"
            Ok(ControlMessage::JoinAccepted { session_name, peer_id, roster }) => {
                let _ = app.emit("joined-network", (&session_name, &roster));
                let me = PeerInfo { id: peer_id, display_name: "You".into(), device_name: device_name(), addr: host_addr };
                tokio::spawn(listen_for_roster_updates(socket, app));
                println!("Connection accepted!");
                return Ok(me);
            }
            Ok(ControlMessage::JoinRejected { reason }) => return Err(reason),
            _ => return Err("Connection closed".into()),
        }
    }
}

async fn listen_for_roster_updates(mut socket: TcpStream, app: AppHandle){
    loop{
        match recv_msg(&mut socket).await{
            Ok(ControlMessage::PeerJoined(p)) => {
                let _ = app.emit("peer-joined", p);
            }
            Ok(ControlMessage::PeerLeft{peer_id}) => {
                let _ = app.emit("peer-left", peer_id);
            },
            Ok(ControlMessage::Kick{reason}) =>{
                let _ = app.emit("kicked", reason);
            }
            Err(_) => {
                let _ = app.emit("disconnected-from-network", ());
                break;
            }
            _ => {}
        }
    }
}

#[tauri::command]
async fn send_file_to_peers(
    app: AppHandle,
    session: State<'_, ClientSessionState>,
    file_path: String,
    target_peer_ids: Vec<String>,
) -> Result<(), String> {
    let roster = session.roster();
    let file_path = Arc::new(file_path);
    let mut set = JoinSet::new();

    for target_id in target_peer_ids {
        let Some(target) = roster.iter().find(|p| p.id == target_id).cloned() else { continue };
        let app = app.clone();
        let file_path = file_path.clone();
        let my_id = session.my_peer_id();
        let transfer_id = uuid::Uuid::new_v4().to_string();

        set.spawn(async move {
            send_one_file(app, file_path, target, my_id, transfer_id).await
        });
    }

    // results arrive as each peer's transfer finishes, not all-at-once
    while let Some(result) = set.join_next().await {
        if let Err(e) = result {
            eprintln!("transfer task panicked: {e}");
        }
    }
    Ok(())
}
async fn send_one_file(app: AppHandle, file_path: Arc<String>, target: PeerInfo, my_id: String,transfer_id: String){
    let Ok(mut socket)     = TcpStream::connect(&target.addr).await else{
        let _ = app.emit("transfer-failed", (transfer_id,"Connection failed"));
        return;
    };

    let metadata = tokio::fs::metadata(&*file_path).await.unwrap();
    let filename = std::path::Path::new(&*file_path).file_name().unwrap().to_string_lossy().to_string();

    let _ = send_msg(&mut socket, &ConnectionKind::Transfer { peer_id: my_id, filename, size: metadata.len(),transfer_id: transfer_id.clone()}).await;

    // stream_file_with_progress(&mut socket, &*file_path, &transfer_id, &target.id, &app).await;

}

#[tauri::command]
fn kick_peer(session:State<'_, SessionState>, peer_id: String){
    if let Some(peer) = session.peers.lock().unwrap().get(&peer_id){
        let _ = peer.outbox.send(ControlMessage::Kick{reason:"Removed by host".into()});
    }
}