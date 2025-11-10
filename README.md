# Automerge Cross-Platform Demo

A minimal example of real-time collaborative editing using Automerge, demonstrating how JavaScript and Rust clients can collaborate on the same document through a sync server.

## What This Demonstrates

- **Cross-platform CRDTs**: Browser (JS) and CLI (Rust) editing the same document
- **Real-time sync**: Changes appear instantly across all clients
- **Local-first**: Works offline, syncs when reconnected
- **Protocol compatibility**: `@automerge/automerge-repo` (JS) ↔ `samod` (Rust)

## Quick Start

```bash
# Terminal 1: Sync server
npx @automerge/automerge-repo-sync-server

# Terminal 2: Frontend
cd frontend && npm install && npm run dev

# Terminal 3: CLI (grab document ID from browser URL)
cd cli
cargo run --release --bin automerge-cli -- automerge:YOUR_DOC_ID increment
```

Open http://localhost:5173 in your browser, then use the CLI to modify the same document.

## CLI Commands

```bash
# View current state
cargo run --release --bin automerge-cli -- automerge:DOCID

# Modify document
cargo run --release --bin automerge-cli -- automerge:DOCID increment
cargo run --release --bin automerge-cli -- automerge:DOCID add-note "Hello from Rust"
cargo run --release --bin automerge-cli -- automerge:DOCID add-user "BotName"

# Debug mode
cargo run --release --bin automerge-cli -- --verbose automerge:DOCID
```

## Architecture

```
Browser (JS)  ──┐
                ├──► Sync Server (:3030) ◄──── CLI (Rust)
Browser (JS)  ──┘
```

**Frontend**: React + TypeScript + `@automerge/automerge-repo`
- Uses WebSocket adapter for sync
- IndexedDB for local persistence
- Standard automerge-repo setup

**CLI**: Rust + `samod` (Rust automerge-repo)
- Connects via WebSocket to same sync server
- Reads/writes using Automerge API directly
- Handles Text objects from JS (important for compatibility!)

**Sync Server**: Standard automerge-repo sync server
- No customization needed
- Just handles message routing

## Key Implementation Details

### JavaScript Side (Frontend)
Creates Automerge documents with simple structure:
```typescript
interface Doc {
  counter: number;
  notes: string;           // Stored as Automerge Text
  collaborators: string[]; // Array of Automerge Text objects
}
```

### Rust Side (CLI)
Must handle JavaScript's Text objects:
```rust
// Reading Text objects from JS
match doc.get(ROOT, "notes") {
    Ok(Some((Value::Object(ObjType::Text), obj_id))) => {
        doc.text(&obj_id)? // Extract text
    }
    // ...
}

// Writing Text objects for JS compatibility
let text_obj = tx.put_object(ROOT, "notes", ObjType::Text)?;
tx.splice_text(&text_obj, 0, 0, "content")?;
```

**Critical**: JS stores strings as Text objects. Rust must handle both Text and Scalar types when reading, and write Text for JS compatibility.

## Document Flow

1. User opens browser → creates document → gets URL with document ID
2. Document stored locally (IndexedDB) and synced to server
3. CLI connects with document ID → fetches from sync server
4. CLI makes changes → syncs back to server → appears in browser
5. All clients stay in sync through CRDT merge operations

## What You'll Learn

1. **Automerge basics**: Creating, reading, modifying CRDT documents
2. **Cross-platform sync**: Making JS and Rust play nicely together
3. **Type handling**: Text vs Scalar types across platforms
4. **Sync protocols**: How automerge-repo coordinates clients
5. **Error handling**: Proper validation vs silent failures

## Known Issues

**Sync timing**: CLI sleeps 2 seconds after connecting to ensure document is fully synced. Proper solution would be listening for sync completion events from samod (TODO).

## Resources

- [Automerge](https://automerge.org) - CRDT library
- [Automerge Repo](https://github.com/automerge/automerge-repo) - JS implementation
- [Samod](https://docs.rs/samod) - Rust implementation
- [Sync Server](https://github.com/automerge/automerge-repo-sync-server) - Reference server

## License

MIT