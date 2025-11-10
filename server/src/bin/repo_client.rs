//! CLI client using samod (automerge-repo for Rust)
//!
//! This CLI connects to an automerge-repo sync server and can
//! read/modify documents that are also being edited in the browser.
//!
//! Usage:
//!   cargo run --bin automerge-cli -- <automerge-url> [command]

use anyhow::{Context, Result};
use automerge::{transaction::Transactable, ObjType, ReadDoc};
use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use std::convert::Infallible;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Parser)]
#[command(name = "automerge-cli")]
#[command(about = "CLI client for Automerge documents using samod", long_about = None)]
struct Cli {
    /// Automerge document URL (e.g., automerge:4VgLSsiuVNfWeZk17m85GgA18VVp)
    #[arg(value_name = "URL")]
    doc_url: String,

    /// Enable verbose debug logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Increment the counter by 1
    Increment,
    /// Decrement the counter by 1
    Decrement,
    /// Set counter to a specific value
    SetCounter { value: i64 },
    /// Add text to notes
    AddNote { text: String },
    /// Add a collaborator
    AddUser { name: String },
    /// Display current document state (default)
    Show,
}

#[derive(Debug, Clone, Default)]
struct Doc {
    counter: i64,
    notes: String,
    collaborators: Vec<String>,
}

impl Doc {
    fn from_automerge(doc: &automerge::Automerge) -> Result<Self> {
        let counter = get_counter(doc);
        let notes = get_notes(doc)?;
        let collaborators = get_collaborators(doc)?;

        Ok(Doc {
            counter,
            notes,
            collaborators,
        })
    }

    fn display(&self) {
        println!("\n📄 Document State:");
        println!("  Counter: {}", self.counter);
        if self.notes.is_empty() {
            println!("  Notes: (empty)");
        } else {
            println!("  Notes: {}", self.notes);
        }
        if self.collaborators.is_empty() {
            println!("  Collaborators: (none)");
        } else {
            println!("  Collaborators:");
            for user in &self.collaborators {
                println!("    - {}", user);
            }
        }
        println!();
    }
}

fn get_counter(doc: &automerge::Automerge) -> i64 {
    match doc.get(automerge::ROOT, "counter") {
        Ok(Some((automerge::Value::Scalar(s), _))) => s.to_i64().unwrap_or(0),
        _ => 0,
    }
}

fn get_notes(doc: &automerge::Automerge) -> Result<String> {
    match doc.get(automerge::ROOT, "notes") {
        Ok(Some((automerge::Value::Scalar(s), _))) => {
            Ok(s.to_str().map(|s| s.to_string()).unwrap_or_default())
        }
        Ok(Some((automerge::Value::Object(ObjType::Text), obj_id))) => {
            doc.text(&obj_id).context("Failed to read text object")
        }
        Ok(None) => Ok(String::new()),
        Ok(Some((val, _))) => {
            anyhow::bail!("Unexpected type for notes field: {:?}", val)
        }
        Err(e) => Err(e).context("Failed to get notes field"),
    }
}

fn get_collaborators(doc: &automerge::Automerge) -> Result<Vec<String>> {
    match doc.get(automerge::ROOT, "collaborators") {
        Ok(Some((automerge::Value::Object(ObjType::List), id))) => {
            let len = doc.length(&id);
            let mut collaborators = Vec::new();

            for i in 0..len {
                match doc.get(&id, i) {
                    Ok(Some((automerge::Value::Scalar(s), _))) => {
                        if let Some(text) = s.to_str() {
                            collaborators.push(text.to_string());
                        }
                    }
                    Ok(Some((automerge::Value::Object(ObjType::Text), obj_id))) => {
                        if let Ok(text) = doc.text(&obj_id) {
                            collaborators.push(text);
                        }
                    }
                    _ => continue,
                }
            }
            Ok(collaborators)
        }
        Ok(None) => Ok(Vec::new()),
        Ok(Some((val, _))) => {
            anyhow::bail!("Unexpected type for collaborators field: {:?}", val)
        }
        Err(e) => Err(e).context("Failed to get collaborators field"),
    }
}

