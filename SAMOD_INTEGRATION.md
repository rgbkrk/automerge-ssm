# Samod Integration

This document describes the transition to using `samod` (Rust implementation of automerge-repo) for the CLI client.

## What Changed

### Before
- Custom WebSocket server with manual sync protocol
- Two CLI clients: `cli_client.rs` (basic) and `repo_client.rs` (samod-based)
- Custom message format and sync handling

### After
- **No custom server needed** - use any automerge-repo sync server
- Single CLI client using `samod` with full protocol compatibility
- Direct compatibility with JavaScript `@automerge/automerge-repo` clients

## Architecture

```
┌─────────────┐                           ┌─────────────┐
│   Browser   │                           │  Rust CLI   │
│  (JS repo)  │                           │  (samod)    │
└──────┬──────┘                           └──────┬──────┘
       │                                         │
       │  WebSocket (automerge-repo protocol)   │
       │                                         │
       └────────────────┬────────────────────────┘
                        │
                  ┌─────▼─────┐
                  │   Sync    │
                  │  Server   │
                  │  :3030    │
                  └───────────┘
```

Both clients:
- Use the same automerge-repo protocol
- Connect to the same sync server
- See each other's changes in real-time
- Can work offline and sync later

## Current Implementation

### CLI Structure

```bash
# View document
cargo run --bin automerge-cli -- automerge:DOCUMENT_ID

# Make changes
cargo run --bin automerge-cli -- automerge:DOCUMENT_ID increment
cargo run --bin automerge-cli -- automerge:DOCUMENT_ID add-note "Text"

# Verbose logging
cargo run --bin automerge-cli -- --verbose automerge:DOCUMENT_ID
```

### Key Components

1. **Connection Setup**: Uses `tokio-tungstenite` to connect to sync server
2. **Bridging**: Converts WebSocket messages to/from samod's expected format
3. **Document Access**: Uses samod's `DocHandle` for thread-safe document access
4. **Data Types**: Properly handles Automerge `Text` objects (not just scalars)

## Known Issues

### 1. Sync Race Condition

**Problem**: The CLI sometimes reads an empty document instead of the synced version.

**Root Cause**: 
- `repo.find()` returns immediately with a `DocHandle`
- The document starts empty (or from storage if available)
- Sync happens asynchronously in the background
- Reading the document before sync completes shows empty/stale data

**Current Workaround**:
```rust
// TODO: Replace sleep with proper reactive sync completion detection
sleep(Duration::from_secs(2)).await;
```

**Proper Fix** (future):
- Listen for sync completion events from samod
- Wait for specific sync state before reading
- Use document version/hash to verify we have latest data
- Implement timeout with fallback behavior

### 2. Document Type Handling

The frontend uses `Text` objects for strings, not scalar strings:

```typescript
// Frontend creates Text objects
d.notes = "text";  // Becomes Automerge.Text
d.collaborators.push("name");  // Also becomes Text
```

The CLI must handle both:
```rust
match doc.get(ROOT, "notes") {
    Ok(Some((Value::Scalar(s), _))) => s.to_str()...,
    Ok(Some((Value::Object(ObjType::Text), obj_id))) => doc.text(&obj_id)...,
    // ...
}
```

## Dependencies

```toml
[dependencies]
automerge = "0.7"
samod = { version = "0.5", features = ["tokio", "tungstenite"] }
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.24"
futures-util = "0.3"
futures = "0.3"
bytes = "1.8"
clap = { version = "4.5", features = ["derive"] }
```

## Testing

### Consistency Check
```bash
# Run multiple times - should always show same data
for i in {1..10}; do 
  cargo run --release --bin automerge-cli -- automerge:DOCID | grep Counter
done
```

Currently inconsistent due to sync race condition.

### Real-time Sync
```bash
# Terminal 1: Watch document
watch -n 1 'cargo run --release --bin automerge-cli -- automerge:DOCID'

# Terminal 2: Make changes
cargo run --release --bin automerge-cli -- automerge:DOCID increment

# Terminal 3: Browser
# Open http://localhost:5174/#automerge:DOCID
```

Changes should appear across all clients within 1-2 seconds.

## Future Improvements

### High Priority
1. **Fix sync race condition** - Listen for actual sync completion
2. **Add sync timeout** - Fail gracefully if sync doesn't complete
3. **Better error messages** - Distinguish "not synced" from "doesn't exist"

### Medium Priority
4. **Persistent storage** - Use filesystem storage adapter instead of in-memory
5. **Interactive mode** - TUI that updates in real-time
6. **Batch operations** - Multiple commands in one connection
7. **JSON output** - Machine-readable format for scripting

### Low Priority
8. **Document creation** - Allow creating new documents from CLI
9. **Offline mode** - Work without sync server, sync later
10. **History/undo** - View and manipulate document history

## Resources

- [Samod Documentation](https://docs.rs/samod)
- [Automerge Repo Protocol](https://github.com/automerge/automerge-repo)
- [Automerge Rust](https://docs.rs/automerge)
- [Example: tcp-example.rs](https://github.com/automerge/automerge-repo-rs/blob/main/examples/tcp-example.rs)

## Development Notes

### Debug Logging

Enable verbose logging to see sync protocol details:
```bash
cargo run --bin automerge-cli -- --verbose automerge:DOCID
```

Or use `RUST_LOG`:
```bash
RUST_LOG=debug cargo run --bin automerge-cli -- automerge:DOCID
```

### Common Issues

**"Connection lost externally" error at exit**
- Normal! The CLI aborts connection tasks when exiting
- Data is already synced before this happens
- Can be ignored

**Empty document on first read**
- Race condition - increase sleep duration
- Check sync server is running and accessible
- Verify document exists in browser first

**"notes field has unexpected type: Object(Text)"**
- Already fixed - CLI handles Text objects
- If you see this, the Text reading code isn't working

## Contributing

When improving the CLI:
1. Test with real sync server (not mock)
2. Test both reading and writing
3. Verify changes appear in browser immediately
4. Check consistency across multiple runs
5. Test with both empty and populated documents