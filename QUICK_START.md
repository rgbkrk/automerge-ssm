# Quick Start Guide

Get the Automerge collaborative demo running in 5 minutes!

## Prerequisites

- Node.js 18+
- Rust (latest stable)
- A sync server on `localhost:3030`

## Step 1: Start a Sync Server

You need an automerge-repo sync server. The easiest way:

```bash
npx @automerge/automerge-repo-sync-server
```

This will start on `ws://localhost:3030`

## Step 2: Start the Frontend

```bash
cd frontend
npm install
npm run dev
```

Open http://localhost:5173 in your browser.

## Step 3: Try the Demo

In your browser:
1. Enter a username (e.g., "WebUser")
2. Click the counter buttons
3. Type in the notes field
4. Copy the URL - it contains your document ID!

Example URL:
```
http://localhost:5173/#automerge:2mdM9TnM2sJgLhHhYjyBzfusSsyr
```

## Step 4: Use the Rust CLI

Now the magic part - modify the same document from Rust!

```bash
# In a new terminal
cd server

# Build the CLI (first time only)
cargo build --release

# Use your document URL from the browser
cargo run --release --bin automerge-cli -- \
  automerge:2mdM9TnM2sJgLhHhYjyBzfusSsyr increment
```

**Watch your browser** - the counter increments instantly! 🎉

### More CLI Commands

```bash
# Add a note
cargo run --release --bin automerge-cli -- \
  automerge:YOUR_DOC_ID add-note "Hello from Rust!"

# Add a collaborator
cargo run --release --bin automerge-cli -- \
  automerge:YOUR_DOC_ID add-user "RustUser"

# Set counter to specific value
cargo run --release --bin automerge-cli -- \
  automerge:YOUR_DOC_ID set-counter 42

# Just view the document
cargo run --release --bin automerge-cli -- \
  automerge:YOUR_DOC_ID show
```

## What's Happening?

```
┌─────────┐                     ┌─────────┐                     ┌─────────┐
│ Browser │ ◄── WebSocket  ───► │  Sync   │ ◄── WebSocket  ───► │ Rust    │
│         │  Automerge Sync     │ Server  │  Automerge Sync     │  CLI    │
└─────────┘                     └─────────┘                     └─────────┘
```

Both the browser and Rust CLI:
- Connect to the same sync server
- Use the automerge-repo protocol
- See each other's changes in real-time
- Can work offline and sync later

## Try Multi-User

1. Open the document URL in multiple browser tabs
2. Run the CLI in multiple terminals
3. Watch changes sync across all clients instantly!

## Troubleshooting

**Port 3030 in use?**
- Kill existing process: `lsof -i :3030` then `kill <PID>`
- Or use a different port (update frontend/src/App.tsx and server/src/bin/repo_client.rs)

**Sync not working?**
- Check sync server is running on port 3030
- Check browser console and terminal logs
- Make sure you're using the full automerge URL (with `automerge:` prefix)

**Build errors?**
- Update Rust: `rustup update`
- Clean build: `cd server && cargo clean && cargo build --release`

## Next Steps

- Read [README.md](README.md) for architecture details
- See [RUST_CLIENT.md](RUST_CLIENT.md) for CLI documentation
- Check [DEMO.md](DEMO.md) for detailed walkthrough

## Tips

- Use `--release` for faster CLI execution
- The CLI connects, makes changes, and disconnects - perfect for automation
- Changes persist in browser's IndexedDB
- Create a shell alias for the CLI:
  ```bash
  alias am='cargo run --release --bin automerge-cli --manifest-path=/path/to/server/Cargo.toml --'
  am automerge:YOUR_DOC_ID increment
  ```

Enjoy collaborating across platforms! 🦀 + 🌐 = 🚀