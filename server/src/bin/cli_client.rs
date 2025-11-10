//! CLI client for interacting with Automerge documents via WebSocket
//!
//! Usage:
//!   cargo run --bin cli_client -- <document-id> [command]
//!
//! Commands:
//!   increment        - Increment the counter by 1
//!   decrement        - Decrement the counter by 1
//!   set-counter <n>  - Set counter to specific value
//!   add-note <text>  - Add text to notes
//!   add-user <name>  - Add a collaborator
//!   show             - Just display the current document state

use anyhow::{Context, Result};
use automerge::{transaction::Transactable, AutoCommit, ObjType, ReadDoc};
use futures_util::{SinkExt, StreamExt};
use serde_json;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info};

#[derive(Debug)]
enum Command {
    Increment,
    Decrement,
    SetCounter(i64),
    AddNote(String),
    AddUser(String),
    Show,
}

impl Command {
    fn from_args(args: &[String]) -> Result<Self> {
        if args.is_empty() {
            return Ok(Command::Show);
        }

        match args[0].as_str() {
            "increment" => Ok(Command::Increment),
            "decrement" => Ok(Command::Decrement),
            "set-counter" => {
                let value = args
                    .get(1)
                    .context("set-counter requires a value")?
                    .parse::<i64>()?;
                Ok(Command::SetCounter(value))
            }
            "add-note" => {
                let text = args
                    .get(1)
                    .context("add-note requires text")?
                    .clone();
                Ok(Command::AddNote(text))
            }
            "add-user" => {
                let name = args
                    .get(1)
                    .context("add-user requires a name")?
                    .clone();
                Ok(Command::AddUser(name))
            }
            "show" => Ok(Command::Show),
            _ => anyhow::bail!("Unknown command: {}", args[0]),
        }
    }
}

async fn execute_command(doc: &mut AutoCommit, command: &Command) -> Result<()> {
    match command {
        Command::Increment => {
            let current = get_counter(doc);
            doc.put(automerge::ROOT, "counter", current + 1)?;
            info!("✅ Incremented counter: {} → {}", current, current + 1);
        }
        Command::Decrement => {
            let current = get_counter(doc);
            doc.put(automerge::ROOT, "counter", current - 1)?;
            info!("✅ Decremented counter: {} → {}", current, current - 1);
        }
        Command::SetCounter(value) => {
            let current = get_counter(doc);
            doc.put(automerge::ROOT, "counter", *value)?;
            info!("✅ Set counter: {} → {}", current, value);
        }
        Command::AddNote(text) => {
            let current_notes = get_notes(doc);
            let new_notes = if current_notes.is_empty() {
                text.clone()
            } else {
                format!("{}\n{}", current_notes, text)
            };
            doc.put(automerge::ROOT, "notes", new_notes.as_str())?;
            info!("✅ Added note: {}", text);
        }
        Command::AddUser(name) => {
            // Get or create collaborators list
            let collaborators = match doc.get(automerge::ROOT, "collaborators")? {
                Some((automerge::Value::Object(ObjType::List), id)) => id,
                _ => {
                    let id = doc.put_object(automerge::ROOT, "collaborators", ObjType::List)?;
                    id
                }
            };

            // Check if user already exists
            let existing_users = get_collaborators(doc);
            if existing_users.contains(name) {
                info!("⚠️  User '{}' already in collaborators list", name);
            } else {
                let len = doc.length(&collaborators);
                doc.insert(&collaborators, len, name.as_str())?;
                info!("✅ Added collaborator: {}", name);
            }
        }
        Command::Show => {
            // Just display, no changes
        }
    }
    Ok(())
}

fn get_counter(doc: &AutoCommit) -> i64 {
    match doc.get(automerge::ROOT, "counter") {
        Ok(Some((automerge::Value::Scalar(s), _))) => s.as_ref().to_i64().unwrap_or(0),
        _ => 0,
    }
}

fn get_notes(doc: &AutoCommit) -> String {
    match doc.get(automerge::ROOT, "notes") {
        Ok(Some((automerge::Value::Scalar(s), _))) => {
            s.as_ref().to_str().map(|s| s.to_string()).unwrap_or_default()
        }
        _ => String::new(),
    }
}

