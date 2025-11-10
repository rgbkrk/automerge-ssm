#!/bin/bash

# Automerge SSM Demo Launcher
# This script starts both the Rust server and the frontend

set -e

echo "🚀 Starting Automerge SSM Demo"
echo "================================"
echo ""

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Cargo is not installed. Please install Rust first."
    echo "Visit: https://rustup.rs/"
    exit 1
fi

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    echo "❌ Error: npm is not installed. Please install Node.js first."
    echo "Visit: https://nodejs.org/"
    exit 1
fi

# Function to cleanup background processes on exit
cleanup() {
    echo ""
    echo "🛑 Shutting down..."
    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
    fi
    if [ ! -z "$FRONTEND_PID" ]; then
        kill $FRONTEND_PID 2>/dev/null || true
    fi
    exit 0
}

trap cleanup INT TERM

# Build and start the Rust server
echo "🦀 Building Rust server..."
cd server
cargo build --release 2>&1 | grep -v "Compiling\|Finished\|Running" || true

echo "🦀 Starting Rust server on ws://localhost:3030..."
cargo run --release 2>&1 &
SERVER_PID=$!
cd ..

# Give the server time to start
sleep 2

# Check if server is running
if ! ps -p $SERVER_PID > /dev/null; then
    echo "❌ Error: Server failed to start"
    exit 1
fi

echo "✅ Server is running (PID: $SERVER_PID)"
echo ""

# Start the frontend
echo "⚛️  Starting frontend..."
cd frontend

# Install dependencies if node_modules doesn't exist
if [ ! -d "node_modules" ]; then
    echo "📦 Installing frontend dependencies..."
    npm install
fi

echo "⚛️  Starting Vite dev server..."
npm run dev &
FRONTEND_PID=$!
cd ..

sleep 2

echo ""
echo "================================"
echo "✨ Demo is ready!"
echo "================================"
echo ""
echo "🌐 Frontend: http://localhost:5173"
echo "🔌 WebSocket Server: ws://localhost:3030"
echo ""
echo "Press Ctrl+C to stop both services"
echo ""

# Wait for both processes
wait $SERVER_PID $FRONTEND_PID
