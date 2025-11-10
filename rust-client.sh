#!/bin/bash

# Wrapper script for the Rust CLI client
# Usage: ./rust-client.sh <automerge-url> [command] [args...]
#
# Example:
#   ./rust-client.sh automerge:2mdM9TnM2sJgLhHhYjyBzfusSsyr increment
#   ./rust-client.sh automerge:2mdM9TnM2sJgLhHhYjyBzfusSsyr add-note "Hello from Rust!"

if [ $# -eq 0 ]; then
    echo "Usage: $0 <automerge-url> [command] [args...]"
    echo ""
    echo "The automerge-url should be the full URL from your browser:"
    echo "  automerge:4VgLSsiuVNfWeZk17m85GgA18VVp"
    echo ""
    echo "Commands:"
    echo "  increment              - Increment the counter by 1"
    echo "  decrement              - Decrement the counter by 1"
    echo "  set-counter <n>        - Set counter to specific value"
    echo "  add-note <text>        - Add text to notes"
    echo "  add-user <name>        - Add a collaborator"
    echo "  show                   - Display current document state (default)"
    echo ""
    echo "Example:"
    echo "  $0 automerge:4VgLSsiuVNfWeZk17m85GgA18VVp increment"
    exit 1
fi

cd "$(dirname "$0")/server"
cargo run --release --bin automerge-cli -- "$@"
