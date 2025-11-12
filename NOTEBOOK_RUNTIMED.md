# Notebook + Runtimed Integration Guide

This document outlines how to integrate the collaborative notebook with [runtimed](https://github.com/runtimed/runtimed) for actual code execution.

## Current Status: Mock Implementation

The current implementation includes:

- ✅ Notebook cell data structures (CRDT-based)
- ✅ CodeMirror editors for code and markdown cells
- ✅ Cell execution UI with execution counts
- ✅ Output references stored as URLs
- 🔄 **Mock execution** - outputs are simulated, not real

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│              Browser Frontend                        │
│  ┌──────────────────────────────────────────────┐  │
│  │  Notebook Cells (Automerge CRDT)             │  │
│  │  - source: Text (collaborative editing)      │  │
│  │  - executionCount: number                    │  │
│  │  - outputRefs: string[] (URLs)               │  │
│  └──────────────────────────────────────────────┘  │
│                      │                               │
│                      ▼                               │
│  ┌──────────────────────────────────────────────┐  │
│  │  Execute Cell Handler                        │  │
│  │  (Frontend: executeCell function)            │  │
│  │  (CLI: ExecuteCell command)                  │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────┐
│            Runtimed Integration Layer                │
│  (To be implemented)                                 │
│                                                      │
│  ┌──────────────────────────────────────────────┐  │
│  │  1. Send code to runtimed kernel             │  │
│  │     - POST /execute                          │  │
│  │     - payload: { source, kernel_type }       │  │
│  └──────────────────────────────────────────────┘  │
│                      │                               │
│                      ▼                               │
│  ┌──────────────────────────────────────────────┐  │
│  │  2. Receive execution results                │  │
│  │     - Stream or WebSocket                    │  │
│  │     - outputs: { type, data, metadata }      │  │
│  └──────────────────────────────────────────────┘  │
│                      │                               │
│                      ▼                               │
│  ┌──────────────────────────────────────────────┐  │
│  │  3. Store outputs to artifact service        │  │
│  │     - S3, local storage, or CDN              │  │
│  │     - Returns: URL to artifact               │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────┐
│            Update Automerge Document                 │
│                                                      │
│  cell.executionCount += 1                            │
│  cell.outputRefs = [outputUrl]                       │
│                                                      │
│  Syncs to all collaborators via WebSocket            │
└─────────────────────────────────────────────────────┘
```

## Implementation Steps

### 1. Set Up Runtimed Backend

Install runtimed via cargo:

```bash
cargo install runtimed
```

Or add to your project:

```toml
[dependencies]
runtimed = "0.1"  # Check latest version
```

Start a runtimed server:

```bash
runtimed serve --port 8888
```

### 2. Frontend Integration

Update `frontend/src/App.tsx` executeCell function:

```typescript
const executeCell = async (index: number) => {
  if (!docHandle) return;

  const cell = doc?.cells[index];
  if (!cell || toStr(cell.cellType) !== "code") return;

  try {
    // 1. Send to runtimed
    const response = await fetch('http://localhost:8888/execute', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        source: toStr(cell.source),
        kernel: 'python',  // or 'javascript', 'rust', etc.
      }),
    });

    const result = await response.json();

    // 2. Store outputs to artifact service
    // For demo, we can use a data URL or local storage
    const outputBlob = new Blob([JSON.stringify(result.outputs)], {
      type: 'application/json'
    });

    // In production, upload to S3/CDN and get URL
    const outputUrl = URL.createObjectURL(outputBlob);

    // Or use a real artifact service:
    // const outputUrl = await uploadToArtifactService(result.outputs);

    // 3. Update CRDT
    docHandle.change((d: Doc) => {
      if (!d.cells || index >= d.cells.length) return;
      const cell = d.cells[index];

      cell.executionCount = (cell.executionCount || 0) + 1;
      cell.outputRefs = [outputUrl];

      if (!d.notebookMetadata) d.notebookMetadata = {};
      d.notebookMetadata.lastModified = Date.now();
    });

  } catch (error) {
    console.error('Execution failed:', error);
    // Handle error - could add error output to cell
  }
};
```

### 3. Rust CLI Integration

Update `cli/src/main.rs` ExecuteCell command:

```rust
Command::ExecuteCell { index } => {
    if *index < state.cells.len() {
        let cell = &mut state.cells[*index];

        if cell.cellType != "code" {
            println!("⚠️  Can only execute code cells");
            return Ok(());
        }

        // 1. Call runtimed
        let client = reqwest::Client::new();
        let response = client
            .post("http://localhost:8888/execute")
            .json(&serde_json::json!({
                "source": cell.source,
                "kernel": "python",
            }))
            .send()
            .await?;

        let result: ExecutionResult = response.json().await?;

        // 2. Upload outputs to artifact service
        let output_url = upload_to_artifact_service(&result.outputs).await?;

        // 3. Update cell
        cell.executionCount = Some(cell.executionCount.unwrap_or(0) + 1);
        cell.outputRefs = vec![output_url.clone()];

        state.notebookMetadata.lastModified = Some(chrono::Utc::now().timestamp_millis());

        println!("✓ Executed cell {} - Output: {}", index, output_url);
    }
}
```

### 4. Artifact Service Options

#### Option A: Local File System

```rust
async fn upload_to_artifact_service(outputs: &[Output]) -> Result<String> {
    let output_id = format!("output-{}", chrono::Utc::now().timestamp_millis());
    let path = format!("./outputs/{}.json", output_id);

    std::fs::create_dir_all("./outputs")?;
    std::fs::write(&path, serde_json::to_string_pretty(outputs)?)?;

    Ok(format!("file://{}", std::fs::canonicalize(&path)?.display()))
}
```

#### Option B: S3/CloudFlare R2

```rust
use aws_sdk_s3::Client;

