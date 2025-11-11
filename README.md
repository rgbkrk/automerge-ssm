# Autodash

Real-time collaborative CRDT demo showcasing **full cross-platform synchronization** between a React frontend and Rust CLI, powered by Automerge.

## ✨ Highlights

**🎯 Cross-Platform Collaboration**
- React + TypeScript frontend and Rust CLI share the same live document
- All edits sync in real-time through WebSocket server
- Proper type handling ensures seamless Rust ↔ JavaScript interoperability

**🔧 Advanced CRDT Operations**
- Counter with concurrent increment merging
- Temperature slider with conflict-free updates
- Character-level text editing with insert/delete at any position
- Todo list with add/toggle/delete operations
- Tag management with array operations
- Metadata with nested object updates

**💪 Production-Ready Patterns**
- TypeScript union types for `ImmutableString | string`
- Rust custom hydration for Text CRDT objects
- UTF-8 safe character positioning
- Full bidirectional sync verified

## Quick Start

### Prerequisites

```bash
# Install Node.js dependencies
cd frontend && npm install

# Rust is required for CLI
# Install from https://rustup.rs

# Optional: Clone vendor submodules (for source exploration only)
# Not required to build or run the application
git submodule update --init --recursive
```

**Note**: The `vendor/` directory contains Git submodules for Automerge and Autosurgeon source code. These are **optional** and only needed if you want to explore the library implementations. The actual dependencies are fetched from crates.io and npm during build.

### Running the Stack

```bash
# Terminal 1: Sync server
pnpx @automerge/automerge-repo-sync-server

# Terminal 2: Frontend (will open browser at localhost:5173)
cd frontend && npm run dev

# Terminal 3: CLI (use document ID from browser URL)
cd cli
cargo run -- automerge:YOUR_DOC_ID show
```

Open http://localhost:5173 and watch changes sync between browser and CLI in real-time!

## CLI Commands

### Scalars
```bash
increment                    # Counter +1
decrement                    # Counter -1
set-counter <value>          # Set counter to specific value
set-temp <0-40>             # Set temperature
toggle-dark                  # Toggle dark mode
set-dark <true|false>       # Set dark mode explicitly
```

### Text (Character-Level Operations)
```bash
add-note <text>             # Append to notes
set-notes <text>            # Replace all notes
clear-notes                 # Clear notes
insert-notes <pos> <text>   # Insert at character position
delete-notes <start> <len>  # Delete character range
```

### Lists - Todos
```bash
add-todo <text>             # Create new todo
toggle-todo <id>            # Toggle completion (use first 8 chars)
delete-todo <id>            # Remove todo
```

### Lists - Tags
```bash
add-tag <tag>               # Add tag to list
remove-tag <tag>            # Remove tag from list
```

### Metadata
```bash
set-title <title>           # Set document title
```

### Display
```bash
show                        # Show current state (default)
```

## Architecture

```
┌─────────────────┐
│  React Browser  │◄─┐
└─────────────────┘  │
                     ├─► WebSocket Sync Server
┌─────────────────┐  │
│  React Browser  │◄─┤
└─────────────────┘  │
                     │
┌─────────────────┐  │
│   Rust CLI      │◄─┘
└─────────────────┘
```

**Frontend Stack**
- React 19 + TypeScript
- @automerge/automerge-repo for CRDT sync
- @automerge/automerge-repo-react-hooks
- shadcn/ui components
- IndexedDB for persistence

**CLI Stack**
- Rust with autosurgeon derive macros
- samod (automerge-repo for Rust)
- Custom hydration for cross-platform Text fields
- In-memory storage

**Sync Server**
- Standard @automerge/automerge-repo-sync-server
- WebSocket-based
- No custom modifications needed

## Cross-Platform Type Handling

### TypeScript Solution

Use union types and a simple conversion helper:

```typescript
import { ImmutableString } from "@automerge/automerge";

interface TodoItem {
  id: ImmutableString | string;
  text: ImmutableString | string;
  completed: boolean;
}

// Simple conversion - works for both types
const toStr = (value: ImmutableString | string): string => {
  if (typeof value === "string") return value;
  return value.toString();
};

// Usage
{todos.map(todo => (
  <span key={toStr(todo.id)}>{toStr(todo.text)}</span>
))}
```

### Rust Solution

Use custom hydration to handle Text CRDT objects from JavaScript:

