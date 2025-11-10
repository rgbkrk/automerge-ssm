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
pnpx @automerge/automerge-repo-sync-server

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

**CLI**: Rust + `samod` (Rust automerge-repo) + `autosurgeon`
- Connects via WebSocket to same sync server
- Uses autosurgeon for type-safe document handling
- Automatic serialization between Rust structs and Automerge documents

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
Uses autosurgeon for ergonomic document handling:
```rust
#[derive(Reconcile, Hydrate, Default)]
struct Doc {
    counter: Counter,        // CRDT counter type
    notes: String,           // Auto-handles Text compatibility
    collaborators: Vec<String>,
}

// Reading from document
let state: Doc = hydrate(doc).unwrap_or_default();

// Writing to document
state.counter.increment(1);
doc.transact(|tx| reconcile(tx, &state))?;
```

**Key benefit**: Autosurgeon automatically handles JS/Rust type compatibility (Text objects, lists, etc.) without manual type checking.

## Document Flow

1. User opens browser → creates document → gets URL with document ID
2. Document stored locally (IndexedDB) and synced to server
3. CLI connects with document ID → fetches from sync server
4. CLI makes changes → syncs back to server → appears in browser
5. All clients stay in sync through CRDT merge operations

## What You'll Learn

1. **Automerge basics**: Creating, reading, modifying CRDT documents
2. **Autosurgeon**: Type-safe document handling with derive macros
3. **Cross-platform sync**: Making JS and Rust play nicely together
4. **Type handling**: Automatic compatibility across platforms
5. **Sync protocols**: How automerge-repo coordinates clients

See [AUTOSURGEON_MIGRATION.md](./AUTOSURGEON_MIGRATION.md) for details on the type-safe approach.

## Known Issues

**Sync timing**: CLI sleeps 2 seconds after connecting to ensure document is fully synced. Proper solution would be listening for sync completion events from samod (TODO).

## Resources

- [Automerge](https://automerge.org) - CRDT library
- [Automerge Repo](https://github.com/automerge/automerge-repo) - JS implementation
- [Samod](https://docs.rs/samod) - Rust implementation
- [Sync Server](https://github.com/automerge/automerge-repo-sync-server) - Reference server

## License

MIT