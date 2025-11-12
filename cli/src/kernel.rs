//! Hokey Kernel Server
//!
//! A simple Rust-based kernel that watches an Automerge document and executes
//! notebook cells, updating them with fake outputs.
//!
//! Usage:
//!   cargo run --bin automerge-kernel -- <automerge-url>

#![allow(non_snake_case)]

use anyhow::{Context, Result};
use automerge_cli::*;
use autosurgeon::{hydrate, reconcile};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <automerge-url>", args[0]);
        eprintln!("Example: {} automerge:4VgLSsiuVNfWeZk17m85GgA18VVp", args[0]);
        eprintln!("       or {} http://localhost:5173/#automerge:4VgLSsiuVNfWeZk17m85GgA18VVp", args[0]);
        std::process::exit(1);
    }

    let doc_url = &args[1];

    // Parse the automerge URL
    let doc_id_str = if let Some(hash_pos) = doc_url.find("#automerge:") {
        &doc_url[hash_pos + 11..]
    } else if doc_url.starts_with("automerge:") {
        doc_url.strip_prefix("automerge:").unwrap()
    } else {
        anyhow::bail!(
            "URL must contain 'automerge:' or '#automerge:' - got: {}",
            doc_url
        );
    };

    println!("\n🔬 Hokey Kernel Server Starting...");
    println!("📄 Document ID: {}", doc_id_str);
    println!("🔌 Connecting to sync server at ws://localhost:3030...\n");

    // Create a repo with filesystem storage
    let repo = samod::Repo::build_tokio()
        .with_storage(samod::storage::TokioFilesystemStorage::new("./autodash-data/"))
        .load()
        .await;

    // Connect to WebSocket server
    let (ws_stream, _) = connect_async("ws://localhost:3030")
        .await
        .context("Failed to connect to WebSocket server")?;

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
        let mut rx = from_samod_rx;
        let mut sink = ws_sink;
        while let Some(bytes) = rx.next().await {
            if sink.send(Message::Binary(bytes)).await.is_err() {
                break;
            }
        }
    };

    // Spawn the connection handling tasks
    tokio::spawn(ws_to_samod);
    tokio::spawn(samod_to_ws);

    // Connect the repo to the sync server
    use std::convert::Infallible;
    use futures_util::stream::StreamExt as _;

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

    // Wait for document to sync if not immediately available
    if doc_handle.is_none() {
        tracing::debug!("Document not immediately available, waiting for sync...");
        sleep(Duration::from_secs(2)).await;
        doc_handle = repo.find(doc_id.clone()).await?;
    }

    let doc_handle = doc_handle.context("Document not found after sync wait")?;

    println!("✓ Connected and synced with document");
    println!("👀 Watching for cells to execute...\n");
    println!("🎯 Kernel will auto-execute cells when you click Run in the browser");
    println!("Press Ctrl+C to stop the kernel\n");

    // Track which cells we've already processed
    let mut last_execution_requests: HashMap<usize, Option<i64>> = HashMap::new();

    // Main loop: watch document and execute cells
    loop {
        sleep(Duration::from_millis(500)).await;

        // Check if any cells need execution
        let cells_to_execute = doc_handle.with_document(|doc| -> Result<Vec<usize>> {
            let state: Doc = hydrate(doc)?;
            let mut to_execute = Vec::new();

            for (idx, cell) in state.cells.iter().enumerate() {
                // Only execute code cells
                if cell.cellType != "code" {
                    continue;
                }

                let current_count = cell.executionCount;
                let last_count = last_execution_requests.get(&idx).copied().flatten();

                // Execute if execution count changed (user clicked Run)
                // This detects when frontend increments the count
                if current_count != last_count {
                    // Only execute if count increased (not initial load)
                    if let (Some(curr), Some(last)) = (current_count, last_count) {
                        if curr > last {
                            to_execute.push(idx);
                        }
                    } else if current_count.is_some() && last_count.is_none() {
                        // New cell with execution request
                        to_execute.push(idx);
                    }

                    // Update tracking
                    last_execution_requests.insert(idx, current_count);
                }
            }

            Ok(to_execute)
        })?;

        // Execute any pending cells
        for cell_idx in cells_to_execute {
            if let Err(e) = execute_cell(&doc_handle, cell_idx).await {
                eprintln!("❌ Error executing cell {}: {}", cell_idx, e);
            }
        }
    }
}