```rust
#[derive(Debug, Clone, Reconcile, Hydrate)]
struct TodoItem {
    #[autosurgeon(hydrate = "hydrate_string_or_text")]
    id: String,
    #[autosurgeon(hydrate = "hydrate_string_or_text")]
    text: String,
    completed: bool,
}

fn hydrate_string_or_text<D: autosurgeon::ReadDoc>(
    doc: &D,
    obj: &automerge::ObjId,
    prop: autosurgeon::Prop,
) -> Result<String, autosurgeon::HydrateError> {
    use automerge::{ObjType, Value};
    match doc.get(obj, &prop)? {
        Some((Value::Scalar(s), _)) => Ok(s.to_str()?.to_string()),
        Some((Value::Object(ObjType::Text), text_obj)) => doc.text(&text_obj),
        _ => Ok(String::new()),
    }
}
```

This allows Rust to read both:
- Scalar strings (from Rust)
- Text CRDT objects (from JavaScript)

## CRDT Benefits in Action

**Counter**: Two users click increment simultaneously → both increments apply (+2 total), no conflict

**Text**: User A types at start, User B types at end → both edits merge correctly

**Character-Level Edits**: `insert-notes 5 "hello"` and concurrent edits merge by CRDT position, not array index

**Lists**: Concurrent todo additions preserve both items with correct causality

**Offline-First**: All changes stored locally in IndexedDB, sync automatically when reconnected

## Type Schema

```rust
#[derive(Debug, Clone, Default, Reconcile, Hydrate)]
struct Doc {
    counter: i64,
    temperature: i64,
    darkMode: bool,
    #[autosurgeon(hydrate = "hydrate_string_or_text")]
    notes: String,
    todos: Vec<TodoItem>,
    #[autosurgeon(hydrate = "hydrate_string_vec")]
    tags: Vec<String>,
    metadata: Metadata,
}

#[derive(Debug, Clone, Default, Reconcile, Hydrate)]
struct Metadata {
    createdAt: Option<i64>,
    lastModified: Option<i64>,
    #[autosurgeon(hydrate = "hydrate_optional_string_or_text")]
    title: Option<String>,
}
```

## Development

### Frontend
```bash
cd frontend
npm install
npm run dev        # Development server
npm run build      # Production build
npm run lint       # Type checking
```

### CLI
```bash
cd cli
cargo build        # Debug build
cargo build --release  # Optimized build
cargo test         # Run tests
cargo run -- automerge:DOC_ID show
```

### Testing Cross-Platform Sync

1. Open browser at http://localhost:5173
2. Copy the document ID from URL (after `#automerge:`)
3. Run CLI: `cargo run -- automerge:DOC_ID increment`
4. Watch counter update in browser instantly
5. Modify data in browser, run `show` in CLI to verify sync

## What We Learned

### TypeScript + ImmutableString
- Union types (`ImmutableString | string`) handle both Rust and JavaScript string representations
- `.toString()` method works polymorphically on both types
- No need for complex runtime type checking

### Rust + Text Hydration
- JavaScript creates Text CRDT objects for string fields
- Custom hydration functions handle both scalar strings and Text objects
- Apply to all string fields that cross platform boundaries

### Character-Level Operations
- Convert character positions to byte indices for UTF-8 safety
- `char_indices()` provides safe navigation through multi-byte characters
- Enables true collaborative text editing

### CRDT Power
- Eliminates merge conflicts across all data types
- Enables offline-first applications
- Simplifies distributed system design

## Resources

- [Automerge Documentation](https://automerge.org/docs/)
- [Automerge Repo](https://github.com/automerge/automerge-repo)
- [Autosurgeon (Rust)](https://docs.rs/autosurgeon)
- [CRDT Tech](https://crdt.tech)
- [AGENTS.md](./AGENTS.md) - AI agent development workflow

## Vendor Directory

The `vendor/` directory contains Git submodules with source code for:
- [Automerge](https://github.com/automerge/automerge) - Core CRDT library
- [Autosurgeon](https://github.com/automerge/autosurgeon) - Rust derive macros

**These submodules are optional.** They are included for source code exploration and debugging, but are not required to build or run the application. All dependencies are fetched from:
- Rust: crates.io
- JavaScript: npm

To clone submodules (if you want to explore source):
```bash
git submodule update --init --recursive
```

To clone the repo **without** submodules:
```bash
git clone https://github.com/your-repo/automerge-ssm.git
# Submodules won't be downloaded
```

## License

MIT