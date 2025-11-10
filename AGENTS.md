# Agentic Engineering with Automerge

This document describes how AI agents can effectively develop and debug this Automerge application using both the CLI and browser MCP (Model Context Protocol) tools.

## Overview

This project demonstrates a powerful agentic engineering workflow where an AI agent can:

1. **Edit code** in both Rust and TypeScript
2. **Run the CLI** to test document operations
3. **Interact with the browser** to verify UI behavior
4. **Debug cross-platform sync** between Rust and JavaScript implementations

## Tool Access

### CLI Access

Agents can run the Rust CLI to:
- Create and modify Automerge documents
- Test CRDT operations (counter, text, lists, maps)
- Verify sync behavior
- Debug data serialization

```bash
# Example CLI commands
cd cli && cargo run -- automerge:DOC_ID show
cd cli && cargo run -- automerge:DOC_ID toggle-dark
cd cli && cargo run -- automerge:DOC_ID add-todo "Fix bug"
```

### Browser MCP Access

The browser MCP provides:
- `browser_navigate` - Load URLs in the browser
- `browser_snapshot` - Capture accessibility tree of current page
- `browser_click` - Interact with UI elements
- `browser_type` - Enter text into form fields
- `browser_wait` - Wait for async operations
- `browser_screenshot` - Capture visual state (may have limitations)

Note: Click interactions may show a cursor movement but not always register. The human operator can assist when needed.

## Development Workflow

### 1. Development Environment (Prerequisites)

**Note**: The user should have these already running before agent work begins:

```bash
# Terminal 1: Sync server (USER runs this)
pnpx @automerge/automerge-repo-sync-server

# Terminal 2: Frontend (USER runs this)
cd frontend && npm run dev

# Terminal 3: Agent operations (AGENT works here)
# Agent executes CLI commands in this terminal
```

The agent assumes `http://localhost:5173` is accessible and the sync server is running on the default port.

### 2. Create a Test Document

```bash
# Navigate browser to create new document
browser_navigate("http://localhost:5173/")
# Document ID will be in URL hash: #automerge:...
```

### 3. Test Cross-Platform Operations

**Pattern**: Make changes via CLI, verify in browser, and vice versa.

```bash
# CLI: Add data
cargo run -- automerge:DOC_ID add-todo "Test item"

# Browser: Verify via snapshot
browser_snapshot()
# Check that "Test item" appears in the todo list

# Browser: Make UI change (human assists with clicks)
# Verify change appears in CLI
cargo run -- automerge:DOC_ID show
```

### 4. Debug Issues

When things break:

1. **Check browser console** - Look for React errors or type mismatches
2. **Run CLI show command** - See what the document actually contains
3. **Create fresh documents** - Old documents may have stale data structures
4. **Use diagnostics tool** - Check TypeScript compilation errors

## Common Debugging Patterns

### Pattern 1: Type Mismatch Investigation

```typescript
// Browser console debugging
console.log(window.handle.doc())
console.log(typeof doc.someField)
console.log(doc.someField)  // Is it a string or {val: "string"}?
```

### Pattern 2: Sync Verification

```bash
# Make change in one location
cargo run -- automerge:DOC_ID increment

# Wait for sync
browser_wait(1)

# Verify change appeared
browser_snapshot()
# Look for updated counter value
```

### Pattern 3: Document Structure Inspection

```bash
# CLI provides formatted output
cargo run -- automerge:DOC_ID show

# Returns structured view like:
# ╭─────────────────────────────────────────╮
# │ 🔢 Counter: 5                           │
# │ 🌡️  Temperature: 20°C                   │
# │ 🌙 Dark Mode: ON                        │
# ╰─────────────────────────────────────────╯
```

## Agentic Engineering Best Practices

### Do's

✅ **Create fresh documents for testing** - Use `browser_navigate("http://localhost:5173/")` to get clean state

✅ **Wait between operations** - Use `browser_wait(1)` after making changes to allow sync

✅ **Verify both directions** - Test CLI→Browser and Browser→CLI sync

✅ **Use type-safe patterns** - When possible, use proper TypeScript types rather than runtime checks

✅ **Commit incrementally** - Small, focused commits with clear messages

✅ **Document discoveries** - Update HANDOFF.md or similar when finding issues

### Don'ts

❌ **Don't assume browser clicks work** - Click events may fail silently, ask human to verify

❌ **Don't test on old documents** - Schema changes may break old data

❌ **Don't ignore type errors** - TypeScript diagnostics often reveal real issues

❌ **Don't skip sync waits** - Async operations need time to complete

