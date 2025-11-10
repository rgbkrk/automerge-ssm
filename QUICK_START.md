# Quick Start Guide

## Prerequisites

- Node.js 18+
- Rust (latest stable)

## 1. Start the Demo (Easy Way)

```bash
./start-demo.sh
```

This starts both the server and frontend automatically.

## 2. Start Manually

**Terminal 1 - Server:**
```bash
cd server
cargo run
```

**Terminal 2 - Frontend:**
```bash
cd frontend
npm install  # First time only
npm run dev
```

## 3. Try It Out

1. Open `http://localhost:5173` in your browser
2. Enter a username to join
3. A document URL will appear in the address bar (something like `#automerge:...`)
4. Copy that URL and open it in another tab/browser
5. Make changes in one tab, watch them sync to the other!

## What to Try

### Real-time Collaboration
- Click the counter buttons in multiple tabs
- Type in the shared notes simultaneously
- Watch changes sync instantly

### Offline Mode
- Disconnect your network
- Make changes (they're saved locally)
- Reconnect - changes sync automatically

### Share & Collaborate
- Click "📋 Copy Share Link"
- Send it to someone else
- Both of you can edit simultaneously

## Quick API Reference

### Frontend - Making Changes

```typescript
// Increment counter
docHandle.change((d) => {
  d.counter = (d.counter || 0) + 1;
});

// Update text
docHandle.change((d) => {
  d.notes = "New text";
});

// Add to array
docHandle.change((d) => {
  d.collaborators.push("Alice");
});
```

### Rust - Working with Documents

```rust
use automerge::{AutoCommit, ObjType};
use automerge::transaction::Transactable;

// Create document
let mut doc = AutoCommit::new();

// Make changes
doc.put(automerge::ROOT, "counter", 0_i64)?;
doc.put(automerge::ROOT, "notes", "Hello")?;

// Save to bytes
let bytes = doc.save();

// Load from bytes
let loaded = AutoCommit::load(&bytes)?;
```

## Development

### Type Checking

Run TypeScript type checking before committing:

```bash
cd frontend
npm run type-check
```

This catches type errors early and ensures code quality.

## Common Issues

### "ESM integration proposal for Wasm"
Already fixed! Just run `npm install` in the frontend directory.

### Port already in use
```bash
# Kill process on port 3030
lsof -i :3030
kill -9 <PID>
```

### Changes not syncing
- Ensure server is running (`cargo run` in server directory)
- Check browser console for errors
- Refresh the page

## Next Steps

- Read `DEMO.md` for in-depth explanations
- Check `server/examples/simple_client.rs` for Rust examples
- Explore the `frontend/src/App.tsx` to see how the UI works
- Run `npm run type-check` in frontend/ to validate TypeScript

## Key Concepts (30 Second Version)

**CRDTs**: Conflict-Free Replicated Data Types - they merge automatically, no conflicts!

**Local-First**: Your data lives on your device. Network is optional.

**Automerge**: Makes it all work together. Same document format in JavaScript and Rust.

**That's it!** You now have a working collaborative app. 🎉