use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::net::SocketAddr;
use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::{task::JoinHandle,net::{TcpListener,TcpSocket,TcpStream}};

#[derive(serde::Serialize,serde::Deserialize)]
enum ConnectionKind{
    Control,
    Transfer{
        transfer_id:String,
        filename:String,
        usize:u64
    }
}

pub struct TcpState{
    listener_task: Mutex<Option<JoinHandle<()>>>,
    bound_port: Mutex<Option<u16>>,
    active_transfers: Mutex<HashMap<String, JoinHandle<()>>>
}

impl TcpState{
    pub fn new() -> TcpState{
        TcpState { 
            listener_task: Mutex::new(None),
            bound_port: Mutex::new(None), 
            active_transfers: Mutex::new(HashMap::new())
        }
    }
}

pub struct MdnsState{
    daemon: ServiceDaemon,
    browse_task: Mutex<Option<JoinHandle<()>>>,
    service_fullname: Mutex<Option<String>>,
}

impl MdnsState{
    pub fn new() -> Self{
        Self{
            daemon: ServiceDaemon::new().expect("Failed to create mDNS daemon"),
            browse_task: Mutex::new(None),
            service_fullname:Mutex::new(None),
        }
    }
}

const SERVICE_TYPE: &str = "_lanapp._tcp.local.";


#[tauri::command]
pub async fn start_service(state:State<'_, MdnsState>, port:u16)-> Result<(),String>{
    let instance_name = "ThankGod-PC";
    let host_name = "thankgod-pc.local.";
    let properties = [("property_1","test"),("property_2","1234")];

    let service = ServiceInfo::new(
        SERVICE_TYPE,
        instance_name,
        host_name,
        "",
        port,
        &properties[..]
    ).map_err(|e| e.to_string())?.enable_addr_auto();

    let fullname = service.get_fullname().to_string();

    state.daemon.register(service).map_err(|e| e.to_string())?;
    *state.service_fullname.lock().unwrap() = Some(fullname);
    
    Ok(())
}

#[tauri::command]
pub async fn stop_advertising(state:State<'_, MdnsState>)-> Result<(),String> {
    let fullname: Option<String> = state.service_fullname.lock().unwrap().take();

    if let Some(name) = fullname{
        let rx = state.daemon.unregister(&name).map_err(|e| e.to_string())?;
        let _ = tokio::time::timeout(Duration::from_secs(2), rx.recv_async()).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn browse_service(state:State<'_,MdnsState>,app:AppHandle)->Result<(),String>{
    let receiver = state.daemon.browse(SERVICE_TYPE).map_err(|e| e.to_string())?;

    let handle = tokio::spawn(async move {
        while let Ok(event) = receiver.recv_async().await{
            match event{
                ServiceEvent::ServiceResolved(info)=>{
                    let _= app.emit("mdns-service-resolved", info.get_fullname().to_string());
                }
                ServiceEvent::ServiceRemoved(_, fullname) =>{
                    let _ = app.emit("mdns-service-removed",fullname);
                }
                _ =>{

                }
            }
        }
    });

    *state.browse_task.lock().unwrap() = Some(handle);

    println!("Started browsing for service(s)");

    Ok(())
}

#[tauri::command]
pub async fn stop_browsing(state:State<'_,MdnsState>)->Result<(),String>{
    let handle = state.browse_task.lock().unwrap().take();

    if let Some(event) = handle{
        event.abort();
    }

    Ok(())
}


#[tauri::command]
pub async fn start_hosting(
    app:AppHandle, 
    tcp_state:State<'_,TcpState>,
    mdns_state: State<'_,MdnsState>
)->Result<u16,String>{
    let listener = TcpListener::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    let shared = Arc::new(listener);

    let listener = Arc::clone(&shared);
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let app_handle = app.clone();

    let handle = tokio::spawn(async move{
        loop{
            match listener.accept().await{
                Ok((socket,addr))=>{
                    let app_handle = app_handle.clone();
                    
                    tokio::spawn(async move{
                        println!("Successfully connected to server!");
                        // handle_connection(socket, addr, app_handle).await;
                    });
                },
                Err(e) =>{
                    eprintln!("Accept error: {e}");
                    break;
                }
            }
        }
    });

    *tcp_state.listener_task.lock().unwrap() = Some(handle);
    *tcp_state.bound_port.lock().unwrap() = Some(port);

    start_service(mdns_state, port).await?;

    println!("Servie up and running on port: {}", port);
    println!("TCP Listening on {}", shared.local_addr().map_err(|e| e.to_string())?);

    Ok(port)
}

#[tauri::command]
pub async fn stop_hosting(
    tcp_state:State<'_,TcpState>,
    mdns_state: State<'_,MdnsState>
) -> Result<(),String>{
    if let Some(handle) = tcp_state.listener_task.lock().unwrap().take(){
        handle.abort();
    }

    for (_,handle) in tcp_state.active_transfers.lock().unwrap().drain(){
        handle.abort();
    }

    *tcp_state.bound_port.lock().unwrap() = None;
    stop_advertising(mdns_state).await.unwrap();

    Ok(())
}

// async fn handle_conection(mut socket: TcpStream, addr: SocketAddr, app:AppHandle){
//     let  kind = match recv_msg(&mut socket).await{
//         Ok(k) => k,
//         Err(-)=> return
//     }

//     match kind{}
// }

