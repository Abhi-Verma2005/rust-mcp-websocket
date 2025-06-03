use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use serde::Serialize;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
mod types;
use types::{CommMessage, Version};

type Tx = broadcast::Sender<RoomMessage>;
type _Rx = broadcast::Receiver<RoomMessage>;
type Clients = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
type ClientRooms = Arc<Mutex<HashMap<SocketAddr, HashSet<String>>>>;
type Rooms = Arc<Mutex<HashMap<String, HashSet<SocketAddr>>>>;


#[derive(Serialize)]

struct QuestionPayload<'a> {
    question_id: &'a str,
    question: serde_json::Value
}
#[derive(Serialize)]
struct AddQuestionBody<'a> {
    contest_id: &'a str,
    questions: Vec<QuestionPayload<'a>>
}


#[derive(Serialize)]
struct UpdateContestMessage<'a> {
    version: &'a str,
    questions: serde_json::Value
}


#[derive(Clone, Debug)]
struct RoomMessage {
    room_id: String,
    content: String
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a broadcast channel for message distribution
    let (tx, _rx) = broadcast::channel::<RoomMessage>(100);
    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(&addr).await?;
    
    println!("WebSocket server listening on: {}", addr);

    let rooms: Rooms = Arc::new(Mutex::new(HashMap::new()));

    let client_rooms: ClientRooms = Arc::new(Mutex::new(HashMap::new()));
    
    while let Ok((stream, addr)) = listener.accept().await {
        let tx_clone = tx.clone();
        let clients_clone = clients.clone();
        
        tokio::spawn(handle_connection(stream, addr, tx_clone, clients_clone, rooms.clone(), client_rooms.clone()));
    }
    
    Ok(())
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    tx: Tx,
    clients: Clients,
    rooms: Rooms,
    client_rooms: ClientRooms
) {
    println!("Client connected: {}", addr);
    
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => {
            println!("Failed WebSocket handshake: {}", addr);
            return;
        }
    };
    
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut rx: broadcast::Receiver<RoomMessage> = tx.subscribe();
    
    {
        let mut clients_lock = clients.lock().await;
        clients_lock.insert(addr, tx.clone());
    }
    
    let _ = ws_sender.send(Message::Text("Welcome to Rust WebSocket server!".into())).await;
    
    let tx_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if ws_sender.send(Message::Text(msg.content.into())).await.is_err() {
                break;
            }
        }
    });
    
    let addr_clone2 = addr.clone();
    let clients_clone = clients.clone();
    let rooms_clone = rooms.clone();
    let client_rooms_clone = client_rooms.clone();
    let rx_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Frame(_)) => todo!(),
                Ok(Message::Text(text)) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                        println!("Message parsed: {parsed}");
                        if let Some(msg_type) = parsed["type"].as_str() {
                            match msg_type {
                                "joinContest" => {
                                    if let Some(contest_id) = parsed["data"]["contest_id"].as_str() {
                                        {
                                            let mut rooms_lock = rooms_clone.lock().await;
                                            let entry = rooms_lock.entry(contest_id.to_string()).or_default();
                                            entry.insert(addr_clone2);
                                        }

                                        {
                                            let mut client_rooms_lock = client_rooms_clone.lock().await;
                                            let entry = client_rooms_lock.entry(addr_clone2).or_default();
                                            entry.insert(contest_id.to_string());
                                        }
                                        println!("{addr_clone2} joined the contest {contest_id}");
                                    }
                                }
                                "addQuestion" => {
                                    if let Some(contest_id) = parsed["data"]["contest_id"].as_str() {
                                        let question_data = &parsed["data"]["q"];
                                        if let Some(question_id) = question_data["id"].as_str() {
                                            let body = AddQuestionBody {
                                                questions: vec![QuestionPayload {
                                                    question: question_data.clone(),
                                                    question_id: question_id
                                                }],
                                                contest_id: contest_id 
                                            };

                                            let client = reqwest::Client::new();
                                            match client.post("http://localhost:3000/api/realTimeAddQuestion").json(&body).send().await {
                                                Ok(resp) => {
                                                    if resp.status().is_success() {
                                                        let questions: serde_json::Value = resp.json().await.unwrap_or_default();
                                                        let rooms_lock = rooms_clone.lock().await;
                                                        if let Some(participants) = rooms_lock.get(contest_id) {
                                                            let client_lock = clients_clone.lock().await; 
                                                            let message_content = UpdateContestMessage {
                                                                version: "contest_update",
                                                                questions: questions
                                                            };

                                                            match serde_json::to_string(&message_content) {
                                                                Ok(json_string) => {
                                                                    for participant_addr in participants {
                                                                        if let Some(participant_tx) = client_lock.get(participant_addr) {
                                                                            let roommessage = RoomMessage {
                                                                                room_id: contest_id.to_string(),
                                                                                content: json_string.clone()
                                                                            };
                                                                            println!("Reached here, sending message");
                                                                            let _ = participant_tx.send(roommessage.clone());
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    println!("Some error occured whiile serding to json string: {}", e)
                                                                }
                                                            }

                                                            
                                                        }
                                                    } else {
                                                        println!("Api error: {}", resp.status());
                                                    }
                                                }
                                                Err(e) => {
                                                    println!("HTTP error: {:?}", e);
                                                }
                                            }
                                        }
                                     }
                                }

                                _ => {
                                    println!("Exception")
                                }
                            }
                        }
                        if let Some (_) = parsed["version"].as_str() {
                            let mcp_parsed = serde_json::from_str::<CommMessage>(&text);
                            match mcp_parsed {
                                Ok(comm) => {
                                    match comm.version {
                                        // added the user to room for chat with ai 
                                        Version::NewChatRoom => {
                                            if !comm.user_email.is_empty() {
                                               {
                                                    let mut rooms_lock = rooms_clone.lock().await;
                                                    let entry = rooms_lock.entry(comm.user_email.to_string()).or_default();
                                                    entry.insert(addr_clone2);
                                               }
                                               println!("{addr_clone2} joined the chat room");
                                            }
                                        }
                                        // here will eceive message 
                                        Version::Message => {

                                        }
                                        _ => {

                                        }
                                    }
                                    
                                }
                                Err(e) => {
                                    println!("Some Error Occured: {}", e);
                                    
                                }
                            };

                        }
                       
                    }
                    

                }
                Ok(Message::Close(_)) => break,
                Ok(Message::Ping(_)) => {
                    // WebSocket will automatically respond with pong
                }
                Ok(Message::Pong(_)) => {}
                Ok(Message::Binary(_)) => {}
                Err(_) => break,
            }
        }
    });
    
    tokio::select! {
        _ = tx_task => {},
        _ = rx_task => {},
    }
    
    {
        let mut clients_lock = clients.lock().await;
        clients_lock.remove(&addr);
    }

    {
        let mut client_rooms_lock = client_rooms.lock().await;
        if let Some(client_room_set) = client_rooms_lock.remove(&addr) {
            let mut rooms_lock = rooms.lock().await;
            for room_id in client_room_set {
                if let Some(participants) = rooms_lock.get_mut(&room_id) {
                    participants.remove(&addr);
                    if participants.is_empty() {
                        rooms_lock.remove(&room_id);
                    }
                }
            }
            
        }
    }
    
    println!("Client disconnected: {}", addr);
}