❌ **Don't mix concerns** - Keep string types (Text vs ImmutableString) consistent

## Example Debugging Session

Here's how an agent debugged the todo checkbox issue:

### Problem Discovery
```bash
# Human reports: "Can't click checkbox on CLI-created todos"
```

### Investigation Steps

1. **Create test document**
   ```typescript
   browser_navigate("http://localhost:5173/")
   // Got: automerge:ABC123
   ```

2. **Add todo via CLI**
   ```bash
   cargo run -- automerge:ABC123 add-todo "Test from CLI"
   ```

3. **Check browser state**
   ```typescript
   browser_snapshot()
   // Found: checkbox exists but not interactive
   ```

4. **Hypothesis**: Type mismatch on `todo.id` field
   - Rust sends `String` → serializes as `ImmutableString`
   - TypeScript expects `string`
   - Comparison `t.id === id` fails because types don't match

5. **Test fix**: Wrap comparisons with `getString()`
   ```typescript
   const todo = d.todos?.find((t) => getString(t.id) === id);
   ```

6. **Verify fix**
   ```bash
   # Create new document
   cargo run -- automerge:NEW_ID add-todo "Test fix"
   # Human clicks checkbox
   cargo run -- automerge:NEW_ID show
   # ✓ Shows checked!
   ```

7. **Commit solution**
   ```bash
   git commit -m "Fix todo checkbox interaction - use getString() for id comparison"
   ```

## Browser MCP Limitations

### What Works Well
- Page navigation
- Accessibility tree inspection (excellent for finding elements)
- Reading page state
- Waiting for async operations

### What's Challenging
- Click events may not register (shows cursor but no action)
- Screenshot may fail with extension errors
- Need human verification for interactive operations

### Workarounds
- Use `browser_snapshot()` extensively to verify state
- Ask human to perform clicks when needed
- Use CLI to make changes, browser to verify
- Focus on data flow rather than UI interaction

## Tips for Effective Agent Work

1. **Understand the architecture**: Repo → DocHandles → Documents → Sync

2. **Know the data flow**: 
   - Changes go through `handle.change()`
   - Sync is automatic via NetworkAdapters
   - Storage persists to IndexedDB

3. **Use the right tool**:
   - Need to test data? Use CLI
   - Need to verify UI? Use browser snapshot
   - Need to debug types? Check TypeScript diagnostics

4. **Work incrementally**:
   - Make small changes
   - Test immediately
   - Commit when working
   - Document learnings

5. **Communicate clearly**:
   - Explain what you're testing
   - Show intermediate results
   - Ask for help when browser interaction is needed
   - Document issues for upstream projects

## Advanced Patterns

### Testing Concurrent Edits

```bash
# Terminal 1: CLI makes change
cargo run -- automerge:DOC_ID set-counter 10

# Terminal 2: Browser makes concurrent change (human assists)
# User clicks increment button

# Both merge correctly to 11
```

### Schema Migration Testing

```bash
# Create old schema document
cargo run -- automerge:OLD_DOC show

# Update code with new schema
# edit_file ...

# Test migration
cargo run -- automerge:OLD_DOC show
# Verify old doc still works or fails gracefully
```

### Performance Testing

```typescript
// Create document with lots of data
for (let i = 0; i < 1000; i++) {
  handle.change(d => d.todos.push({
    id: `${Date.now()}-${Math.random()}`,
    text: `Todo ${i}`,
    completed: false
  }));
}
// Monitor memory, sync time, render performance
```

## Getting Help

When stuck:

1. **Check HANDOFF.md** - Documents known issues and investigation plans
2. **Read diagnostics** - TypeScript errors are usually accurate
3. **Ask human** - Especially for browser interaction verification
4. **Search docs** - Automerge docs fetched in context
5. **Create minimal repro** - Isolate the issue for upstream reporting

## Contributing

When you discover issues or improvements:

1. **Test thoroughly** - Verify cross-platform behavior
2. **Document clearly** - Update HANDOFF.md or create new docs
3. **Commit with context** - Explain why, not just what
4. **Consider upstream** - Is this an Automerge issue? Document for maintainers

## Resources

- [Automerge Docs](https://automerge.org/docs/) - Full documentation
- `README.md` - Project setup and usage
- `HANDOFF.md` - Current investigation status
- Browser DevTools - Invaluable for debugging
- Rust tracing output - Add `--verbose` flag to CLI

---

**Remember**: Agentic engineering works best when combining AI capabilities (code generation, pattern recognition) with human strengths (UI interaction verification, judgment calls). This project demonstrates effective collaboration between human and AI developers.