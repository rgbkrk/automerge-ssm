#!/bin/bash
set -e  # Exit on error

echo "🔧 Setting up Autodash workspace..."

# Check for required tools
echo "Checking prerequisites..."

if ! command -v node &> /dev/null; then
    echo "❌ Error: Node.js is not installed."
    echo "Please install Node.js from https://nodejs.org/"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Rust is not installed."
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "✅ Node.js and Rust are installed"

# Install frontend dependencies
echo ""
echo "📦 Installing frontend dependencies..."
cd frontend
npm install

# Build Rust CLI
echo ""
echo "🦀 Building Rust CLI..."
cd ../cli
cargo build

echo ""
echo "✅ Workspace setup complete!"
echo ""
echo "📝 Notes:"
echo "  - Sync server should be running on port 3030 (pnpx @automerge/automerge-repo-sync-server)"
echo "  - Use 'Run' button to start the frontend dev server"
echo "  - Git submodules in vendor/ are optional (for source exploration only)"
