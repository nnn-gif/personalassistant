#!/bin/bash

echo "Testing Crane backend with CPU only..."

# Kill any existing processes
echo "Cleaning up existing processes..."
lsof -ti:5173 | xargs kill -9 2>/dev/null || true
pkill -f "personalassistant" 2>/dev/null || true

# Set environment variables
export RUST_LOG=info
export CANDLE_MODEL_ID="TinyLlama/TinyLlama-1.1B-Chat-v1.0"
export INFERENCE_PROVIDER="crane"
export CANDLE_USE_METAL=0  # Force CPU mode

echo "Configuration:"
echo "  Model: $CANDLE_MODEL_ID"
echo "  Provider: $INFERENCE_PROVIDER"
echo "  Metal: Disabled (CPU only)"
echo

# Create a test script
cat > test_crane_api.sh << 'EOF'
#!/bin/bash

# Wait for the app to start
echo "Waiting for app to start..."
sleep 10

# Test the chat API
echo "Testing chat API..."
curl -X POST http://localhost:3000/api/chat \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "What is the capital of France?",
    "max_tokens": 50
  }' | jq

echo
echo "Test completed!"
EOF

chmod +x test_crane_api.sh

# Start the app in background
cd /Users/nnn-gif/sandbox/src/github.com/nnn-gif/personalassistant
npm run tauri dev &
APP_PID=$!

# Run the test
./test_crane_api.sh

# Kill the app
kill $APP_PID 2>/dev/null || true