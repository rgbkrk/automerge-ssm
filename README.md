# Autodash

Comprehensive Automerge CRDT demonstration showcasing real-time collaboration between React (TypeScript) and Rust clients.

## What This Demonstrates

**All Automerge Data Types:**
- **Scalars**: `number`, `boolean`, `string`
- **Text**: CRDT text with character-level merging
- **Lists**: Arrays of primitives and objects
- **Maps**: Nested object structures
- **Timestamps**: Metadata tracking

**Cross-Platform:**
- Browser: React + TypeScript + shadcn/ui
- CLI: Rust + autosurgeon + samod
- Sync: Standard automerge-repo WebSocket server

## Quick Start

```bash
# Terminal 1: Sync server
pnpx @automerge/automerge-repo-sync-server

# Terminal 2: Frontend
cd frontend && npm install && npm run dev

# Terminal 3: CLI
cd cli
cargo run --release --bin automerge-cli -- "http://localhost:5173/#automerge:YOUR_DOC_ID" show
```

Open http://localhost:5173, then use the CLI with the URL from your browser.

## Data Types Demo

### Scalar Types

**Counter** (`number`)
```bash
cargo run --release --bin automerge-cli -- <url> increment
cargo run --release --bin automerge-cli -- <url> decrement
cargo run --release --bin automerge-cli -- <url> set-counter 42
```

**Temperature Slider** (`number`)
```bash
cargo run --release --bin automerge-cli -- <url> set-temp 25
```

**Dark Mode** (`boolean`)
```bash
cargo run --release --bin automerge-cli -- <url> toggle-dark
cargo run --release --bin automerge-cli -- <url> set-dark true
```

### Text Type (CRDT)

**Collaborative Notes** (`string` as Automerge Text)
```bash
cargo run --release --bin automerge-cli -- <url> add-note "Hello from CLI"
```

Character-level CRDT merging prevents conflicts when multiple users type simultaneously.

### List Types

**Todo List** (`Array<Object>`)
```bash
# Add todo
cargo run --release --bin automerge-cli -- <url> add-todo "Implement feature"

# Toggle completion (use first 8 chars of ID from show command)
cargo run --release --bin automerge-cli -- <url> toggle-todo 12345678

# Delete todo
cargo run --release --bin automerge-cli -- <url> delete-todo 12345678
```

**Tags** (`Array<string>`)
```bash
cargo run --release --bin automerge-cli -- <url> add-tag "crdt"
cargo run --release --bin automerge-cli -- <url> remove-tag "crdt"
```



### Nested Objects

**Metadata** (`Object` with timestamps and title)
- `createdAt`: Timestamp
- `lastModified`: Timestamp  
- `title`: String

Automatically synced across all clients.

## Architecture

```
Browser (React)  ──┐
                   ├──► Sync Server (:3030) ◄──── CLI (Rust)
Browser (React)  ──┘
```

### Frontend
- **Framework**: React 19 + TypeScript + Vite
- **UI**: shadcn/ui (Tailwind CSS)
- **Automerge**: `@automerge/automerge-repo` v2.4
- **Storage**: IndexedDB for persistence
- **Sync**: WebSocket adapter

### CLI
- **Language**: Rust
- **Automerge**: `autosurgeon` v0.9 (derive macros)
- **Repo**: `samod` v0.5 (Rust automerge-repo)
- **Type Safety**: Compile-time schema validation

### Sync Server
- Standard automerge-repo WebSocket server
- No customization needed
- Handles message routing and sync protocol

## Type-Safe Rust Schema

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

**Benefits:**
- Compile-time schema validation
- Automatic JS/Rust type compatibility
- Single `hydrate()` call to read entire document
- Single `reconcile()` call to write changes
- No manual type checking or conversion

**String Handling:**
- Use `autosurgeon::Text` for all string fields to ensure CRDT compatibility
- `Text` objects serialize as collaborative text in Automerge
- Prevents `ImmutableString` serialization issues between Rust and JavaScript
- Access content with `.as_str()`, create with `Text::from()`

## CLI Usage

### URL Flexibility

