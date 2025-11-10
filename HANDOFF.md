# HANDOFF: ImmutableString Issue in Automerge Rust/JS Interop

## Problem Summary

When the Rust CLI writes to an Automerge document, it converts string fields to `ImmutableString` objects (format: `{val: "string"}`) instead of collaborative text. When React tries to render these objects, it crashes with:

```
Error: Objects are not valid as a React child (found: object with keys {val})
```

## Root Cause

`autosurgeon`'s default `Reconcile` implementation for `String` and `Vec<String>` uses `ImmutableString` for atomic (non-collaborative) strings. According to Automerge 3 migration docs:

- **Collaborative strings** (character-by-character merging) → plain `string` in JS, `autosurgeon::Text` in Rust
- **Atomic strings** (replace whole value) → `ImmutableString` in Automerge

The problem: When Rust calls `autosurgeon::reconcile()`, it reconciles the ENTIRE document, not just the changed field. This converts ALL String fields to ImmutableString, even if we only changed a boolean.

## What We Fixed

### In Rust (`cli/src/bin/repo_client.rs`):
1. Changed `notes: String` → `notes: autosurgeon::Text` (line ~206)
2. Changed `TodoItem.text: String` → `TodoItem.text: autosurgeon::Text` (line ~73)

### In Frontend (`frontend/src/App.tsx`):
1. Added `getString()` helper function to safely extract strings from ImmutableString objects
2. Applied `getString()` to:
   - `doc.notes` in Textarea (line ~477)
   - `todo.text` when rendering todos (line ~527)
   - `tag` when rendering tags (lines ~574-576)
   - `name` when rendering collaborators (line ~626)

### Added Error Boundary
- Created `ErrorBoundary.tsx` component to catch and display React errors with copy-to-clipboard functionality

## What Still Needs Fixing

### Still Crashing
The app STILL crashes when Rust CLI toggles dark mode. Looking at the error: "in the <p> component", there's likely another place rendering ImmutableString objects we haven't found.

### Known Issues with Current Approach

**Problem**: `tags` and `collaborators` are `Vec<String>` in Rust:
```rust
#[autosurgeon(hydrate = "hydrate_string_vec_or_text")]
tags: Vec<String>,
```

When autosurgeon reconciles these, it writes ImmutableString to each array element. We have custom hydration but NO custom reconciliation.

## Solutions to Try

### Option 1: Fix in Rust (Recommended)
Change tags/collaborators to use Text objects:
```rust
tags: Vec<autosurgeon::Text>,
collaborators: Vec<autosurgeon::Text>,
```

Update all places that push to these arrays:
```rust
state.tags.push(autosurgeon::Text::from(tag.as_str()));
state.collaborators.push(autosurgeon::Text::from(name.as_str()));
```

### Option 2: Custom Reconcile (Complex)
Implement custom reconcile functions for Vec<String> that write collaborative text instead of ImmutableString. This is complex due to autosurgeon's API.

### Option 3: Harden Frontend (Fallback)
Ensure EVERY place that renders strings uses `getString()`. Search for:
- `{doc.something}` 
- `{item.something}`
- Any place JSX renders a value from the document

## Testing Strategy

1. Create a FRESH document (old documents have ImmutableString data)
2. Use the CLI to make ANY change (not just string fields)
3. Watch browser console - if `notes` becomes `ImmutableString type: object`, the fix didn't work
4. Check the UI doesn't crash

Test URL pattern: `http://localhost:5173/#automerge:[docId]`

## Debugging Tips

### Check what type values are:
Look at browser console logs in `updateDoc()` (around line 140):
```
currentDoc.notes: afsdfasdf type: string  ✅ GOOD
currentDoc.notes: ImmutableString type: object  ❌ BAD
```

### Find where it's crashing:
Error boundary shows component stack. Look for what's being rendered in that component.

## Current Branch

Branch: `fix-text-object-rendering`

Commits:
- `f62670b` - Fix ImmutableString issue by using autosurgeon::Text for notes
- `c96686a` - Fix TodoItem text field to use autosurgeon::Text
- Several earlier commits adding getString helper and error boundary

## Key Files

- `cli/src/bin/repo_client.rs` - Rust document structure and reconciliation
- `frontend/src/App.tsx` - React app, has getString helper and rendering
- `frontend/src/ErrorBoundary.tsx` - Error boundary for catching render errors

## Reference Links

- Automerge 3 Migration: See vendor docs about ImmutableString vs collaborative text
- autosurgeon Reconcile trait: Need to understand how to customize string reconciliation
- Peritext/Text in Automerge: How collaborative text works

## Next Steps

1. Search for ALL remaining places ImmutableString objects might be rendered
2. Either fix all in Rust (change to Text) OR ensure getString() is used everywhere in frontend
3. Test with multiple fresh documents
4. Consider: Do we even WANT collaborative text for tags/collaborators? Maybe ImmutableString is fine and we just need robust rendering

## Notes

- The issue ONLY manifests when Rust writes to a document JavaScript created, or vice versa
- Pure JS-to-JS or Rust-to-Rust works fine
- The crash happens during React render, not during document update
- Diagnostic logs show notes stays as "type: string" after our fixes ✅