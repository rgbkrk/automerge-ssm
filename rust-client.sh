#!/bin/bash

# Convenience wrapper for the Rust CLI client
# This makes it easier to interact with documents from the command line

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVER_DIR="$SCRIPT_DIR/server"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_usage() {
    echo "🦀 Automerge Rust CLI Client"
    echo ""
    echo "Usage: $0 <document-id> [command] [args...]"
    echo ""
    echo "Commands:"
    echo "  increment              - Increment the counter by 1"
    echo "  decrement              - Decrement the counter by 1"
    echo "  set-counter <n>        - Set counter to specific value"
    echo "  add-note <text>        - Add text to notes"
    echo "  add-user <name>        - Add a collaborator"
    echo "  show                   - Display current document state (default)"
    echo ""
    echo "Examples:"
    echo "  # Show document state"
    echo "  $0 2mdM9TnM2sJgLhHhYjyBzfusSsyr"
    echo ""
    echo "  # Increment counter"
    echo "  $0 2mdM9TnM2sJgLhHhYjyBzfusSsyr increment"
    echo ""
    echo "  # Add a note"
    echo "  $0 2mdM9TnM2sJgLhHhYjyBzfusSsyr add-note \"Hello from Rust!\""
    echo ""
    echo "  # Add a collaborator"
    echo "  $0 2mdM9TnM2sJgLhHhYjyBzfusSsyr add-user \"RustUser\""
    echo ""
    echo "Note: Extract document ID from browser URL hash:"
    echo "  http://localhost:5173/#automerge:2mdM9TnM2sJgLhHhYjyBzfusSsyr"
    echo "                                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^"
    echo "  Just copy the part after 'automerge:'"
    echo ""
}

# Check if help is requested
if [ "$1" = "-h" ] || [ "$1" = "--help" ] || [ $# -lt 1 ]; then
    print_usage
    exit 0
fi

# Check if server is running
if ! lsof -i :3030 >/dev/null 2>&1; then
    echo -e "${RED}❌ Error: WebSocket server is not running on port 3030${NC}"
    echo -e "${YELLOW}Start the server first:${NC}"
    echo "  cd server && cargo run"
    exit 1
fi

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Error: Cargo is not installed${NC}"
    echo "Install Rust from: https://rustup.rs/"
    exit 1
fi

echo -e "${BLUE}🦀 Running Rust CLI client...${NC}"
echo ""

# Change to server directory and run the client
cd "$SERVER_DIR"
cargo run --quiet --bin cli_client -- "$@"

exit_code=$?

if [ $exit_code -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✨ Success!${NC}"
    echo ""
    echo -e "${YELLOW}💡 Tip:${NC} Check your browser to see the changes reflected in real-time!"
else
    echo ""
    echo -e "${RED}❌ Command failed with exit code $exit_code${NC}"
fi

exit $exit_code
