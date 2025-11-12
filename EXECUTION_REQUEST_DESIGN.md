# Execution Request Design: Ephemeral Messages vs Ledger

## Context

We need a mechanism for requesting code execution in notebook cells that:
- Works across multiple kernel types (pyodide browser, local, remote)
- Handles kernel crashes and restarts gracefully
- Supports collaborative multi-client scenarios
- Uses only the Automerge WebSocket (no additional channels)

Automerge provides two relevant capabilities:
1. **Ephemeral Messages**: Transient event broadcasts via `docHandle.broadcast()` and `docHandle.ephemera()`
2. **CRDT State**: Persistent shared state in the Automerge document

## Option A: Ephemeral Messages

### Architecture

**Frontend:**
```typescript
// Send execution request
docHandle.broadcast({
  type: "execute",
  cellIndex: 0,
  requestId: crypto.randomUUID()
});
```

**Kernel (Rust):**
```rust
// Listen for ephemeral messages
let mut ephemera = doc_handle.ephemera();
while let Some(message_bytes) = ephemera.next().await {
    let message: ExecuteMessage = decode_cbor(&message_bytes)?;
    if message.type == "execute" {
        execute_cell(&doc_handle, message.cellIndex).await?;
    }
}
```

### Advantages
- ✅ **Clean CRDT**: No pollution of document history with transient execution state
- ✅ **Simple model**: Events are ephemeral, state (outputs) is persistent
- ✅ **Lightweight**: No cleanup logic needed

### Disadvantages
- ❌ **Lost messages**: If kernel is down when execute is broadcast, request is lost forever
- ❌ **No crash recovery**: If kernel dies mid-execution, no way to detect or retry
- ❌ **No observability**: Other clients can't see that execution is in progress
- ❌ **Network fragility**: Offline or slow network means lost requests
- ⚠️ **Race conditions**: Multiple clients broadcasting same execution needs deduplication logic

## Option B: Execution Ledger (in CRDT)

### Architecture

**Data Structure:**
```rust
#[derive(Debug, Clone, Reconcile, Hydrate)]
pub struct ExecutionRequest {
    pub id: String,              // UUID for deduplication
    pub cellIndex: usize,
    pub timestamp: i64,
    pub status: String,          // "pending" | "executing" | "completed" | "failed"
    pub kernelId: Option<String>, // Which kernel claimed it
    pub requestedBy: String,     // Client/user ID
}
```

**Frontend:**
```typescript
// Add execution request to CRDT
docHandle.change((doc) => {
  doc.executionRequests.push({
    id: crypto.randomUUID(),
    cellIndex: 0,
    timestamp: Date.now(),
    status: "pending",
    requestedBy: clientId
  });
});
```

**Kernel:**
```rust
// Poll for pending requests
loop {
    let requests = doc_handle.with_document(|doc| {
        let state: Doc = hydrate(doc)?;
        state.executionRequests.iter()
            .filter(|r| r.status == "pending")
            .cloned()
            .collect()
    });

    for request in requests {
        // Claim request
        claim_and_execute(&doc_handle, request).await?;
    }

    sleep(Duration::from_millis(500)).await;
}
```

### Advantages
- ✅ **Durable requests**: Survive kernel crashes, network issues, offline scenarios
- ✅ **Automatic queuing**: Kernel down? Requests queue up, execute when kernel starts
- ✅ **Crash recovery**: Detect stale "executing" requests, can retry or mark failed
- ✅ **Multi-client safety**: CRDT merge semantics handle concurrent requests naturally
- ✅ **Observability**: All clients see execution state ("Cell 3 is executing...")
- ✅ **Network resilient**: Requests sync when connection restored

### Disadvantages
- ⚠️ **CRDT bloat**: Execution history accumulates in document
  - *Mitigation*: Periodic cleanup of completed requests (keep last N or last 24h)
- ⚠️ **Polling overhead**: Kernel must poll for pending requests
  - *Note*: Minimal overhead, can listen to CRDT change stream to optimize
- ⚠️ **More complex**: Status state machine, cleanup logic, timeout detection

## Scenario Analysis

| Scenario | Ephemeral | Ledger | Winner |
|----------|-----------|--------|--------|
| **Simple execution** | ✅ Works | ✅ Works | Tie |
| **Kernel is down** | ⚠️ Manual retry needed | ✅ Auto-executes when kernel starts | **Ledger** |
| **Kernel crashes mid-execution** | ⚠️ Manual retry needed | ✅ Detectable via stale status, can retry | **Ledger** |
| **Multiple clients execute simultaneously** | ⚠️ Need deduplication | ✅ CRDT merges, one executes | **Ledger** |
| **Long-running execution (5+ min)** | ⚠️ No visibility | ✅ Status visible to all clients | **Ledger** |
| **CRDT document size** | ✅ No bloat | ⚠️ Needs cleanup | **Ephemeral** |
| **Offline/poor network** | ❌ Requests lost | ✅ Requests durable | **Ledger** |

## Hybrid Approach?

Could we combine both:
- **Ledger** for durable request storage
- **Ephemeral messages** for instant notifications ("kernel started executing cell 3")

**Analysis:** The CRDT change notifications already provide near-instant feedback when the kernel updates the document. The hybrid adds complexity without significant benefit.

## Recommendation: **Ledger Approach**

For a collaborative, network-aware notebook system with multiple kernel types, the **Ledger approach is more robust**:

1. Handles real-world failure modes (crashes, network issues)
2. Provides automatic recovery and queuing
3. Better multi-client coordination
4. Visible execution state for all collaborators

The CRDT bloat concern is manageable with cleanup logic. The benefits for reliability and collaboration outweigh the overhead.

### Implementation Plan

1. Add `executionRequests: Vec<ExecutionRequest>` to `Doc` struct
2. Frontend: Add request on "Run" button click
3. Kernel:
   - Poll for `pending` requests
   - Claim by setting `status: "executing"` + `kernelId`
   - Execute cell, update outputs
   - Mark `status: "completed"`
4. Cleanup: Periodically remove old completed/failed requests
5. Timeout detection: Flag stale "executing" requests (>60s old) for retry

## References

- **Automerge Ephemeral Messages**: https://automerge.org/docs/reference/repositories/ephemeral/
- **Samod (Rust) API**: `DocHandle::broadcast()` and `DocHandle::ephemera()`
- **CRDT Semantics**: Automerge handles concurrent updates via operational transformation