async fn execute_command(doc_handle: &samod::DocHandle, command: &Command) -> Result<()> {
    match command {
        Command::Increment => {
            doc_handle.with_document(|doc| {
                let current = get_counter(doc);
                doc.transact(|tx| {
                    tx.put(automerge::ROOT, "counter", current + 1)?;
                    Ok::<_, automerge::AutomergeError>(())
                }).expect("Failed to increment counter");
            });
            tracing::debug!("Incremented counter");
        }
        Command::Decrement => {
            doc_handle.with_document(|doc| {
                let current = get_counter(doc);
                doc.transact(|tx| {
                    tx.put(automerge::ROOT, "counter", current - 1)?;
                    Ok::<_, automerge::AutomergeError>(())
                }).expect("Failed to decrement counter");
            });
            tracing::debug!("Decremented counter");
        }
        Command::SetCounter { value } => {
            doc_handle.with_document(|doc| {
                doc.transact(|tx| {
                    tx.put(automerge::ROOT, "counter", *value)?;
                    Ok::<_, automerge::AutomergeError>(())
                }).expect("Failed to set counter");
            });
            tracing::debug!("Set counter to {}", value);
        }
        Command::AddNote { text } => {
            doc_handle.with_document(|doc| {
                doc.transact(|tx| {
                    // Get existing notes
                    let notes_obj = match tx.get(automerge::ROOT, "notes")? {
                        Some((automerge::Value::Object(ObjType::Text), id)) => id,
                        _ => {
                            // Create as Text object for JS compatibility
                            tx.put_object(automerge::ROOT, "notes", ObjType::Text)?
                        }
                    };

                    // Get current text
                    let current = tx.text(&notes_obj)?;
                    let new_text = if current.is_empty() {
                        text.clone()
                    } else {
                        format!("{}\n{}", current, text)
                    };

                    // Replace all text
                    tx.splice_text(&notes_obj, 0, current.len() as isize, &new_text)?;
                    Ok::<_, automerge::AutomergeError>(())
                }).expect("Failed to add note");
            });
            tracing::debug!("Added note");
        }
        Command::AddUser { name } => {
            doc_handle.with_document(|doc| {
                doc.transact(|tx| {
                    // Get or create collaborators list
                    let collaborators = match tx.get(automerge::ROOT, "collaborators")? {
                        Some((automerge::Value::Object(ObjType::List), id)) => id,
                        _ => tx.put_object(automerge::ROOT, "collaborators", ObjType::List)?,
                    };

                    // Check if user already exists
                    let len = tx.length(&collaborators);
                    let mut exists = false;
                    for i in 0..len {
                        if let Ok(Some((automerge::Value::Object(ObjType::Text), obj_id))) = tx.get(&collaborators, i) {
                            if let Ok(text) = tx.text(&obj_id) {
                                if text == *name {
                                    exists = true;
                                    break;
                                }
                            }
                        }
                    }

                    if !exists {
                        // Insert as Text object for JS compatibility
                        let text_obj = tx.insert_object(&collaborators, len, ObjType::Text)?;
                        tx.splice_text(&text_obj, 0, 0, name)?;
                        tracing::debug!("Added collaborator: {}", name);
                    } else {
                        tracing::debug!("User '{}' already exists", name);
                    }

                    Ok::<_, automerge::AutomergeError>(())
                }).expect("Failed to add user");
            });
        }
        Command::Show => {
            // Just display, no changes
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging based on verbose flag
    tracing_subscriber::fmt()
        .with_env_filter(
            if cli.verbose {
                tracing_subscriber::EnvFilter::new("debug")
            } else {
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"))
            },
        )
        .init();

    let doc_url = &cli.doc_url;
    let command = cli.command.unwrap_or(Command::Show);

    // Parse the automerge URL
    if !doc_url.starts_with("automerge:") {
        anyhow::bail!("URL must start with 'automerge:' - got: {}", doc_url);
    }

    let doc_id_str = doc_url.strip_prefix("automerge:").unwrap();

    tracing::debug!("Initializing automerge-repo");

    // Create a repo with in-memory storage
    let repo = samod::Repo::build_tokio()
        .with_storage(samod::storage::InMemoryStorage::new())
        .load()
        .await;

    tracing::debug!("Connecting to sync server");

    // Connect to WebSocket server using tokio-tungstenite
    let (ws_stream, _) = connect_async("ws://localhost:3030")
        .await
        .context("Failed to connect to WebSocket server")?;

    tracing::debug!("WebSocket connected");

    let (ws_sink, ws_stream) = ws_stream.split();

    // Create channels to bridge WebSocket and samod
    let (to_samod_tx, to_samod_rx) = futures::channel::mpsc::unbounded::<Vec<u8>>();
    let (from_samod_tx, from_samod_rx) = futures::channel::mpsc::unbounded::<Vec<u8>>();

    // Forward WebSocket messages to samod
    let ws_to_samod = async move {
        let mut stream = ws_stream;
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    if to_samod_tx.unbounded_send(data).is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) => break,
                Ok(_) => {} // Ignore text/ping/pong
                Err(e) => {
                    tracing::warn!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    };

    // Forward samod messages to WebSocket
    let samod_to_ws = async move {
        use futures_util::SinkExt;
        let mut rx = from_samod_rx;
        let mut sink = ws_sink;
        while let Some(bytes) = rx.next().await {
            if sink.send(Message::Binary(bytes)).await.is_err() {
                break;
            }
        }
    };

    // Spawn the connection handling tasks
    let ws_to_samod_handle = tokio::spawn(ws_to_samod);
    let samod_to_ws_handle = tokio::spawn(samod_to_ws);

    // Connect the repo to the sync server
    let repo_clone = repo.clone();
    tokio::spawn(async move {
        let result = repo_clone
            .connect(
                to_samod_rx.map(Ok::<_, Infallible>),
                from_samod_tx,
                samod::ConnDirection::Outgoing,
            )
            .await;

        tracing::debug!("Sync connection finished: {:?}", result);
    });

    tracing::debug!("Loading document: automerge:{}", doc_id_str);

    // Create DocumentId from string
    let doc_id: samod::DocumentId = doc_id_str.parse()?;

    // Try to find the document (may return None if not synced yet)
    tracing::debug!("Looking for document...");
    let mut doc_handle = repo.find(doc_id.clone()).await?;

    // TODO: Replace sleep with proper reactive sync completion detection
    // Race condition: samod's sync happens asynchronously. We need to wait for
    // the document to be fully synced from the server before reading it.
    // Proper fix: Listen for sync state changes or document ready events.
    // Current workaround: Sleep to allow sync protocol to complete.
    if doc_handle.is_none() {
        tracing::debug!("Document not immediately available, waiting for sync...");
        sleep(Duration::from_secs(2)).await;

        // Try again after sync
        doc_handle = repo.find(doc_id.clone()).await?;
    } else {
        tracing::debug!("Document found, waiting for full sync...");
        sleep(Duration::from_secs(2)).await;
    }

    let doc_handle = doc_handle.context(
        "Document not found. Make sure:\n  1. The sync server is running\n  2. The document exists in the browser\n  3. The document ID is correct"
    )?;

    // Display state before changes
    if !matches!(command, Command::Show) {
        println!("\n📄 Before:");
    }

    let doc_data = doc_handle.with_document(|doc| {
        Doc::from_automerge(doc)
    })?;

    doc_data.display();

    // Execute the command
    if !matches!(command, Command::Show) {
        execute_command(&doc_handle, &command).await?;

        println!("\n📄 After:");
        let doc_data = doc_handle.with_document(|doc| {
            Doc::from_automerge(doc)
        })?;
        doc_data.display();
    }

    // Clean up
    ws_to_samod_handle.abort();
    samod_to_ws_handle.abort();

    Ok(())
}
