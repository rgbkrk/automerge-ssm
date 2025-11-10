# Rust CLI Client for Automerge

A command-line tool written in Rust that connects to the WebSocket server and modifies Automerge documents. This lets you see real-time collaboration between Rust and JavaScript clients!

## What It Does

The Rust CLI client:
- Connects to your WebSocket server (`ws://localhost:3030`)
- Loads or creates an Automerge document by ID
- Applies changes (increment counter, add notes, add users, etc.)
- Sends the updated document to the server
- The server broadcasts changes to all connected browser clients
- You see the changes appear in real-time in your browser! 🎉

## Quick Start

### 1. Start the Server

```bash
cd server
cargo run
```

### 2. Open the Frontend

```bash
cd frontend
npm run dev
```

Open `http://localhost:5173` in your browser and note the document ID in the URL:
```
http://localhost:5173/#automerge:2mdM9TnM2sJgLhHhYjyBzfusSsyr
                                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                                  This is your document ID
```

### 3. Use the Rust Client

**Easy way (using the wrapper script):**
```bash
./rust-client.sh 2mdM9TnM2sJgLhHhYjyBzfusSsyr increment
```

**Direct way:**
```bash
cd server
cargo run --bin cli_client -- 2mdM9TnM2sJgLhHhYjyBzfusSsyr increment
```

Watch your browser - the counter will increment in real-time! 🚀

## Commands

### Show Document State
```bash
./rust-client.sh <doc-id>
# or
./rust-client.sh <doc-id> show
```

Displays the current state without making changes.

### Increment Counter
```bash
./rust-client.sh <doc-id> increment
```

Adds 1 to the counter. Watch it update in all connected browsers!

### Decrement Counter
```bash
./rust-client.sh <doc-id> decrement
```

Subtracts 1 from the counter.

### Set Counter to Specific Value
```bash
./rust-client.sh <doc-id> set-counter 42
```

Sets the counter to exactly 42 (or any number you choose).

### Add a Note
```bash
./rust-client.sh <doc-id> add-note "Message from Rust!"
```

Appends text to the shared notes field. Great for testing text sync.

### Add a Collaborator
```bash
./rust-client.sh <doc-id> add-user "RustBot"
```

Adds a username to the collaborators list.

## Complete Examples

### Example 1: Testing Counter Sync

**Terminal 1 - Start Server:**
```bash
cd server && cargo run
```

**Terminal 2 - Start Frontend:**
```bash
cd frontend && npm run dev
```

**Browser:**
- Open `http://localhost:5173`
- Enter username "Alice"
- Note the document ID: `2mdM9TnM2sJgLhHhYjyBzfusSsyr`

**Terminal 3 - Run Rust Client:**
```bash
./rust-client.sh 2mdM9TnM2sJgLhHhYjyBzfusSsyr increment
```

**Result:** Counter increments in browser instantly! ✨

### Example 2: Multi-User Collaboration

```bash
# Browser: Alice opens document and clicks counter to 5

# Rust client increments
./rust-client.sh <doc-id> increment
# Browser now shows 6

# Browser: Alice clicks increment
# Now shows 7

# Rust client adds note
./rust-client.sh <doc-id> add-note "Rust was here!"
# Browser shows the note appear in real-time

# Rust client adds itself as collaborator
./rust-client.sh <doc-id> add-user "RustBot"
# Browser shows "RustBot" in collaborators list
```

### Example 3: Testing Conflict Resolution

Open document in two browser tabs and use Rust client simultaneously:

```bash
# Tab 1: Click increment (counter = 1)
# Tab 2: Click increment (counter = 2)
# Rust: ./rust-client.sh <doc-id> increment
# All three see counter = 3 (all increments preserved!)
```

This demonstrates Automerge's CRDT conflict resolution in action.

## Understanding the Output

When you run a command, you'll see:

```
🔌 Connecting to ws://localhost:3030...
📡 Connected! Document ID: 2mdM9TnM2sJgLhHhYjyBzfusSsyr
✅ Incremented counter: 5 → 6

📄 Document State:
  Counter: 6
  Notes: Hello from browser!
Message from Rust!
  Collaborators:
    - Alice
    - RustBot

📤 Sending changes to server (262 bytes)...
⏳ Waiting for server response...
✅ Received updated document from server

📄 Document State:
  Counter: 6
  Notes: Hello from browser!
Message from Rust!
  Collaborators:
    - Alice
    - RustBot

✨ Done!
```

## How It Works

### The Protocol

The client uses a simple binary protocol over WebSocket:

