# Jupyter-Style Notebook Implementation

This document summarizes the collaborative notebook implementation built on top of Automerge CRDTs.

## What Was Built

### 1. Data Structures (Rust + TypeScript)

**Rust** (`cli/src/lib.rs`):
```rust
pub struct NotebookCell {
    pub id: String,
    pub cellType: String,        // "code" or "markdown"
    pub source: String,           // CRDT Text via CodeMirror
    pub executionCount: Option<i64>,
    pub outputRefs: Vec<String>,  // URLs to external outputs
}

pub struct NotebookMetadata {
    pub title: Option<String>,
    pub createdAt: Option<i64>,
    pub lastModified: Option<i64>,
}

// Added to main Doc struct:
pub cells: Vec<NotebookCell>,
pub notebookMetadata: NotebookMetadata,
```

**TypeScript** (`frontend/src/App.tsx`):
```typescript
interface NotebookCell {
  id: ImmutableString | string;
  cellType: ImmutableString | string;
  source: ImmutableString | string;
  executionCount: number | null;
  outputRefs: (ImmutableString | string)[];
}

interface NotebookMetadata {
  title?: ImmutableString | string;
  createdAt?: number;
  lastModified?: number;
}
```

### 2. Frontend Components

#### `NotebookCell.tsx`
- Renders individual notebook cells with CodeMirror
- Supports code and markdown cell types
- Execution button with loading state
- Cell reordering (move up/down)
- Delete functionality
- Output display area

#### `Output.tsx`
- Renders cell outputs with multiple MIME type support:
  - `text/plain`
  - `text/html`
  - `image/png`
  - `image/jpeg`
  - Error outputs with special styling

#### Updated `App.tsx`
- Full notebook UI section
- Add Code Cell / Add Markdown Cell buttons
- Cell management functions:
  - `addCell(cellType)`
  - `deleteCell(index)`
  - `moveCellUp(index)`
  - `moveCellDown(index)`
  - `executeCell(index)` - currently mocked

### 3. Rust CLI Support

**New Commands** (`cli/src/main.rs`):
```bash
add-cell [code|markdown]           # Add new cell
delete-cell <index>                # Delete cell by index
set-cell-source <index> <source>   # Update cell source
execute-cell <index>               # Execute cell (mock)
show cells                         # Display all cells
```

**Features**:
- Add/delete cells from command line
- Update cell source programmatically
- Mock execution with output refs
- Display cell information with formatting

### 4. Design Principles

#### Clean CRDT Structure
- Cell source code stored as CRDT Text (collaborative editing)
- Cell metadata (type, ID, execution count) as scalars
- **Outputs NOT stored in CRDT** - only refs to external URLs
- This keeps the CRDT document size manageable

#### Output Storage Strategy
```
Cell Execution Flow:
1. User clicks "Run" or executes via CLI
2. Code sent to execution backend (runtimed)
3. Execution produces outputs
4. Outputs stored in artifact service (S3, CDN, etc.)
5. Artifact URL added to cell.outputRefs[]
6. All collaborators fetch outputs from URL
```

**Benefits**:
- CRDT stays small (only URLs, not output data)
- Large outputs (plots, images) don't bloat document
- Easy to expire/clean old outputs
- Can use CDN for fast delivery

#### CodeMirror Integration
- Each cell uses `AutomergeCodeMirror` component
- Path: `["cells", index, "source"]`
- Language switching based on cell type
- Character-level CRDT synchronization

### 5. Key Files Modified/Created

**Created**:
- `frontend/src/components/NotebookCell.tsx` (145 lines)
- `frontend/src/components/Output.tsx` (74 lines)
- `NOTEBOOK_RUNTIMED.md` (integration guide)
- `NOTEBOOK_IMPLEMENTATION.md` (this file)

**Modified**:
- `cli/src/lib.rs` - Added `NotebookCell` and `NotebookMetadata` structs
- `cli/src/main.rs` - Added 4 new notebook commands
- `frontend/src/App.tsx` - Added notebook UI and functions
- `README.md` - Documented notebook features

### 6. What Works Now

✅ **Collaborative Editing**
- Multiple users can edit cell source simultaneously
- Real-time sync via Automerge CRDT
- Works across browser and CLI

✅ **Cell Management**
- Add/delete cells from UI or CLI
- Reorder cells (move up/down)
- Support for code and markdown cells