async fn execute_cell(doc_handle: &samod::DocHandle, cell_idx: usize) -> Result<()> {
    println!("⚡ Executing cell {}...", cell_idx);

    // Get cell info
    let (cell_id, source) = doc_handle.with_document(|doc| -> Result<(String, String)> {
        let state: Doc = hydrate(doc)?;
        if let Some(cell) = state.cells.get(cell_idx) {
            Ok((cell.id.clone(), cell.source.clone()))
        } else {
            Err(anyhow::anyhow!("Cell {} not found", cell_idx))
        }
    })?;

    let source_preview: String = source.chars().take(50).collect();
    println!("   📝 Source: {}...", source_preview);

    // Simulate execution time
    sleep(Duration::from_millis(200 + (source.len() as u64 * 2))).await;

    // Generate hokey output based on source code
    let output = generate_hokey_output(&source);

    let output_preview = output.data.get("text/plain")
        .map(|s| s.chars().take(60).collect::<String>())
        .unwrap_or_else(|| "".to_string());
    println!("   💬 Output: {}...", output_preview);

    // Store output (mock artifact storage)
    let output_id = format!("output-{}-{}", chrono::Utc::now().timestamp_millis(), cell_idx);
    let output_url = format!("hokey://localhost/outputs/{}", output_id);

    let output_json = serde_json::to_string_pretty(&output)?;
    std::fs::create_dir_all("./outputs")?;
    std::fs::write(format!("./outputs/{}.json", output_id), output_json)?;

    println!("   📦 Stored at: {}", output_url);

    // Update the document with execution results
    doc_handle.with_document(|doc| -> Result<()> {
        let mut state: Doc = hydrate(doc)?;

        if let Some(cell) = state.cells.get_mut(cell_idx) {
            // Keep the execution count (frontend already incremented it)
            // Just add the output reference
            cell.outputRefs = vec![output_url.clone()];

            state.notebookMetadata.lastModified = Some(chrono::Utc::now().timestamp_millis());
        }

        doc.transact(|tx| {
            reconcile(tx, &state)
        })
        .map_err(|e| anyhow::anyhow!("Failed to reconcile document: {:?}", e))?;

        Ok(())
    })?;

    println!("   ✅ Cell {} complete!\n", cell_idx);

    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Output {
    output_type: String,
    data: HashMap<String, String>,
}

fn generate_hokey_output(source: &str) -> Output {
    let mut data = HashMap::new();

    // Analyze the source and generate contextual output
    if source.contains("console.log") || source.contains("print") {
        // Extract what's being printed (very hokey parsing)
        let output_text = if let Some(start) = source.find("console.log(") {
            let rest = &source[start + 12..];
            if let Some(end) = rest.find(')') {
                rest[..end].trim_matches(|c| c == '"' || c == '\'' || c == '`').to_string()
            } else {
                "Hello from hokey kernel!".to_string()
            }
        } else if let Some(start) = source.find("print(") {
            let rest = &source[start + 6..];
            if let Some(end) = rest.find(')') {
                rest[..end].trim_matches(|c| c == '"' || c == '\'').to_string()
            } else {
                "Hello from hokey kernel!".to_string()
            }
        } else {
            "Output from hokey kernel".to_string()
        };

        data.insert("text/plain".to_string(), output_text);
    } else if source.contains("Math.") || source.to_lowercase().contains("calculate") {
        // Math operation
        data.insert("text/plain".to_string(), "42\n(calculated by hokey kernel)".to_string());
    } else if source.contains("fetch") || source.contains("http") {
        // HTTP request
        data.insert("text/plain".to_string(), "HTTP 200 OK\n{ \"message\": \"Success from hokey kernel\" }".to_string());
    } else if source.contains("import") || source.contains("require") {
        // Module import
        data.insert("text/plain".to_string(), "✓ Modules loaded successfully\n(hokey kernel)".to_string());
    } else if source.trim().is_empty() {
        // Empty cell
        data.insert("text/plain".to_string(), "".to_string());
    } else {
        // Generic output
        let line_count = source.lines().count();
        let char_count = source.chars().count();
        data.insert(
            "text/plain".to_string(),
            format!(
                "✓ Executed successfully\n\nCode stats:\n  {} lines\n  {} characters\n\nResult: Success\n\n(Hokey kernel - inspect source for fun patterns!)",
                line_count,
                char_count
            )
        );
    }

    Output {
        output_type: "execute_result".to_string(),
        data,
    }
}
