#!/bin/bash

echo "Starting Personal Assistant with Crane backend (TinyLlama)..."

# Kill any existing processes
echo "Cleaning up existing processes..."
lsof -ti:5173 | xargs kill -9 2>/dev/null || true
pkill -f "personalassistant" 2>/dev/null || true

# Set environment variables
export RUST_LOG=info
export CANDLE_MODEL_ID="TinyLlama/TinyLlama-1.1B-Chat-v1.0"
export INFERENCE_PROVIDER="crane"

echo "Configuration:"
echo "  Model: $CANDLE_MODEL_ID"
echo "  Provider: $INFERENCE_PROVIDER"
echo

# Start the app
cd /Users/nnn-gif/sandbox/src/github.com/nnn-gif/personalassistant
npm run tauri dev