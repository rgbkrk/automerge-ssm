# Autodash

Real-time collaborative CRDT demo with React frontend and Rust CLI, syncing through Automerge.

## What This Demonstrates

**Automerge Data Types:**
- **Scalars**: numbers, booleans
- **Text**: CRDT text with character-level merging
- **Lists**: Arrays of objects and strings  
- **Maps**: Nested objects with timestamps

**Cross-Platform Sync:**
- React + TypeScript frontend
- Rust CLI client
- Standard automerge-repo WebSocket server

## Quick Start

```bash
# Terminal 1: Sync server
pnpx @automerge/automerge-repo-sync-server

# Terminal 2: Frontend
cd frontend && npm install && npm run dev

# Terminal 3: CLI (copy doc URL from browser)
cd cli
cargo run -- "http://localhost:5173/#automerge:YOUR_DOC_ID" show
```

Open http://localhost:5173 and interact via browser or CLI - changes sync instantly.

## CLI Commands

```bash
# Scalars
increment / decrement / set-counter <value>
set-temp <0-40>
toggle-dark / set-dark <true|false>

# Text
add-note <text>

# Lists
add-todo <text>
toggle-todo <id>    # Use first 8 chars of ID
delete-todo <id>
add-tag <tag>
remove-tag <tag>

# Metadata
set-title <title>

# View state
show  # Default command
```

## Architecture

```
React Browser  ──┐
                 ├──► WebSocket Sync Server ◄──── Rust CLI
React Browser  ──┘
```

**Frontend**: React 19 + TypeScript + automerge-repo + shadcn/ui  
**CLI**: Rust + autosurgeon + samod  
**Storage**: IndexedDB (browser), in-memory (CLI)

## Type-Safe Schema

```rust
#[derive(Reconcile, Hydrate)]
struct Doc {
    counter: i64,
    temperature: i64,
    darkMode: bool,
    notes: autosurgeon::Text,
    todos: Vec<TodoItem>,
    tags: Vec<autosurgeon::Text>,
    metadata: Metadata,
}
```

**Key Point**: Use `autosurgeon::Text` for string fields to ensure proper CRDT serialization between Rust and JavaScript.

## CRDT Benefits

**Counter**: Concurrent increments merge correctly - two users clicking "+1" = +2, not a conflict.

**Text**: Character-level merging - multiple users can type in different parts simultaneously without conflicts.

**Lists**: Operations merge by position and causality, not indices. Handles concurrent insertions gracefully.

**Maps**: Field-level merging - changes to different fields never conflict.

**Offline-First**: All changes preserved locally, synced automatically when reconnected.

## Development

```bash
# Frontend
cd frontend && npm install && npm run dev

# CLI  
cd cli && cargo run -- <url> show

# Run tests
npm test          # Frontend
cargo test        # CLI
```

## Resources

- [Automerge](https://automerge.org) - CRDT library
- [Automerge Repo](https://github.com/automerge/automerge-repo) - Sync framework
- [Autosurgeon](https://docs.rs/autosurgeon) - Rust derive macros
- [CRDT Tech](https://crdt.tech) - Learn more about CRDTs

## License

MIT