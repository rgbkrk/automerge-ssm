# Automerge SSM Demo

A demonstration of real-time collaborative editing using Automerge with a frontend web client and a Rust CLI client that both connect to a sync server and can collaborate on the same document.

## Overview

This project showcases Automerge's capabilities for building local-first, multiplayer applications. It consists of:

- **Frontend**: A React + TypeScript web application using `@automerge/automerge-repo`
- **CLI**: A Rust command-line client using `samod` (Rust automerge-repo)
- Both connect to a sync server at `localhost:3030` and collaborate in real-time

## Features

- ✨ Real-time collaborative counter
- 📝 Shared notes with live updates
- 👥 Active collaborators list
- 🔄 Automatic conflict resolution via CRDT
- 💾 Persistent storage (IndexedDB on frontend)
- 🌐 WebSocket synchronization
- 🦀 Rust backend integration
- 🔧 Rust CLI client for server-side document manipulation

## 🚀 Quick Start

**Want to jump right in?** See [QUICK_START.md](QUICK_START.md) for a one-page guide to get up and running in minutes.

For detailed explanations and architecture info, continue reading below or check out [DEMO.md](DEMO.md).

## Prerequisites

- Node.js (v18+)
- Rust (latest stable)
- Cargo
- A sync server running on `ws://localhost:3030` (e.g., [automerge-repo-sync-server](https://github.com/automerge/automerge-repo-sync-server))

## Setup

### 1. Clone the Repository

```bash
git clone <your-repo-url>
cd automerge-ssm
```

### 2. Install Frontend Dependencies

```bash
cd frontend
npm install
```

**Note:** The project includes `vite-plugin-wasm` and `vite-plugin-top-level-await` to handle Automerge's WebAssembly module loading. These are already configured in `vite.config.ts`.

### 3. Build CLI

```bash
cd ../server
cargo build --release
```

## Running the Demo

You'll need to run a sync server and the frontend:

### Terminal 1: Start a Sync Server

You need an automerge-repo sync server running on port 3030. For example:

```bash
npx @automerge/automerge-repo-sync-server
```

### Terminal 2: Start the Frontend

```bash
cd frontend
npm run dev
```

The frontend will start on `http://localhost:5173` (or similar)

### Terminal 3 (Optional): Use the Rust CLI Client

```bash
# Get the document URL from your browser
# Example: http://localhost:5173/#automerge:2mdM9TnM2sJgLhHhYjyBzfusSsyr

# Increment the counter from Rust
cargo run --release --bin automerge-cli -- automerge:2mdM9TnM2sJgLhHhYjyBzfusSsyr increment

# Add a note from Rust
cargo run --release --bin automerge-cli -- automerge:2mdM9TnM2sJgLhHhYjyBzfusSsyr add-note "Hello from Rust!"
```

**See [RUST_CLIENT.md](RUST_CLIENT.md) for full CLI documentation.**

## Usage

1. Open the frontend URL in your browser
2. Enter a username to join the collaboration
3. A new document will be created (or loaded if you have a URL with a document ID)
4. Try these features:
   - Click the counter buttons to increment/decrement
   - Type in the shared notes - changes sync in real-time
   - Copy the share link and open it in another tab or browser
   - Watch as changes sync between all connected clients
   - **Use the Rust CLI client to modify documents from the command line!**

## Architecture

### Frontend (`/frontend`)

- **Technology**: React + TypeScript + Vite
- **Automerge Integration**: Uses `@automerge/automerge-repo` with:
  - `BrowserWebSocketClientAdapter` for network sync
  - `IndexedDBStorageAdapter` for local persistence
- **Features**:
  - Real-time UI updates
  - Automatic reconnection
  - Local-first architecture (works offline)

### CLI (`/server`)

- **Technology**: Rust with Tokio async runtime
- **Automerge Integration**: Uses `samod` (Rust implementation of automerge-repo)
- **Features**:
  - Full automerge-repo protocol compatibility
  - Connects to any automerge-repo sync server
  - Can read/write Automerge documents
  - Command-line interface for document manipulation
</parameter>

### Communication Flow

```
┌─────────────┐         WebSocket          ┌─────────────┐
│   Browser   │ ◄────────────────────────► │    Sync     │
│  (Client 1) │      Automerge Sync        │   Server    │
└─────────────┘                             │ :3030       │
                                            └─────────────┘
                                                   ▲
                                                   │
                                            WebSocket
                                         (Automerge Sync)
                                                   │
                                                   ├────────► Browser (Client 2)
                                                   │
                                                   └────────► Rust CLI
```

## Document Structure

The demo document has the following structure:

```typescript
interface Doc {
  counter: number;           // Shared counter value
  notes: string;            // Collaborative text notes
  collaborators: string[];  // List of active users
}
```

## Sync Server

Both the frontend and CLI connect to `ws://localhost:3030`. You need a sync server running at this address. You can use the official automerge-repo sync server:

```bash
npx @automerge/automerge-repo-sync-server
```

To change the server URL, edit `frontend/src/App.tsx` and `server/src/bin/repo_client.rs`:
### Frontend: Creating a Repo

```typescript
const repo = new Repo({
  network: [new BrowserWebSocketClientAdapter("ws://localhost:3030")],
  storage: new IndexedDBStorageAdapter(),
});
```

**Note:** Automerge uses WebAssembly. The Vite config includes `vite-plugin-wasm` and `vite-plugin-top-level-await` to handle WASM module loading automatically.

## Development Notes

### Rust CLI Implementation

The Rust CLI (`server/src/bin/repo_client.rs`) demonstrates:
- Using `samod` (Rust automerge-repo) for full protocol compatibility
- WebSocket connection to sync servers
- Document creation and manipulation
- Real-time synchronization with browser clients

### Future Enhancements

- [x] Implement full Automerge sync protocol (via samod)
- [ ] Add authentication/authorization to sync server
- [ ] Add persistent storage adapter
- [ ] Rich text editing with Peritext
- [ ] Presence awareness (cursor positions)
- [ ] Document history/time-travel
- [ ] Multiple document types
- [ ] Interactive TUI mode for CLI

## Troubleshooting

### Sync server won't start

- Make sure port 3030 is not already in use
- Try using a different port and update both frontend and CLI accordingly

### Frontend won't connect

- Verify the server is running
- Check browser console for WebSocket errors
- Ensure the WebSocket URL is correct

### Changes not syncing

- Check that both server and frontend are running
- Look for errors in server logs and browser console
- Try refreshing the page

## Rust CLI Client

Want to modify documents from Rust? Use the CLI client built with `samod`:

```bash
cargo run --release --bin automerge-cli -- automerge:<document-id> increment
cargo run --release --bin automerge-cli -- automerge:<document-id> add-note "From Rust!"
```

This demonstrates **true cross-platform collaboration** using the automerge-repo protocol - changes from the Rust CLI appear instantly in all browser tabs!

📖 **Full guide:** [RUST_CLIENT.md](RUST_CLIENT.md)

## Resources

- [Automerge Documentation](https://automerge.org)
- [Automerge Repo](https://github.com/automerge/automerge-repo)
- [Automerge Rust](https://docs.rs/automerge)
- [Samod (Rust automerge-repo)](https://docs.rs/samod)
- [Automerge Repo Sync Server](https://github.com/automerge/automerge-repo-sync-server)

## License

MIT