fn get_collaborators(doc: &AutoCommit) -> Vec<String> {
    match doc.get(automerge::ROOT, "collaborators") {
        Ok(Some((automerge::Value::Object(ObjType::List), id))) => {
            let len = doc.length(&id);
            (0..len)
                .filter_map(|i| {
                    doc.get(&id, i).ok().and_then(|opt| {
                        opt.and_then(|(val, _)| match val {
                            automerge::Value::Scalar(s) => {
                                s.as_ref().to_str().map(|s| s.to_string())
                            }
                            _ => None,
                        })
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn display_document(doc: &AutoCommit) {
    println!("\n📄 Document State:");
    println!("  Counter: {}", get_counter(doc));
    println!("  Notes: {}", get_notes(doc));
    let collaborators = get_collaborators(doc);
    if collaborators.is_empty() {
        println!("  Collaborators: (none)");
    } else {
        println!("  Collaborators:");
        for user in collaborators {
            println!("    - {}", user);
        }
    }
    println!();
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <document-id> [command] [args...]", args[0]);
        eprintln!();
        eprintln!("Commands:");
        eprintln!("  increment              - Increment the counter by 1");
        eprintln!("  decrement              - Decrement the counter by 1");
        eprintln!("  set-counter <n>        - Set counter to specific value");
        eprintln!("  add-note <text>        - Add text to notes");
        eprintln!("  add-user <name>        - Add a collaborator");
        eprintln!("  show                   - Display current document state (default)");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {} 2mdM9TnM2sJgLhHhYjyBzfusSsyr increment", args[0]);
        std::process::exit(1);
    }

    let doc_id = &args[1];
    let command = Command::from_args(&args[2..])?;

    info!("🔌 Connecting to ws://localhost:3030...");
    let (ws_stream, _) = connect_async("ws://localhost:3030")
        .await
        .context("Failed to connect to WebSocket server")?;

    let (mut write, mut read) = ws_stream.split();

    info!("📡 Connected! Document ID: {}", doc_id);

    // Request the existing document from the server
    info!("📥 Requesting existing document from server...");
    let request = serde_json::json!({
        "type": "get",
        "doc_id": doc_id
    });
    write.send(Message::Text(request.to_string())).await?;

    // Wait for the document response
    let mut doc = match timeout(Duration::from_secs(3), read.next()).await {
        Ok(Some(Ok(Message::Binary(data)))) if data.len() > 36 => {
            let doc_bytes = &data[36..];
            match AutoCommit::load(doc_bytes) {
                Ok(loaded_doc) => {
                    info!("✅ Loaded existing document from server");
                    loaded_doc
                }
                Err(e) => {
                    info!("⚠️  Could not load document ({}), creating new one", e);
                    let mut new_doc = AutoCommit::new();
                    new_doc.put(automerge::ROOT, "counter", 0_i64)?;
                    new_doc.put(automerge::ROOT, "notes", "")?;
                    new_doc.put_object(automerge::ROOT, "collaborators", ObjType::List)?;
                    new_doc
                }
            }
        }
        _ => {
            info!("⚠️  No existing document found, creating new one");
            let mut new_doc = AutoCommit::new();
            new_doc.put(automerge::ROOT, "counter", 0_i64)?;
            new_doc.put(automerge::ROOT, "notes", "")?;
            new_doc.put_object(automerge::ROOT, "collaborators", ObjType::List)?;
            new_doc
        }
    };

    info!("📄 Current document state before changes:");
    display_document(&doc);

    // Execute the command
    execute_command(&mut doc, &command).await?;

    // Display current state
    display_document(&doc);

    // Prepare the message: doc_id (36 bytes) + automerge data
    let doc_bytes = doc.save();
    let mut message = vec![0u8; 36 + doc_bytes.len()];

    // Pad doc_id to 36 bytes
    let doc_id_bytes = doc_id.as_bytes();
    let copy_len = doc_id_bytes.len().min(36);
    message[..copy_len].copy_from_slice(&doc_id_bytes[..copy_len]);
    message[36..].copy_from_slice(&doc_bytes);

    info!("📤 Sending changes to server ({} bytes)...", message.len());
    write.send(Message::Binary(message)).await?;

    // Wait for response with timeout
    info!("⏳ Waiting for server response...");
    match timeout(Duration::from_secs(2), read.next()).await {
        Ok(Some(Ok(Message::Binary(data)))) => {
            if data.len() > 36 {
                let response_bytes = &data[36..];
                match AutoCommit::load(response_bytes) {
                    Ok(response_doc) => {
                        info!("✅ Received updated document from server");
                        display_document(&response_doc);
                    }
                    Err(e) => {
                        error!("Failed to parse server response: {}", e);
                    }
                }
            }
        }
        Ok(Some(Ok(_))) => {
            info!("Received non-binary response from server");
        }
        Ok(Some(Err(e))) => {
            error!("Error reading from WebSocket: {}", e);
        }
        Ok(None) => {
            info!("Server closed connection");
        }
        Err(_) => {
            info!("⏰ Timeout waiting for response (changes may still have been applied)");
        }
    }

    info!("✨ Done!");
    Ok(())
}
