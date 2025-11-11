use anyhow::{Context, Result};
use automerge::transaction::Transactable;

use automerge_cli::*;
use autosurgeon::{hydrate, reconcile};
use chrono::Utc;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::convert::Infallible;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tui_textarea::{Input, TextArea};

#[derive(Parser, Debug)]
#[command(author, version, about = "TUI collaborative notes editor", long_about = None)]
struct Cli {
    /// Document URL (e.g., automerge:... or http://localhost:5173/#automerge:...)
    #[arg(value_name = "URL")]
    doc_url: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

enum AppEvent {
    Input(Event),
    Tick,
}

struct App<'a> {
    textarea: TextArea<'a>,
    doc_handle: samod::DocHandle,
    status_message: String,
    last_known_text: String,
    should_quit: bool,
    unsaved_changes: bool,
}

impl<'a> App<'a> {
    fn new(doc_handle: samod::DocHandle) -> Result<Self> {
        // Load initial text from document
        let initial_text = doc_handle.with_document(|doc| -> Result<String> {
            let state: Doc = hydrate(doc)?;
            Ok(state.notes.clone())
        })?;

        let lines: Vec<String> = if initial_text.is_empty() {
            vec![String::new()]
        } else {
            initial_text.lines().map(|s| s.to_string()).collect()
        };

        let mut textarea = TextArea::new(lines);
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Notes (Ctrl+Q to quit, Ctrl+S to save)"),
        );
        textarea.set_cursor_line_style(Style::default());
        textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));

        Ok(Self {
            textarea,
            doc_handle,
            status_message: "Connected. Start editing!".to_string(),
            last_known_text: initial_text,
            should_quit: false,
            unsaved_changes: false,
        })
    }

    fn handle_input(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Char('q') => {
                            if self.unsaved_changes {
                                self.save_changes()?;
                            }
                            self.should_quit = true;
                            return Ok(());
                        }
                        KeyCode::Char('s') => {
                            self.save_changes()?;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                self.textarea.input(Input::from(event));
                self.unsaved_changes = true;
            }
            _ => {}
        }
        Ok(())
    }

    fn save_changes(&mut self) -> Result<()> {
        let current_text = self.textarea.lines().join("\n");

        if current_text == self.last_known_text {
            self.status_message = "No changes to save".to_string();
            self.unsaved_changes = false;
            return Ok(());
        }

        self.doc_handle.with_document(|doc| -> Result<()> {
            let mut state: Doc = hydrate(doc)?;

            // Replace entire text
            // TODO: Use character-level diff for better merging with automerge::splice
            state.notes = current_text.clone();
            state.metadata.lastModified = Some(Utc::now().timestamp_millis());

            doc.transact(|tx| reconcile(tx, &state))
                .map_err(|e| anyhow::anyhow!("Failed to reconcile: {:?}", e))?;
            Ok(())
        })?;

        self.last_known_text = current_text;
        self.unsaved_changes = false;
        self.status_message = format!("Saved at {}", chrono::Local::now().format("%H:%M:%S"));
        Ok(())
    }

    fn apply_remote_changes(&mut self) -> Result<()> {
        let remote_text = self.doc_handle.with_document(|doc| -> Result<String> {
            let state: Doc = hydrate(doc)?;
            Ok(state.notes.clone())
        })?;

        if remote_text != self.last_known_text {
            // Store cursor position
            let cursor = self.textarea.cursor();

            // Update textarea content
            let lines: Vec<String> = if remote_text.is_empty() {
                vec![String::new()]
            } else {
                remote_text.lines().map(|s| s.to_string()).collect()
            };

            self.textarea = TextArea::new(lines);
            self.textarea.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Notes (Ctrl+Q to quit, Ctrl+S to save)"),
            );
            self.textarea.set_cursor_line_style(Style::default());
            self.textarea
                .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));

            // Try to restore cursor position (clamped to valid range)
            let new_line_count = self.textarea.lines().len();
            if new_line_count > 0 {
                let new_row = cursor.0.min(new_line_count.saturating_sub(1));
                let new_col = if new_row < new_line_count {
                    cursor.1.min(self.textarea.lines()[new_row].len())
                } else {
                    0
                };

                // Move cursor to the clamped position
                for _ in 0..new_row {
                    self.textarea.move_cursor(tui_textarea::CursorMove::Down);
                }
                for _ in 0..new_col {
                    self.textarea.move_cursor(tui_textarea::CursorMove::Forward);
                }
            }

            self.last_known_text = remote_text;
            self.unsaved_changes = false;
            self.status_message =
                format!("Remote update at {}", chrono::Local::now().format("%H:%M:%S"));
        }
        Ok(())
    }
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(f.area());

    // Render textarea
    f.render_widget(app.textarea.widget(), chunks[0]);

    // Render status bar
    let status_indicator = if app.unsaved_changes { "*" } else { "" };
    let status_text = vec![Line::from(vec![
        Span::styled(
            format!("Status{}: ", status_indicator),
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(&app.status_message, Style::default().fg(Color::Green)),
    ])];

    let status = Paragraph::new(status_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Status")
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(status, chunks[1]);
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up tracing
    let filter = if cli.verbose {
        "samod=debug,automerge_cli=debug"
    } else {
        "samod=info,automerge_cli=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Parse document ID from URL
    let doc_id_str = if let Some(pos) = cli.doc_url.find("automerge:") {
        &cli.doc_url[pos + 10..]
    } else if let Some(pos) = cli.doc_url.find("#automerge:") {
        &cli.doc_url[pos + 11..]
    } else {
        anyhow::bail!(
            "URL must contain 'automerge:' or '#automerge:' - got: {}",
            cli.doc_url
        );
    };

    tracing::debug!("Initializing automerge-repo");

    // Create a repo with filesystem storage
    let repo = samod::Repo::build_tokio()
        .with_storage(samod::storage::TokioFilesystemStorage::new(
            "./autodash-data/",
        ))
        .load()
        .await;

    tracing::debug!("Connecting to sync server");

    // Connect to WebSocket server
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
    let _ws_to_samod_handle = tokio::spawn(ws_to_samod);
    let _samod_to_ws_handle = tokio::spawn(samod_to_ws);

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

    // Wait for sync to complete
    if doc_handle.is_none() {
        tracing::debug!("Document not immediately available, waiting for sync...");
        sleep(Duration::from_secs(2)).await;
        doc_handle = repo.find(doc_id.clone()).await?;
    } else {
        tracing::debug!("Document found, waiting for full sync...");
        sleep(Duration::from_secs(1)).await;
    }

    let doc_handle = doc_handle.context(
        "Document not found. Make sure:\n  1. The sync server is running\n  2. The document exists in the browser\n  3. The document ID is correct",
    )?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(doc_handle.clone())?;

    // Create event channel
    let (tx, mut rx) = mpsc::channel(100);

    // Spawn input handler
    let input_tx = tx.clone();
    tokio::spawn(async move {
        loop {
            if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(evt) = event::read() {
                    if input_tx.send(AppEvent::Input(evt)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Spawn periodic tick for auto-save and checking for remote changes
    let tick_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            interval.tick().await;
            if tick_tx.send(AppEvent::Tick).await.is_err() {
                break;
            }
        }
    });

    // Main event loop
    loop {
        terminal.draw(|f| ui(f, &app))?;

        // Use timeout to ensure we don't block forever
        match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Some(event)) => match event {
                AppEvent::Input(evt) => {
                    app.handle_input(evt)?;
                }
                AppEvent::Tick => {
                    // Check for remote changes and auto-save if needed
                    app.apply_remote_changes()?;
                    if app.unsaved_changes {
                        app.save_changes()?;
                    }
                }
            },
            Ok(None) => break, // Channel closed
            Err(_) => {}       // Timeout, continue
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