async fn upload_to_artifact_service(outputs: &[Output]) -> Result<String> {
    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let output_id = format!("output-{}", chrono::Utc::now().timestamp_millis());
    let key = format!("notebook-outputs/{}.json", output_id);

    client
        .put_object()
        .bucket("my-notebook-artifacts")
        .key(&key)
        .body(serde_json::to_vec(outputs)?.into())
        .send()
        .await?;

    Ok(format!("https://my-cdn.com/{}", key))
}
```

#### Option C: Data URLs (Simple, but not scalable)

```typescript
const dataUrl = `data:application/json;base64,${btoa(JSON.stringify(outputs))}`;
cell.outputRefs = [dataUrl];
```

### 5. Output Rendering

The `Output.tsx` component already supports multiple MIME types. Extend it to fetch from URLs:

```typescript
// In NotebookCell.tsx
useEffect(() => {
  const loadOutputs = async () => {
    if (!cell.outputRefs || cell.outputRefs.length === 0) {
      setOutputs([]);
      return;
    }

    const outputs = await Promise.all(
      cell.outputRefs.map(async (ref) => {
        const url = typeof ref === "string" ? ref : ref.toString();

        if (url.startsWith('data:')) {
          // Data URL - parse directly
          const base64 = url.split(',')[1];
          return JSON.parse(atob(base64));
        } else {
          // Fetch from URL
          const response = await fetch(url);
          return response.json();
        }
      })
    );

    setOutputs(outputs);
  };

  loadOutputs();
}, [cell.outputRefs]);
```

## Runtimed Kernel Configuration

Configure which kernels are available:

```rust
// In runtimed setup
runtimed::KernelManager::new()
    .add_kernel("python", PythonKernel::new())
    .add_kernel("javascript", DenoKernel::new())
    .add_kernel("rust", RustKernel::new())
    .start()
```

## Security Considerations

1. **Sandboxing**: Runtimed should run code in isolated containers
2. **Resource Limits**: Set CPU/memory limits per execution
3. **Timeout**: Kill long-running executions
4. **Authentication**: Require auth for execution endpoints
5. **Output Size**: Limit output size to prevent CRDT bloat

## Future Enhancements

- [ ] Support for multiple kernel types (Python, JS, Rust, etc.)
- [ ] Streaming execution updates (progress bars)
- [ ] Interrupt/cancel cell execution
- [ ] Cell dependencies and automatic re-execution
- [ ] Rich output types (plots, interactive widgets)
- [ ] Kernel session management (variables persist across cells)
- [ ] Share kernels across collaborators
- [ ] Export notebooks to .ipynb format

## Testing the Integration

1. Start runtimed: `runtimed serve`
2. Start sync server: `pnpx @automerge/automerge-repo-sync-server`
3. Start frontend: `cd frontend && npm run dev`
4. Open browser and create cells
5. Execute Python code:
   ```python
   import sys
   print(f"Hello from Python {sys.version}")
   ```

## References

- [Runtimed GitHub](https://github.com/runtimed/runtimed)
- [Jupyter Protocol](https://jupyter-client.readthedocs.io/en/stable/messaging.html)
- [Automerge Documentation](https://automerge.org/docs/)