```
[36 bytes: document ID padded] + [Automerge document bytes]
```

### The Flow

1. **Connect** - Opens WebSocket to `ws://localhost:3030`
2. **Load Document** - Creates new or loads existing Automerge doc
3. **Apply Changes** - Makes requested modifications using Automerge API
4. **Send** - Serializes document and sends to server
5. **Server Broadcasts** - Server sends to all connected clients
6. **Receive** - Gets updated document back from server
7. **Display** - Shows final state

### CRDT Magic

All changes merge automatically! Whether from browser or Rust:
- Counter increments add up (5 + 1 + 1 = 7)
- Array additions preserve all items
- Text changes merge intelligently
- No conflicts, no data loss

## Advanced Usage

### Direct Cargo Command

```bash
cd server
cargo run --bin cli_client -- <doc-id> <command> [args]
```

### Building Release Version

For faster execution:

```bash
cd server
cargo build --release --bin cli_client
./target/release/cli_client <doc-id> increment
```

### Using from Scripts

```bash
#!/bin/bash
DOC_ID="2mdM9TnM2sJgLhHhYjyBzfusSsyr"

# Increment counter 10 times
for i in {1..10}; do
    ./rust-client.sh $DOC_ID increment
done

# Add a summary note
./rust-client.sh $DOC_ID add-note "Incremented 10 times from script"
```

## Troubleshooting

### "Error: Failed to connect to WebSocket server"

The server isn't running. Start it:
```bash
cd server && cargo run
```

### "Error: WebSocket server is not running on port 3030"

Same issue - make sure server is running on port 3030.

### Changes don't appear in browser

1. Check that browser is connected (look for WebSocket status in DevTools)
2. Try refreshing the browser
3. Verify you're using the correct document ID

### "Timeout waiting for response"

This is normal - the changes were still sent. The timeout just means the server didn't respond within 2 seconds.

### Invalid document ID

Make sure you copied the ID correctly from the browser URL hash:
- Correct: `2mdM9TnM2sJgLhHhYjyBzfusSsyr`
- Wrong: `automerge:2mdM9TnM2sJgLhHhYjyBzfusSsyr` (don't include "automerge:")
- Wrong: `#automerge:2mdM9TnM2sJgLhHhYjyBzfusSsyr` (don't include "#")

## Code Structure

The CLI client is in `server/src/bin/cli_client.rs`:

- **Command parsing** - Interprets command-line arguments
- **Document operations** - Uses `automerge` crate to modify docs
- **WebSocket client** - Connects via `tokio-tungstenite`
- **Protocol handling** - Formats messages for server

## Extending the Client

Want to add new commands? Edit `server/src/bin/cli_client.rs`:

```rust
enum Command {
    Increment,
    Decrement,
    // Add your command here:
    ClearNotes,
}

async fn execute_command(doc: &mut AutoCommit, command: &Command) -> Result<()> {
    match command {
        // ... existing commands ...
        Command::ClearNotes => {
            doc.put(automerge::ROOT, "notes", "")?;
            info!("✅ Cleared notes");
        }
    }
    Ok(())
}
```

## Tips & Tricks

### Quick Testing Loop

Keep this running while developing:
```bash
watch -n 1 './rust-client.sh <doc-id> show'
```

Shows document state every second.

### Batch Operations

```bash
# Set up a test document
DOC_ID="your-doc-id"
./rust-client.sh $DOC_ID set-counter 0
./rust-client.sh $DOC_ID add-user "Alice"
./rust-client.sh $DOC_ID add-user "Bob"
./rust-client.sh $DOC_ID add-note "Test session started"
```

### Stress Testing

```bash
# See how many increments per second
for i in {1..100}; do
    ./rust-client.sh $DOC_ID increment &
done
wait
```

All increments will be preserved due to CRDT properties!

## Next Steps

- Try modifying multiple documents simultaneously
- Build a Rust service that watches for changes
- Implement more complex document operations
- Add authentication/authorization
- Create a TUI (terminal UI) client

## Related Files

- `server/src/bin/cli_client.rs` - CLI client implementation
- `rust-client.sh` - Convenience wrapper script
- `server/src/main.rs` - WebSocket server
- `server/examples/simple_client.rs` - Standalone Automerge examples

## Resources

- [Automerge Rust Docs](https://docs.rs/automerge)
- [tokio-tungstenite](https://docs.rs/tokio-tungstenite) - WebSocket library
- Main project [README.md](README.md)
- [DEMO.md](DEMO.md) - Full architecture guide

---

**Happy hacking!** 🦀✨