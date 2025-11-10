# Automerge SSM Demo

A demonstration of real-time collaborative editing using Automerge with both a frontend web client and a Rust server component that can collaborate on the same document.

## Overview

This project showcases Automerge's capabilities for building local-first, multiplayer applications. It consists of:

- **Frontend**: A React + TypeScript web application using `@automerge/automerge-repo`
- **Server**: A Rust WebSocket server that can also read/write to Automerge documents
- Both components can collaborate on the same document in real-time

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

### 3. Build Server Dependencies

```bash
cd ../server
cargo build
```

## Running the Demo

You'll need to run both the server and the frontend:

### Terminal 1: Start the Rust Server

```bash
cd server
cargo run
```

The server will start on `ws://localhost:3030`

### Terminal 2: Start the Frontend

```bash
cd frontend
npm run dev
```

The frontend will start on `http://localhost:5173` (or similar)

### Terminal 3 (Optional): Use the Rust CLI Client

```bash
# Get the document ID from your browser URL
# Example: http://localhost:5173/#automerge:2mdM9TnM2sJgLhHhYjyBzfusSsyr
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

# Increment the counter from Rust
./rust-client.sh 2mdM9TnM2sJgLhHhYjyBzfusSsyr increment

# Add a note from Rust
./rust-client.sh 2mdM9TnM2sJgLhHhYjyBzfusSsyr add-note "Hello from Rust!"
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

### Server (`/server`)

- **Technology**: Rust with Tokio async runtime
- **Automerge Integration**: Uses native `automerge` crate
- **Features**:
  - WebSocket server for client connections
  - Document storage and synchronization
  - Broadcasts changes to all connected clients
  - Can read/write Automerge documents directly

### Communication Flow

```
┌─────────────┐         WebSocket          ┌─────────────┐
│   Browser   │ ◄────────────────────────► │    Rust     │
│  (Client 1) │      Automerge Sync        │   Server    │
└─────────────┘                             └─────────────┘
                                                   ▲
                                                   │
                                            WebSocket
                                                   │
                                                   ▼
                                            ┌─────────────┐
                                            │   Browser   │
                                            │  (Client 2) │
                                            └─────────────┘
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

## Connecting to Your Own Sync Server

The frontend is configured to connect to `ws://localhost:3030`. You already have a WebSocket server running at this address, so the demo will use both:

1. Your existing sync server at `ws://localhost:3030`
2. This Rust demo server (you'll need to change the port if running both)

To change the server URL, edit `frontend/src/App.tsx`:
### Frontend: Creating a Repo

```typescript
const repo = new Repo({
  network: [new BrowserWebSocketClientAdapter("ws://localhost:3030")],
  storage: new IndexedDBStorageAdapter(),
});
```

**Note:** Automerge uses WebAssembly. The Vite config includes `vite-plugin-wasm` and `vite-plugin-top-level-await` to handle WASM module loading automatically.

## Development Notes

### Rust Server Implementation

The Rust server (`server/src/main.rs`) demonstrates:
- WebSocket connection handling with `tokio-tungstenite`
- Automerge document management
- Change broadcasting to connected clients
- Basic sync protocol implementation

### Future Enhancements

- [ ] Implement full Automerge sync protocol
- [ ] Add authentication/authorization
- [ ] Persistent server-side storage
- [ ] Rich text editing with Peritext
- [ ] Presence awareness (cursor positions)
- [ ] Document history/time-travel
- [ ] Multiple document types

## Troubleshooting

### Server won't start

- Make sure port 3030 is not already in use
- Check that Rust dependencies compiled successfully: `cargo check`

### Frontend won't connect

- Verify the server is running
- Check browser console for WebSocket errors
- Ensure the WebSocket URL is correct

### Changes not syncing

- Check that both server and frontend are running
- Look for errors in server logs and browser console
- Try refreshing the page

## Rust CLI Client

Want to modify documents from Rust? Use the CLI client:

```bash
./rust-client.sh <document-id> increment
./rust-client.sh <document-id> add-note "From Rust!"
```

This demonstrates **true cross-platform collaboration** - changes from the Rust CLI appear instantly in all browser tabs!

📖 **Full guide:** [RUST_CLIENT.md](RUST_CLIENT.md)

## Resources

- [Automerge Documentation](https://automerge.org)
- [Automerge Repo](https://github.com/automerge/automerge-repo)
- [Automerge Rust](https://docs.rs/automerge)

## License

MIT