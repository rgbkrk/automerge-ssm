//! CLI client using samod (automerge-repo for Rust)
//!
//! This properly integrates with automerge-repo's sync protocol,
//! so it can collaborate with the JavaScript frontend!
//!
//! Usage:
//!   cargo run --bin repo_client -- <automerge-url> [command]
//!
//! Commands:
//!   increment        - Increment the counter by 1
//!   decrement        - Decrement the counter by 1
//!   set-counter <n>  - Set counter to specific value
//!   add-note <text>  - Add text to notes
//!   add-user <name>  - Add a collaborator
//!   show             - Just display the current document state

use anyhow::{Context, Result};
use automerge::{transaction::Transactable, ObjType, ReadDoc};
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

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
                let text = args.get(1).context("add-note requires text")?.clone();
                Ok(Command::AddNote(text))
            }
            "add-user" => {
                let name = args.get(1).context("add-user requires a name")?.clone();
                Ok(Command::AddUser(name))
            }
            "show" => Ok(Command::Show),
            _ => anyhow::bail!("Unknown command: {}", args[0]),
        }
    }
}

fn get_counter(doc: &automerge::Automerge) -> i64 {
    match doc.get(automerge::ROOT, "counter") {
        Ok(Some((automerge::Value::Scalar(s), _))) => s.to_i64().unwrap_or(0),
        _ => 0,
    }
}

fn get_notes(doc: &automerge::Automerge) -> String {
    match doc.get(automerge::ROOT, "notes") {
        Ok(Some((automerge::Value::Scalar(s), _))) => {
            s.to_str().map(|s| s.to_string()).unwrap_or_default()
        }
        _ => String::new(),
    }
}

