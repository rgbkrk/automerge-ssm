//! CLI client using samod (automerge-repo for Rust)
//!
//! Comprehensive CLI for the Autodash demo, showcasing all Automerge CRDT types.
//!
//! Usage:
//!   cargo run --bin automerge-cli -- <automerge-url> [command]

#![allow(non_snake_case)]

use anyhow::{Context, Result};
use autosurgeon::{hydrate, reconcile, Hydrate, Reconcile};
use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use std::convert::Infallible;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Parser)]
#[command(name = "automerge-cli")]
#[command(about = "CLI client for Autodash - Comprehensive Automerge demo", long_about = None)]
struct Cli {
    /// Automerge document URL or full browser URL
    /// Examples:
    ///   automerge:4VgLSsiuVNfWeZk17m85GgA18VVp
    ///   http://localhost:5173/#automerge:4VgLSsiuVNfWeZk17m85GgA18VVp
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
    /// Set temperature value (0-40)
    SetTemp { value: i64 },
    /// Toggle dark mode
    ToggleDark,
    /// Set dark mode on/off
    SetDark { enabled: bool },
    /// Add text to notes
    AddNote { text: String },
    /// Add a todo item
    AddTodo { text: String },
    /// Toggle todo completion
    ToggleTodo { id: String },
    /// Delete a todo
    DeleteTodo { id: String },
    /// Add a tag
    AddTag { tag: String },
    /// Remove a tag
    RemoveTag { tag: String },
    /// Add a collaborator
    AddUser { name: String },
    /// Display current document state (default)
    Show,
}

#[derive(Debug, Clone, Reconcile, Hydrate)]
struct TodoItem {
    #[autosurgeon(hydrate = "hydrate_string_or_text")]
    id: String,
    #[autosurgeon(hydrate = "hydrate_string_or_text")]
    text: String,
    completed: bool,
}

// Helper function to hydrate strings that might be stored as Text objects (from JS)
// or as scalar strings (from Rust)
fn hydrate_string_or_text<D: autosurgeon::ReadDoc>(
    doc: &D,
    obj: &automerge::ObjId,
    prop: autosurgeon::Prop,
) -> Result<String, autosurgeon::HydrateError> {
    use automerge::{ObjType, Value};

    tracing::debug!("hydrate_string_or_text: prop={:?}", prop);
    match doc.get(obj, &prop)? {
        Some((Value::Scalar(s), _)) => {
            // Scalar string - direct case
            Ok(s.to_str()
                .ok_or_else(|| autosurgeon::HydrateError::unexpected("string", format!("scalar {:?}", s)))?
                .to_string())
        }
        Some((Value::Object(ObjType::Text), text_obj)) => {
            // Text object - from JavaScript
            doc.text(&text_obj).map_err(|e| {
                autosurgeon::HydrateError::unexpected("text object", format!("error reading text: {}", e))
            })
        }
        Some((val, _)) => {
            tracing::error!("hydrate_string_or_text: unexpected value type for prop={:?}, val={:?}", prop, val);
            Err(autosurgeon::HydrateError::unexpected("string or text", format!("{:?}", val)))
        }
        None => {
            tracing::debug!("hydrate_string_or_text: prop={:?} is None, returning empty string", prop);
            Ok(String::new())
        }
    }
}

// Helper function to hydrate Option<String> that might be stored as Text object (from JS)
fn hydrate_optional_string_or_text<D: autosurgeon::ReadDoc>(
    doc: &D,
    obj: &automerge::ObjId,
    prop: autosurgeon::Prop,
) -> Result<Option<String>, autosurgeon::HydrateError> {
    use automerge::{ObjType, Value};

    tracing::debug!("hydrate_optional_string_or_text: prop={:?}", prop);
    match doc.get(obj, &prop)? {
        Some((Value::Scalar(s), _)) => {
            // Scalar string - direct case
            Ok(Some(s.to_str()
                .ok_or_else(|| autosurgeon::HydrateError::unexpected("string", format!("scalar {:?}", s)))?
                .to_string()))
        }
        Some((Value::Object(ObjType::Text), text_obj)) => {
            // Text object - from JavaScript
            doc.text(&text_obj).map(Some).map_err(|e| {
                autosurgeon::HydrateError::unexpected("text object", format!("error reading text: {}", e))
            })
        }
        Some((val, _)) => {
            tracing::error!("hydrate_optional_string_or_text: unexpected value type for prop={:?}, val={:?}", prop, val);
            Err(autosurgeon::HydrateError::unexpected("string or text", format!("{:?}", val)))
        }
        None => {
            tracing::debug!("hydrate_optional_string_or_text: prop={:?} is None", prop);
            Ok(None)
        }
    }
}