✅ **Cell Execution (Mock)**
- Click "Run" button increments execution count
- Generates mock output ref
- Displays execution count badge

✅ **Type Safety**
- Full TypeScript types for all data structures
- Rust structs with proper hydration
- Cross-platform compatibility verified

✅ **UI/UX**
- Clean shadcn/ui design
- Dark mode support
- Responsive layout
- Keyboard-friendly

### 7. What's Not Implemented (Future Work)

❌ **Real Code Execution**
- Currently mocked - no actual code runs
- See `NOTEBOOK_RUNTIMED.md` for integration plan

❌ **Output Fetching**
- Output refs are created but not fetched
- Need artifact service implementation

❌ **Advanced Features**
- Kernel session management
- Streaming outputs
- Interrupt execution
- Cell dependencies
- Rich outputs (interactive widgets)
- Export to .ipynb format

### 8. Testing the Implementation

**Start the Stack**:
```bash
# Terminal 1: Sync server
pnpx @automerge/automerge-repo-sync-server

# Terminal 2: Frontend
cd frontend && npm run dev

# Terminal 3: CLI (optional)
cd cli
cargo run -- "http://localhost:5173/#automerge:YOUR_DOC_ID" show
```

**Try It Out**:
1. Open http://localhost:5173
2. Scroll to "Collaborative Notebook" section
3. Click "Add Code Cell" or "Add Markdown Cell"
4. Type in the CodeMirror editor
5. Open another browser tab with same URL - see real-time sync!
6. Click "Run" on a code cell - see execution count increment
7. Try CLI commands:
   ```bash
   cargo run -- "YOUR_URL" add-cell markdown
   cargo run -- "YOUR_URL" show cells
   cargo run -- "YOUR_URL" execute-cell 0
   ```

### 9. Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│                 Browser #1                           │
│  ┌───────────────────────────────────────────────┐  │
│  │  Cell 0: [Code] "print('hello')"              │  │
│  │  Cell 1: [Markdown] "# Title"                 │  │
│  └───────────────────────────────────────────────┘  │
└──────────────────┬──────────────────────────────────┘
                   │
                   ▼
         ┌─────────────────┐
         │  Sync Server     │◄────────────┐
         │  (WebSocket)     │             │
         └─────────────────┘             │
                   │                      │
    ┌──────────────┴──────────────┐      │
    │                             │      │
    ▼                             ▼      ▼
┌─────────────────┐    ┌─────────────────────┐
│  Browser #2      │    │   Rust CLI          │
│  (Collaborative) │    │   cargo run --      │
│                  │    │   add-cell code     │
└─────────────────┘    └─────────────────────┘

All three clients see the same cells in real-time!
```

### 10. Performance Considerations

**CRDT Size**:
- Each cell adds ~200 bytes (ID, type, metadata)
- Cell source is CRDT Text - grows with edits
- 100 cells with 1KB each = ~100KB CRDT
- Outputs NOT in CRDT - prevents bloat

**Sync Performance**:
- Automerge efficiently syncs only changes
- Character-level edits send minimal data
- Binary sync protocol is compact

**Recommended Limits**:
- Max 1000 cells per notebook
- Max 100KB per cell source
- Output refs should expire after 30 days

### 11. Design Decisions Explained

**Why separate cells array instead of cell-per-doc?**
- Simpler for small notebooks (<100 cells)
- Easier to reorder
- Can switch to per-doc for large notebooks

**Why store outputs as refs?**
- Jupyter outputs can be HUGE (plots, dataframes)
- CRDT would become slow with large binary data
- Refs keep document lightweight
- Can use CDN for fast delivery

**Why mock execution?**
- Focus on CRDT/collaboration first
- Execution is orthogonal concern
- Easy to integrate runtimed later
- Safe for demo (no arbitrary code execution)

**Why CodeMirror over plain textarea?**
- Syntax highlighting
- Multi-cursor support
- Professional editing experience
- CRDT integration built-in

## Conclusion

This implementation demonstrates a production-ready pattern for building collaborative notebooks with Automerge CRDTs. The clean separation of concerns (CRDT for structure, external storage for outputs) ensures scalability while maintaining real-time collaboration.

The next step is integrating with [runtimed](https://github.com/runtimed/runtimed) for actual code execution - see `NOTEBOOK_RUNTIMED.md` for the implementation plan.
