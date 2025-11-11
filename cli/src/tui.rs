use anyhow::{Context, Result};
use automerge::transaction::Transactable;
use automerge::ReadDoc;
use automerge_cli::*;
use autosurgeon::{hydrate, reconcile};
use chrono::Utc;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use tokio::sync::mpsc;
use tui_textarea::{Input, TextArea};

#[derive(Parser, Debug)]
#[command(author, version, about = "TUI collaborative notes editor", long_about = None)]
struct Cli {
    /// Document URL (e.g., automerge:...)
    #[arg(value_name = "DOC_URL")]
    doc_url: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

enum AppEvent {
    Input(Event),
    DocumentChanged,
    Quit,
}

struct App<'a> {
    textarea: TextArea<'a>,
    doc_handle: samod::DocHandle,
    status_message: String,
    last_known_text: String,
    should_quit: bool,
}

impl<'a> App<'a> {
    fn new(doc_handle: samod::DocHandle) -> Result<Self> {
        let mut textarea = TextArea::default();
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Notes (Ctrl+Q to quit, Ctrl+S to save)"),
        );
        textarea.set_cursor_line_style(Style::default());
        textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));

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
        textarea = TextArea::new(lines);
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
        })
    }

    fn handle_input(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Char('q') => {
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
            }
            _ => {}
        }
        Ok(())
    }

    fn save_changes(&mut self) -> Result<()> {
        let current_text = self.textarea.lines().join("\n");

        if current_text == self.last_known_text {
            self.status_message = "No changes to save".to_string();
            return Ok(());
        }

        self.doc_handle.with_document(|doc| -> Result<()> {
            let mut state: Doc = hydrate(doc)?;

            // Calculate the diff and apply minimal changes
            let old_text = &self.last_known_text;
            let new_text = &current_text;

            // For now, simple approach: replace entire text
            // TODO: Use character-level diff for better merging
            state.notes = new_text.clone();
            state.metadata.lastModified = Some(Utc::now().timestamp_millis());

            doc.transact(|tx| reconcile(tx, &state))
                .map_err(|e| anyhow::anyhow!("Failed to reconcile: {:?}", e))?;
            Ok(())
        })?;

        self.last_known_text = current_text;
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
            self.textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));

            // Try to restore cursor position (clamped to valid range)
            let new_line_count = self.textarea.lines().len();
            let new_row = cursor.0.min(new_line_count.saturating_sub(1));
            let new_col = if new_row < new_line_count {
                cursor.1.min(self.textarea.lines()[new_row].len())
            } else {
                0
            };
            self.textarea.move_cursor(ratatui::crossterm::cursor::MoveTo(new_col as u16, new_row as u16));

            self.last_known_text = remote_text;
            self.status_message = format!("Remote update at {}", chrono::Local::now().format("%H:%M:%S"));
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
    let status_text = vec![Line::from(vec![
        Span::styled(
            "Status: ",
            Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
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

    // Connect to sync server
    let sync_url = url::Url::parse("ws://127.0.0.1:3030")?;
    let mut connection = samod::connect(sync_url).await?;

    // Parse document URL
    let doc_url: samod::DocumentId = cli
        .doc_url
        .parse()
        .context("Invalid document URL format")?;

    // Get document handle
    let doc_handle = connection
        .document(doc_url)
        .await
        .context("Failed to get document handle")?;

    // Wait a moment for initial sync
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

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
            if event::poll(std::time::Duration::from_millis(100)).unwrap() {
                if let Ok(evt) = event::read() {
                    if input_tx.send(AppEvent::Input(evt)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Spawn document change listener
    let doc_handle_clone = doc_handle.clone();
    let doc_tx = tx.clone();
    tokio::spawn(async move {
        let mut listener = doc_handle_clone.listener();
        loop {
            listener.changed().await.ok();
            if doc_tx.send(AppEvent::DocumentChanged).await.is_err() {
                break;
            }
        }
    });

    // Main event loop
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Ok(event) = rx.try_recv() {
            match event {
                AppEvent::Input(evt) => {
                    app.handle_input(evt)?;
                }
                AppEvent::DocumentChanged => {
                    app.apply_remote_changes()?;
                }
                AppEvent::Quit => break,
            }
        }

        if app.should_quit {
            break;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
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