// Helper function to hydrate Vec<String> that might contain Text objects (from JS)
fn hydrate_string_vec_or_text<D: autosurgeon::ReadDoc>(
    doc: &D,
    obj: &automerge::ObjId,
    prop: autosurgeon::Prop,
) -> Result<Vec<String>, autosurgeon::HydrateError> {
    use automerge::{ObjType, Value};

    tracing::debug!("hydrate_string_vec_or_text: prop={:?}", prop);
    match doc.get(obj, &prop)? {
        Some((Value::Object(ObjType::List), list_obj)) => {
            let len = doc.length(&list_obj);
            let mut result = Vec::new();

            for i in 0..len {
                match doc.get(&list_obj, i)? {
                    Some((Value::Scalar(s), _)) => {
                        if let Some(text) = s.to_str() {
                            result.push(text.to_string());
                        }
                    }
                    Some((Value::Object(ObjType::Text), text_obj)) => {
                        if let Ok(text) = doc.text(&text_obj) {
                            result.push(text);
                        }
                    }
                    _ => {}
                }
            }
            Ok(result)
        }
        None => {
            tracing::debug!("hydrate_string_vec_or_text: prop={:?} is None, returning empty vec", prop);
            Ok(Vec::new())
        }
        Some((val, _)) => {
            tracing::error!("hydrate_string_vec_or_text: unexpected value type for prop={:?}, val={:?}", prop, val);
            Err(autosurgeon::HydrateError::unexpected("list", format!("{:?}", val)))
        }
    }
}

#[derive(Debug, Clone, Default, Reconcile, Hydrate)]
struct Metadata {
    createdAt: Option<i64>,
    lastModified: Option<i64>,
    #[autosurgeon(hydrate = "hydrate_optional_string_or_text")]
    title: Option<String>,
}

#[derive(Debug, Clone, Default, Reconcile, Hydrate)]
struct Stats {
    totalEdits: i64,
    activeUsers: i64,
}

#[derive(Debug, Clone, Default, Reconcile, Hydrate)]
struct Doc {
    counter: i64,
    temperature: i64,
    darkMode: bool,
    #[autosurgeon(hydrate = "hydrate_string_or_text")]
    notes: String,
    todos: Vec<TodoItem>,
    #[autosurgeon(hydrate = "hydrate_string_vec_or_text")]
    tags: Vec<String>,
    #[autosurgeon(hydrate = "hydrate_string_vec_or_text")]
    collaborators: Vec<String>,
    metadata: Metadata,
    stats: Stats,
}

impl Doc {
    fn display(&self) {
        println!("\n📊 Autodash State:");
        println!("╭─────────────────────────────────────────╮");

        // Basic types
        println!("│ 🔢 Counter: {:<28}│", self.counter);
        println!("│ 🌡️  Temperature: {}°C{:<23}│", self.temperature, "");
        println!("│ 🌙 Dark Mode: {:<26}│", if self.darkMode { "ON" } else { "OFF" });

        // Text
        if self.notes.is_empty() {
            println!("│ 📝 Notes: (empty){:<23}│", "");
        } else {
            let preview = if self.notes.len() > 30 {
                format!("{}...", &self.notes[..27])
            } else {
                self.notes.clone()
            };
            println!("│ 📝 Notes: {:<28}│", preview);
        }

        // Lists
        println!("│ ✓  Todos: {:<28}│", self.todos.len());
        println!("│ 🏷️  Tags: {:<29}│", self.tags.len());
        println!("│ 👥 Collaborators: {:<22}│", self.collaborators.len());

        // Metadata
        if let Some(title) = &self.metadata.title {
            println!("│ 📄 Title: {:<28}│", title);
        }
        println!("│ 📊 Total Edits: {:<22}│", self.stats.totalEdits);
        println!("│ 👤 Active Users: {:<21}│", self.stats.activeUsers);

        println!("╰─────────────────────────────────────────╯");

        // Details
        if !self.todos.is_empty() {
            println!("\n✓ Todos:");
            for todo in &self.todos {
                let status = if todo.completed { "✓" } else { "○" };
                println!("  {} [{}] {}", status, &todo.id[..8], todo.text);
            }
        }

        if !self.tags.is_empty() {
            println!("\n🏷️  Tags: {}", self.tags.join(", "));
        }

        if !self.collaborators.is_empty() {
            println!("\n👥 Collaborators:");
            for user in &self.collaborators {
                println!("  • {}", user);
            }
        }

        println!();
    }
}

