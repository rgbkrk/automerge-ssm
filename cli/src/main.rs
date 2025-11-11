//! CLI client using samod (automerge-repo for Rust)
//!
//! Comprehensive CLI for the Autodash demo, showcasing all Automerge CRDT types.
//!
//! Usage:
//!   cargo run --bin automerge-cli -- <automerge-url> [command]

#![allow(non_snake_case)]

use anyhow::{Context, Result};
use automerge_cli::*;
use autosurgeon::{hydrate, reconcile};
use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use std::convert::Infallible;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};

// Global counter for unique todo IDs
static TODO_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

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
    /// Steadily increase temperature (1°C per 0.2s)
    Heat,
    /// Toggle dark mode
    ToggleDark,
    /// Set dark mode on/off
    SetDark { enabled: bool },
    /// Add text to notes
    AddNote { text: String },
    /// Clear notes field
    ClearNotes,
    /// Replace notes content
    SetNotes { text: String },
    /// Insert text at position in notes
    InsertNotes { position: usize, text: String },
    /// Delete characters from notes
    DeleteNotes { start: usize, length: usize },
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
    /// Set document title
    SetTitle { title: String },
    /// Display current document state (default)
    Show,
}



async fn heat_command(doc_handle: &samod::DocHandle) -> Result<()> {
    println!("\n🔥 Heating with smooth ease-in... (press Ctrl+C to stop)");
    println!("Starting from 0°C, easing to 40°C\n");

    // Set temperature to 0
    doc_handle.with_document(|doc| -> Result<()> {
        let mut state: Doc = hydrate(doc)?;
        state.temperature = 0;
        state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
        doc.transact(|tx| {
            reconcile(tx, &state)
        })
        .map_err(|e| anyhow::anyhow!("Failed to reconcile document: {:?}", e))?;
        Ok(())
    })?;

    println!("🌡️  Temperature: 0°C");
    sleep(Duration::from_millis(200)).await;

    // Ease-in animation: slow at start, fast at end
    let target_temp = 40.0;
    let duration_ms = 8000.0; // 8 seconds total
    let start_time = std::time::Instant::now();

    loop {
        let elapsed_ms = start_time.elapsed().as_millis() as f64;
        let progress = (elapsed_ms / duration_ms).min(1.0);

        // Ease-in quadratic: slow start, fast end
        let eased = progress.powf(2.0);
        let new_temp = (eased * target_temp).round() as i64;

        if new_temp >= 40 || progress >= 1.0 {
            // Final update to exactly 40
            doc_handle.with_document(|doc| -> Result<()> {
                let mut state: Doc = hydrate(doc)?;
                state.temperature = 40;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                doc.transact(|tx| {
                    reconcile(tx, &state)
                })
                .map_err(|e| anyhow::anyhow!("Failed to reconcile document: {:?}", e))?;
                Ok(())
            })?;
            println!("🌡️  Temperature: 40°C");
            println!("🔥 Maximum temperature reached!");
            break;
        }

        // Update temperature
        doc_handle.with_document(|doc| -> Result<()> {
            let mut state: Doc = hydrate(doc)?;
            if state.temperature != new_temp {
                state.temperature = new_temp;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                doc.transact(|tx| {
                    reconcile(tx, &state)
                })
                .map_err(|e| anyhow::anyhow!("Failed to reconcile document: {:?}", e))?;
            }
            Ok(())
        })?;

        let current_temp: i64 = doc_handle.with_document(|doc| -> Result<i64> {
            let data: Doc = hydrate(doc)?;
            Ok(data.temperature)
        })?;

        println!("🌡️  Temperature: {}°C", current_temp);

        sleep(Duration::from_millis(100)).await;
    }

    println!("\n📄 Final state:");
    let doc_data: Doc = doc_handle.with_document(|doc| {
        hydrate(doc).context("Failed to hydrate document")
    })?;
    doc_data.display();

    Ok(())
}

async fn execute_command(doc_handle: &samod::DocHandle, command: &Command) -> Result<()> {
    doc_handle.with_document(|doc| -> Result<()> {
        // Hydrate current state from document
        let mut state: Doc = hydrate(doc).context("Failed to hydrate document state")?;

        // Apply command to local state
        match command {
            Command::Increment => {
                state.counter += 1;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Incremented counter to {}", state.counter);
            }
            Command::Decrement => {
                state.counter -= 1;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Decremented counter to {}", state.counter);
            }
            Command::SetCounter { value } => {
                state.counter = *value;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Set counter to {}", value);
            }
            Command::SetTemp { value } => {
                let temp = (*value).clamp(0, 40);
                state.temperature = temp;
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Set temperature to {}°C", temp);
            }
            Command::Heat => {
                // Handled specially in heat_command() function
                tracing::debug!("Heat command - handled separately");
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
                let current = state.notes.clone();
                if current.is_empty() {
                    state.notes = text.clone();
                } else {
                    state.notes = format!("{}\n{}", current, text);
                }
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Added note");
            }
            Command::ClearNotes => {
                state.notes.clear();
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Cleared notes");
            }
            Command::SetNotes { text } => {
                state.notes = text.clone();
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Set notes to: {}", text);
            }
            Command::InsertNotes { position, text } => {
                // Convert character position to byte index
                let byte_pos = state.notes
                    .char_indices()
                    .nth(*position)
                    .map(|(idx, _)| idx)
                    .unwrap_or(state.notes.len());
                state.notes.insert_str(byte_pos, text);
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Inserted '{}' at character position {} (byte {})", text, position, byte_pos);
            }
            Command::DeleteNotes { start, length } => {
                // Convert character positions to byte indices
                let char_count = state.notes.chars().count();
                let start_char = (*start).min(char_count);
                let end_char = (start_char + length).min(char_count);

                if start_char < end_char {
                    let start_byte = state.notes
                        .char_indices()
                        .nth(start_char)
                        .map(|(idx, _)| idx)
                        .unwrap_or(state.notes.len());
                    let end_byte = state.notes
                        .char_indices()
                        .nth(end_char)
                        .map(|(idx, _)| idx)
                        .unwrap_or(state.notes.len());

                    state.notes.replace_range(start_byte..end_byte, "");
                    state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                    tracing::debug!("Deleted {} characters from position {}", end_char - start_char, start_char);
                }
            }
            Command::AddTodo { text } => {
                let counter = TODO_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
                let todo = TodoItem {
                    id: format!("{}-{}", chrono::Utc::now().timestamp_millis(), counter),
                    text: text.clone(),
                    completed: false,
                };
                state.todos.push(todo);
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
                if !state.tags.iter().any(|t| t == tag) {
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
            Command::SetTitle { title } => {
                state.metadata.title = Some(title.clone());
                state.metadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
                tracing::debug!("Set title to: {}", title);
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
        .with_storage(
            samod::storage::TokioFilesystemStorage::new("./autodash-data/")
            )
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

    // Special handling for Heat command
    if matches!(command, Command::Heat) {
        heat_command(&doc_handle).await?;
    } else {
        // Normal command execution
        // Display state before changes
        if !matches!(command, Command::Show) {
            println!("\n📄 Before:");
        }

        let doc_data: Doc = doc_handle.with_document(|doc| {
            match hydrate(doc) {
                Ok(data) => Ok(data),
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
    }

    // Give time for final messages to flush before disconnecting
    tracing::debug!("Waiting for sync to complete...");
    sleep(Duration::from_millis(100)).await;

    // Clean up connection tasks
    ws_to_samod_handle.abort();
    samod_to_ws_handle.abort();

    Ok(())
}
