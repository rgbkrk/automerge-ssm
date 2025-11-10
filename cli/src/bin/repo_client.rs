//! CLI client using samod (automerge-repo for Rust)
//!
//! This CLI connects to an automerge-repo sync server and can
//! read/modify documents that are also being edited in the browser.
//!
//! Usage:
//!   cargo run --bin automerge-cli -- <automerge-url> [command]

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
#[command(about = "CLI client for Automerge documents using samod", long_about = None)]
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
    /// Add text to notes
    AddNote { text: String },
    /// Add a collaborator
    AddUser { name: String },
    /// Display current document state (default)
    Show,
}

#[derive(Debug, Clone, Default, Reconcile, Hydrate)]
struct Doc {
    counter: i64,
    notes: String,
    collaborators: Vec<String>,
}

impl Doc {
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

async fn execute_command(doc_handle: &samod::DocHandle, command: &Command) -> Result<()> {
    doc_handle.with_document(|doc| {
        // Hydrate current state from document
        let mut state: Doc = hydrate(doc).unwrap_or_default();

        // Apply command to local state
        match command {
            Command::Increment => {
                state.counter += 1;
                tracing::debug!("Incremented counter to {}", state.counter);
            }
            Command::Decrement => {
                state.counter -= 1;
                tracing::debug!("Decremented counter to {}", state.counter);
            }
            Command::SetCounter { value } => {
                state.counter = *value;
                tracing::debug!("Set counter to {}", value);
            }
            Command::AddNote { text } => {
                if state.notes.is_empty() {
                    state.notes = text.clone();
                } else {
                    state.notes = format!("{}\n{}", state.notes, text);
                }
                tracing::debug!("Added note");
            }
            Command::AddUser { name } => {
                if !state.collaborators.contains(name) {
                    state.collaborators.push(name.clone());
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
        }).expect("Failed to reconcile document");
    });

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
        hydrate(doc).unwrap_or_default()
    });

    doc_data.display();

    // Execute the command
    if !matches!(command, Command::Show) {
        execute_command(&doc_handle, &command).await?;

        println!("\n📄 After:");
        let doc_data: Doc = doc_handle.with_document(|doc| {
            hydrate(doc).unwrap_or_default()
        });
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
