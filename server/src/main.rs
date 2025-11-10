use anyhow::Result;
use automerge::AutoCommit;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{error, info, warn};

type DocumentStore = Arc<RwLock<HashMap<String, Arc<RwLock<AutoCommit>>>>>;
type Clients = Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Vec<u8>>>>>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "sync")]
    Sync { doc_id: String, sync_message: Vec<u8> },
    #[serde(rename = "get")]
    GetDocument { doc_id: String },
    #[serde(rename = "create")]
    CreateDocument { doc_id: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "sync")]
    Sync { doc_id: String, sync_message: Vec<u8> },
    #[serde(rename = "document")]
    Document { doc_id: String, data: Vec<u8> },
    #[serde(rename = "error")]
    Error { message: String },
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    documents: DocumentStore,
    clients: Clients,
) {
    info!("New connection from: {}", addr);

    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("Error during websocket handshake: {}", e);
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    let client_id = uuid::Uuid::new_v4().to_string();
    clients.write().await.insert(client_id.clone(), tx);

    // Task to send messages to client
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(Message::Binary(msg)).await {
                error!("Error sending message: {}", e);
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = ws_receiver.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                error!("Error receiving message: {}", e);
                break;
            }
        };

        match msg {
            Message::Text(text) => {
                if let Err(e) = handle_text_message(&text, &documents, &clients, &client_id).await {
                    error!("Error handling message: {}", e);
                }
            }
            Message::Binary(data) => {
                if let Err(e) = handle_binary_message(&data, &documents, &clients, &client_id).await {
                    error!("Error handling binary message: {}", e);
                }
            }
            Message::Close(_) => {
                info!("Client {} disconnected", addr);
                break;
            }
            _ => {}
        }
    }

    // Cleanup
    clients.write().await.remove(&client_id);
    send_task.abort();
    info!("Connection closed: {}", addr);
}

async fn handle_text_message(
    text: &str,
    documents: &DocumentStore,
    clients: &Clients,
    _client_id: &str,
) -> Result<()> {
    let msg: ClientMessage = serde_json::from_str(text)?;

    match msg {
        ClientMessage::CreateDocument { doc_id } => {
            let mut docs = documents.write().await;
            if !docs.contains_key(&doc_id) {
                let doc = AutoCommit::new();
                docs.insert(doc_id.clone(), Arc::new(RwLock::new(doc)));
                info!("Created new document: {}", doc_id);
            }
        }
        ClientMessage::GetDocument { doc_id } => {
            let docs = documents.read().await;
            if let Some(doc_lock) = docs.get(&doc_id) {
                let mut doc = doc_lock.write().await;
                let data = doc.save();

                let response = ServerMessage::Document {
                    doc_id,
                    data,
                };

                // Send to all clients
                let response_data = serde_json::to_vec(&response)?;
                for (_, client_tx) in clients.read().await.iter() {
                    let _ = client_tx.send(response_data.clone());
                }
            }
        }
        ClientMessage::Sync { doc_id, sync_message: _ } => {
            // This would need proper Automerge sync protocol implementation
            info!("Received sync message for doc: {}", doc_id);
        }
    }

    Ok(())
}

async fn handle_binary_message(
    data: &[u8],
    documents: &DocumentStore,
    clients: &Clients,
    client_id: &str,
) -> Result<()> {
    // Simple protocol: first 36 bytes are document ID (UUID), rest is Automerge changes
    if data.len() < 36 {
        return Ok(());
    }

    let doc_id = String::from_utf8_lossy(&data[0..36]).to_string();
    let changes_data = &data[36..];

    let mut docs = documents.write().await;
    let doc_lock = docs.entry(doc_id.clone())
        .or_insert_with(|| Arc::new(RwLock::new(AutoCommit::new())));

    let mut doc = doc_lock.write().await;

    // Try to load changes into the document
    if let Ok(loaded_doc) = AutoCommit::load(changes_data) {
        // Merge the changes
        if let Err(e) = doc.merge(&mut loaded_doc.clone()) {
            warn!("Error merging document: {}", e);
        } else {
            info!("Merged changes for document: {}", doc_id);

            // Broadcast to other clients
            let response_data = doc.save();
            let mut full_message = doc_id.as_bytes().to_vec();
            full_message.extend_from_slice(&response_data);

            for (cid, client_tx) in clients.read().await.iter() {
                if cid != client_id {
                    let _ = client_tx.send(full_message.clone());
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:3030".parse::<SocketAddr>()?;
    let listener = TcpListener::bind(&addr).await?;

    info!("Automerge WebSocket server listening on: {}", addr);

    let documents: DocumentStore = Arc::new(RwLock::new(HashMap::new()));
    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

    while let Ok((stream, addr)) = listener.accept().await {
        let documents = documents.clone();
        let clients = clients.clone();

        tokio::spawn(async move {
            handle_connection(stream, addr, documents, clients).await;
        });
    }

    Ok(())
}