Accept both formats:
```bash
# Plain automerge URL
cargo run --release --bin automerge-cli -- automerge:ABC123 show

# Full browser URL (copy-paste from address bar)
cargo run --release --bin automerge-cli -- "http://localhost:5173/#automerge:ABC123" show
```

### View State

```bash
cargo run --release --bin automerge-cli -- <url> show
```

Output displays:
- All scalar values (counter, temperature, dark mode)
- Text content preview
- List counts (todos, tags)
- Metadata (title)
- Detailed lists (todos with IDs, tags)

### All Commands

```bash
# Counters
increment, decrement, set-counter <value>

# Temperature
set-temp <0-40>

# Dark mode
toggle-dark, set-dark <true|false>

# Text
add-note <text>

# Todos
add-todo <text>
toggle-todo <id>  # Use first 8 chars of ID
delete-todo <id>

# Tags
add-tag <tag>
remove-tag <tag>

# View
show  # Default if no command specified
```

## CRDT Benefits Demonstrated

### Counter
Concurrent increments merge correctly. Two users clicking "+1" simultaneously results in "+2", not a conflict.

### Text
Character-level merging. Multiple users can type in different parts of the notes simultaneously without conflicts.

### Lists
Operations (insert, delete, reorder) merge based on position and causality, not indices.

### Maps/Objects
Field-level merging. Changes to different fields don't conflict.

### Timestamps
Last-write-wins for scalar values. Automerge tracks causality to resolve conflicts deterministically.

## Key Features

**Offline-First:**
- Works without network connection
- Syncs automatically when reconnected
- All changes preserved and merged

**Conflict-Free:**
- CRDTs guarantee convergence
- No merge conflicts
- Deterministic conflict resolution

**Real-Time:**
- Changes sync instantly
- Sub-second latency
- Live cursor tracking (UI-level, not demonstrated)

**Type-Safe:**
- Rust side has compile-time guarantees
- JS side has TypeScript interfaces
- Autosurgeon handles serialization

## Development

### Frontend
```bash
cd frontend
npm install
npm run dev      # Dev server
npm run build    # Production build
npm run lint     # ESLint
```

### CLI
```bash
cd cli
cargo build --release    # Optimized build
cargo run -- <url> show  # Quick test (debug build)
cargo test              # Run tests
```

### Sync Server
```bash
# Run locally
pnpx @automerge/automerge-repo-sync-server

# Or install globally
npm install -g @automerge/automerge-repo-sync-server
automerge-repo-sync-server
```

## Technical Highlights

### Autosurgeon Integration
Eliminated ~80 lines of manual serialization code by using derive macros. Compare:

**Before:**
```rust
let counter = doc.get(ROOT, "counter")?;
let notes = match doc.get(ROOT, "notes")? {
    Some((Value::Object(ObjType::Text), id)) => doc.text(&id)?,
    Some((Value::Scalar(s), _)) => s.to_str().unwrap_or(""),
    // ... more pattern matching
};
```

**After:**
```rust
let state: Doc = hydrate(doc)?;
println!("Counter: {}", state.counter);
println!("Notes: {}", state.notes);
```

### Field Name Mapping
JS uses camelCase, Rust uses snake_case. Solution: Use camelCase in Rust with `#![allow(non_snake_case)]` for exact field name matching.

### Browser URL Parsing
CLI accepts full browser URLs. Extracts document ID from `#automerge:` fragment:
```rust
let doc_id = if let Some(pos) = url.find("#automerge:") {
    &url[pos + 11..]
} else if url.starts_with("automerge:") {
    url.strip_prefix("automerge:").unwrap()
} else {
    bail!("URL must contain 'automerge:' or '#automerge:'")
};
```

## Resources

- [Automerge](https://automerge.org) - CRDT library
- [Automerge Repo](https://github.com/automerge/automerge-repo) - Sync and storage
- [Autosurgeon](https://docs.rs/autosurgeon) - Rust derive macros
- [Samod](https://docs.rs/samod) - Rust automerge-repo implementation
- [shadcn/ui](https://ui.shadcn.com) - UI components
- [CRDT Tech](https://crdt.tech) - CRDT resources

## License

MIT