async fn execute_command(doc_handle: &samod::DocHandle, command: &Command) -> Result<()> {
    doc_handle.with_document(|doc| -> Result<()> {
        // Hydrate current state from document
        let mut state: Doc = hydrate(doc).context("Failed to hydrate document state")?;

        // Apply command to local state
        match command {
            Command::Increment => {
                state.counter += 1;
                state.stats.totalEdits += 1;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Incremented counter to {}", state.counter);
            }
            Command::Decrement => {
                state.counter -= 1;
                state.stats.totalEdits += 1;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Decremented counter to {}", state.counter);
            }
            Command::SetCounter { value } => {
                state.counter = *value;
                state.stats.totalEdits += 1;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Set counter to {}", value);
            }
            Command::SetTemp { value } => {
                let temp = (*value).clamp(0, 40);
                state.temperature = temp;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Set temperature to {}°C", temp);
            }
            Command::ToggleDark => {
                state.darkMode = !state.darkMode;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Toggled dark mode to {}", state.darkMode);
            }
            Command::SetDark { enabled } => {
                state.darkMode = *enabled;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Set dark mode to {}", enabled);
            }
            Command::AddNote { text } => {
                if state.notes.is_empty() {
                    state.notes = text.clone();
                } else {
                    state.notes = format!("{}\n{}", state.notes, text);
                }
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Added note");
            }
            Command::AddTodo { text } => {
                let todo = TodoItem {
                    id: format!("{}", chrono::Utc::now().timestamp_millis()),
                    text: text.clone(),
                    completed: false,
                };
                state.todos.push(todo);
                state.stats.totalEdits += 1;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Added todo: {}", text);
            }
            Command::ToggleTodo { id } => {
                if let Some(todo) = state.todos.iter_mut().find(|t| t.id.starts_with(id)) {
                    todo.completed = !todo.completed;
                    state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                    tracing::debug!("Toggled todo {}", id);
                } else {
                    tracing::warn!("Todo {} not found", id);
                }
            }
            Command::DeleteTodo { id } => {
                if let Some(pos) = state.todos.iter().position(|t| t.id.starts_with(id)) {
                    state.todos.remove(pos);
                    state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                    tracing::debug!("Deleted todo {}", id);
                } else {
                    tracing::warn!("Todo {} not found", id);
                }
            }
            Command::AddTag { tag } => {
                if !state.tags.contains(tag) {
                    state.tags.push(tag.clone());
                    state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                    tracing::debug!("Added tag: {}", tag);
                } else {
                    tracing::debug!("Tag '{}' already exists", tag);
                }
            }
            Command::RemoveTag { tag } => {
                if let Some(pos) = state.tags.iter().position(|t| t == tag) {
                    state.tags.remove(pos);
                    state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                    tracing::debug!("Removed tag: {}", tag);
                } else {
                    tracing::warn!("Tag '{}' not found", tag);
                }
            }
            Command::AddUser { name } => {
                if !state.collaborators.contains(name) {
                    state.collaborators.push(name.clone());
                    state.stats.activeUsers = state.collaborators.len() as i64;
                    tracing::debug!("Added collaborator: {}", name);
                } else {
                    tracing::debug!("User '{}' already exists", name);
                }
            }
            Command::Show => {
                // No changes needed
            }
        }

        // Reconcile changes back to document
        doc.transact(|tx| {
            reconcile(tx, &state)
        })
        .map_err(|e| anyhow::anyhow!("Failed to reconcile document: {:?}", e))?;

        Ok(())
    })?;

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

    // Parse the automerge URL - accept both plain URLs and browser URLs
    let doc_id_str = if let Some(hash_pos) = doc_url.find("#automerge:") {
        // Extract from browser URL: http://localhost:5173/#automerge:DOCID
        &doc_url[hash_pos + 11..] // Skip "#automerge:"
    } else if doc_url.starts_with("automerge:") {
        // Plain automerge URL: automerge:DOCID
        doc_url.strip_prefix("automerge:").unwrap()
    } else {
        anyhow::bail!(
            "URL must contain 'automerge:' or '#automerge:' - got: {}",
            doc_url
        );
    };

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

    let doc_data: Doc = doc_handle.with_document(|doc| {
        tracing::debug!("Attempting to hydrate document...");
        match hydrate(doc) {
            Ok(data) => {
                tracing::debug!("Successfully hydrated document");
                Ok(data)
            }
            Err(e) => {
                tracing::error!("Failed to hydrate document: {:?}", e);
                Err(anyhow::anyhow!("Failed to hydrate document for display: {:?}", e))
            }
        }
    })?;

    doc_data.display();

    // Execute the command
    if !matches!(command, Command::Show) {
        execute_command(&doc_handle, &command).await?;

        println!("\n📄 After:");
        let doc_data: Doc = doc_handle.with_document(|doc| {
            hydrate(doc).context("Failed to hydrate document after command")
        })?;
        doc_data.display();
    }

    // Give time for final messages to flush before disconnecting
    tracing::debug!("Waiting for sync to complete...");
    sleep(Duration::from_millis(100)).await;

    // Clean up connection tasks
    ws_to_samod_handle.abort();
    samod_to_ws_handle.abort();

    Ok(())
}
