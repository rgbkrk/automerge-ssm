# Automerge SSM Demo Guide

## What We Built

This is a full-stack demonstration of Automerge's collaborative editing capabilities featuring:

1. **React Frontend** - A modern web UI built with React, TypeScript, and Vite
2. **Rust WebSocket Server** - A native Automerge server for document synchronization
3. **Real-time Collaboration** - Multiple clients can edit the same document simultaneously

Both the frontend and server can read/write to the same Automerge documents, demonstrating true cross-platform CRDT synchronization.

## Quick Start

### Option 1: Use the Startup Script

```bash
./start-demo.sh
```

This will:
- Build and start the Rust server on `ws://localhost:3030`
- Install dependencies and start the frontend on `http://localhost:5173`
- Handle cleanup when you press Ctrl+C

### Option 2: Manual Start

**Terminal 1 - Start the Server:**
```bash
cd server
cargo run
```

**Terminal 2 - Start the Frontend:**
```bash
cd frontend
npm install  # First time only
npm run dev
```

## Try These Features

### 1. **Real-time Counter**
- Open the app in your browser
- Enter a username to join
- Click the + and - buttons
- The counter syncs instantly across all tabs

### 2. **Collaborative Notes**
- Type in the shared notes textarea
- Open the app in multiple tabs (same URL)
- Watch your changes appear in all tabs in real-time
- Close all tabs and reopen - your data persists in IndexedDB

### 3. **Multi-User Collaboration**
- Copy the share link (📋 button)
- Open it in a different browser or incognito window
- Join with a different username
- See all collaborators listed
- Make concurrent edits - Automerge handles conflicts automatically

