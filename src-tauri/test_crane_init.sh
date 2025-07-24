#!/bin/bash

echo "Testing Crane initialization..."
cd /Users/nnn-gif/sandbox/src/github.com/nnn-gif/personalassistant

# Kill any existing processes
lsof -ti:5173 | xargs kill -9 2>/dev/null || true

# Start the app with Crane backend and capture logs
RUST_LOG=info CANDLE_MODEL_ID="Qwen/Qwen2.5-3B-Instruct" INFERENCE_PROVIDER="crane" npm run tauri dev 2>&1 | tee crane_logs.txt &

# Wait for initialization
echo "Waiting for app to start..."
sleep 15

# Check logs
echo
echo "=== Crane Initialization Logs ==="
grep -E "\[CraneBackend\]|\[LLM\]|Error|error" crane_logs.txt | head -50

# Kill the app
pkill -f "tauri dev" 2>/dev/null || true
lsof -ti:5173 | xargs kill -9 2>/dev/null || true