fn get_collaborators(doc: &automerge::Automerge) -> Vec<String> {
    match doc.get(automerge::ROOT, "collaborators") {
        Ok(Some((automerge::Value::Object(ObjType::List), id))) => {
            let len = doc.length(&id);
            (0..len)
                .filter_map(|i| {
                    doc.get(&id, i).ok().and_then(|opt| {
                        opt.and_then(|(val, _)| match val {
                            automerge::Value::Scalar(s) => s.to_str().map(|s| s.to_string()),
                            _ => None,
                        })
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn display_document(doc: &automerge::Automerge) {
    println!("\n📄 Document State:");
    println!("  Counter: {}", get_counter(doc));
    let notes = get_notes(doc);
    if notes.is_empty() {
        println!("  Notes: (empty)");
    } else {
        println!("  Notes: {}", notes);
    }
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

async fn execute_command(
    doc_handle: &samod::DocHandle,
    command: &Command,
) -> Result<()> {
    match command {
        Command::Increment => {
            doc_handle.with_document(|doc| {
                let current = get_counter(doc);
                doc.transact(|tx| {
                    tx.put(automerge::ROOT, "counter", current + 1)?;
                    info!("✅ Incremented counter: {} → {}", current, current + 1);
                    Ok::<_, automerge::AutomergeError>(())
                }).unwrap()
            });
        }
        Command::Decrement => {
            doc_handle.with_document(|doc| {
                let current = get_counter(doc);
                doc.transact(|tx| {
                    tx.put(automerge::ROOT, "counter", current - 1)?;
                    info!("✅ Decremented counter: {} → {}", current, current - 1);
                    Ok::<_, automerge::AutomergeError>(())
                }).unwrap()
            });
        }
        Command::SetCounter(value) => {
            doc_handle.with_document(|doc| {
                let current = get_counter(doc);
                doc.transact(|tx| {
                    tx.put(automerge::ROOT, "counter", *value)?;
                    info!("✅ Set counter: {} → {}", current, value);
                    Ok::<_, automerge::AutomergeError>(())
                }).unwrap()
            });
        }
        Command::AddNote(text) => {
            doc_handle.with_document(|doc| {
                let current_notes = get_notes(doc);
                let new_notes = if current_notes.is_empty() {
                    text.clone()
                } else {
                    format!("{}\n{}", current_notes, text)
                };
                doc.transact(|tx| {
                    tx.put(automerge::ROOT, "notes", new_notes.as_str())?;
                    info!("✅ Added note: {}", text);
                    Ok::<_, automerge::AutomergeError>(())
                }).unwrap()
            });
        }
        Command::AddUser(name) => {
            doc_handle.with_document(|doc| {
                let existing_users = get_collaborators(doc);
                doc.transact(|tx| {
                    // Get or create collaborators list
                    let collaborators = match tx.get(automerge::ROOT, "collaborators")? {
                        Some((automerge::Value::Object(ObjType::List), id)) => id,
                        _ => tx.put_object(automerge::ROOT, "collaborators", ObjType::List)?,
                    };

                    // Check if user already exists
                    if existing_users.contains(name) {
                        info!("⚠️  User '{}' already in collaborators list", name);
                    } else {
                        let len = tx.length(&collaborators);
                        tx.insert(&collaborators, len, name.as_str())?;
                        info!("✅ Added collaborator: {}", name);
                    }
                    Ok::<_, automerge::AutomergeError>(())
                }).unwrap()
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
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <automerge-url> [command] [args...]", args[0]);
        eprintln!();
        eprintln!("The automerge-url should be the full URL from your browser:");
        eprintln!("  automerge:4VgLSsiuVNfWeZk17m85GgA18VVp");
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
        eprintln!(
            "  {} automerge:4VgLSsiuVNfWeZk17m85GgA18VVp increment",
            args[0]
        );
        std::process::exit(1);
    }

    let doc_url = &args[1];
    let command = Command::from_args(&args[2..])?;

    // Parse the automerge URL
    if !doc_url.starts_with("automerge:") {
        anyhow::bail!("URL must start with 'automerge:' - got: {}", doc_url);
    }

    let doc_id_str = doc_url.strip_prefix("automerge:").unwrap();

    info!("🦀 Initializing automerge-repo...");

    // Create a repo with WebSocket sync to your existing server
    let repo = samod::Repo::build_tokio()
        .with_storage(samod::storage::InMemoryStorage::new())
        .load()
        .await;

    info!("📡 Connecting to sync server at ws://localhost:3030...");

    // Connect to websocket sync server using samod's built-in method
    tokio::spawn(repo.connect_websocket(
        "ws://localhost:3030".parse()?,
        samod::ConnDirection::Outgoing,
    ));

    info!("✅ Connection initiated");

    // Give connection time to establish
    sleep(Duration::from_millis(500)).await;

    info!("📥 Loading document: automerge:{}", doc_id_str);

    // Create DocumentId from string
    let doc_id: samod::DocumentId = doc_id_str.parse()?;

    // Try to find the document
    let doc_handle = match repo.find(doc_id.clone()).await? {
        Some(handle) => {
            info!("✅ Document found!");
            handle
        }
        None => {
            info!("📝 Document not found, creating new one...");
            // Create a new document with initial structure
            let mut initial_doc = automerge::Automerge::new();
            initial_doc.transact::<_, _, automerge::AutomergeError>(|tx| {
                tx.put(automerge::ROOT, "counter", 0_i64)?;
                tx.put(automerge::ROOT, "notes", "")?;
                tx.put_object(automerge::ROOT, "collaborators", ObjType::List)?;
                Ok(())
            }).unwrap();
            repo.create(initial_doc).await?
        }
    };

    // Wait for sync
    sleep(Duration::from_secs(1)).await;

    // Display state before changes
    doc_handle.with_document(|doc| {
        info!("📄 Current document state:");
        display_document(doc);
    });

    // Execute the command
    if !matches!(command, Command::Show) {
        info!("🔧 Executing command...");
        execute_command(&doc_handle, &command).await?;

        // Give changes time to sync
        sleep(Duration::from_millis(500)).await;

        // Display final state
        doc_handle.with_document(|doc| {
            info!("📄 After changes:");
            display_document(doc);
        });
    }

    info!("✨ Done! Changes should appear in your browser now.");
    info!("💡 Check your browser to see the updates!");

    // Keep alive for a bit to ensure changes are fully synced
    sleep(Duration::from_secs(2)).await;

    Ok(())
}