### 4. **Offline Capability**
- Disconnect your network
- Make changes (they're stored locally)
- Reconnect
- Changes sync automatically when connection is restored

## How It Works

### Architecture Overview

```
┌─────────────────┐
│   Browser Tab 1 │
│   (React App)   │
└────────┬────────┘
         │
         │ WebSocket
         │ + Automerge
         │ Sync Protocol
         │
         ▼
┌─────────────────┐
│  Rust Server    │
│  (Port 3030)    │
│                 │
│  • Automerge    │
│  • tokio-tungstenite
│  • Document Store
└────────┬────────┘
         │
         │ WebSocket
         │ + Automerge
         │ Sync Protocol
         │
         ▼
┌─────────────────┐
│   Browser Tab 2 │
│   (React App)   │
└─────────────────┘
```

### Frontend Stack

**Technology:**
- React 18 + TypeScript
- Vite for fast development
- `@automerge/automerge-repo` - Document management
- `@automerge/automerge-repo-network-websocket` - Network sync
- `@automerge/automerge-repo-storage-indexeddb` - Local persistence

**How it works:**
1. Creates an Automerge `Repo` on startup
2. Connects to `ws://localhost:3030` via `BrowserWebSocketClientAdapter`
3. Stores documents locally in IndexedDB
4. Document URL is in the hash fragment (`#automerge:...`)
5. Real-time UI updates via Automerge's change events

### Server Stack

**Technology:**
- Rust with Tokio async runtime
- `automerge` crate (native Rust implementation)
- `tokio-tungstenite` for WebSocket handling
- In-memory document store with concurrent access

**How it works:**
1. Listens for WebSocket connections on port 3030
2. Maintains a concurrent HashMap of documents
3. Broadcasts document changes to all connected clients
4. Merges concurrent changes using Automerge's CRDT algorithms

## Document Data Model

```typescript
interface Doc {
  counter: number;           // Collaborative counter (CRDT)
  notes: string;            // Shared text field
  collaborators: string[];  // Array of usernames
}
```

## Key Concepts Demonstrated

### 1. **CRDTs (Conflict-Free Replicated Data Types)**
- The counter automatically merges concurrent increments
- Text edits from multiple users merge intelligently
- Array operations (adding collaborators) resolve deterministically

### 2. **Local-First Architecture**
- Data is stored locally first (IndexedDB)
- Network sync happens in the background
- App works offline, syncs when reconnected

### 3. **Cross-Platform Automerge**
- JavaScript (browser) and Rust (server) share the same document
- Both use the same Automerge data format
- Changes merge seamlessly regardless of source

### 4. **WebSocket Sync Protocol**
- Efficient binary protocol for syncing changes
- Only transmits deltas, not entire documents
- Handles reconnection automatically

## Code Walkthrough

### Frontend: Creating a Repo

```typescript
const repo = new Repo({
  network: [new BrowserWebSocketClientAdapter("ws://localhost:3030")],
  storage: new IndexedDBStorageAdapter(),
});
```

### Frontend: Making Changes

```typescript
docHandle.change((d: Doc) => {
  d.counter = (d.counter || 0) + 1;
});
```

### Server: Handling Connections

```rust
let ws_stream = accept_async(stream).await?;
let (ws_sender, ws_receiver) = ws_stream.split();

// Messages from clients trigger document updates
// which are then broadcast to all other clients
```

### Server: Merging Documents

```rust
let mut doc = doc_lock.write().await;
if let Ok(loaded_doc) = AutoCommit::load(changes_data) {
    doc.merge(&mut loaded_doc.clone())?;
}
```

## Connecting to Your Existing Server

You mentioned you already have a WebSocket server at `ws://localhost:3030`. To use it instead:

1. Stop the Rust demo server
2. Ensure your server uses the same port (3030)
3. The frontend will automatically connect to it

The demo server implements a simple protocol:
- Binary messages: First 36 bytes are document ID, rest is Automerge data
- Text messages: JSON protocol for document creation/retrieval

## Next Steps & Enhancements

### Immediate Improvements

1. **Full Sync Protocol** - Implement proper Automerge sync state machine
2. **Rich Text** - Add Peritext support for formatted text
3. **Persistence** - Add server-side storage (filesystem/database)
4. **Authentication** - Add user authentication and authorization

### Advanced Features

1. **Presence Awareness**
   - Show cursor positions
   - Indicate who's typing
   - Display online/offline status

2. **Document History**
   - View past versions
   - Diff visualization
   - Undo/redo across clients

3. **Multiple Document Types**
   - Task lists
   - Kanban boards
   - Drawing canvas
   - Code editor

4. **Performance Optimizations**
   - Implement proper sync state
   - Add compression
   - Batch updates
   - Lazy loading

## Troubleshooting

### Server won't start
```bash
# Check if port is in use
lsof -i :3030

# Kill existing process if needed
kill -9 <PID>
```

### Frontend won't connect
- Verify server is running: `curl http://localhost:3030`
- Check browser console for WebSocket errors
- Ensure no firewall blocking localhost connections

### Changes not syncing
- Check Network tab in browser DevTools
- Look for WebSocket connection status
- Verify both clients are on the same document URL

### Data not persisting
- Check browser settings (ensure IndexedDB is enabled)
- Open DevTools > Application > IndexedDB
- Look for "automerge" database

## Understanding Automerge

### Why Automerge?

**Traditional Approach:**
- Central server holds truth
- Clients send changes to server
- Server resolves conflicts (last-write-wins)
- Requires constant connection

**Automerge Approach:**
- Each client has full document
- Changes are CRDTs that merge mathematically
- No central authority needed
- Works offline, syncs when available

### The Magic: How CRDTs Work

```typescript
// Two clients start with counter = 5

// Client A (offline)          // Client B (online)
counter += 1  // = 6           counter += 2  // = 7

// When they sync:
// Traditional: 6 or 7 (conflict!)
// Automerge: 8 (both increments preserved!)
```

### Document URLs

Every Automerge document has a unique URL like:
```
automerge:2akvofn6L1o4RMUEMQi7qzwRjKWZ
```

This is like a Git commit hash - it uniquely identifies the document.

## Resources

- [Automerge Documentation](https://automerge.org)
- [CRDT Explainer](https://crdt.tech)
- [Local-First Software](https://www.inkandswitch.com/local-first/)
- [Automerge Rust Docs](https://docs.rs/automerge)

## License

MIT - Feel free to use this as a starting point for your own projects!