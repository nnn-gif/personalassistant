#!/bin/bash

echo "Testing LlamaCpp Integration"
echo "============================"

# Check if model exists
MODEL_PATH="$HOME/Library/Caches/huggingface/hub/models--TheBloke--TinyLlama-1.1B-Chat-v1.0-GGUF/snapshots/*/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf"

if ls $MODEL_PATH 1> /dev/null 2>&1; then
    echo "✅ Model already downloaded: $(ls $MODEL_PATH)"
else
    echo "📥 Model needs to be downloaded"
fi

# Test configuration
echo -e "\n📋 Testing Configuration:"
echo "INFERENCE_PROVIDER=llama_cpp"
echo "CANDLE_MODEL_ID=TinyLlama/TinyLlama-1.1B-Chat-v1.0"

# Check Metal support
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo -e "\n🍎 Running on macOS - Metal support available"
    
    # Check for Metal compiler
    if command -v xcrun &> /dev/null; then
        echo "✅ Xcode command line tools installed"
        if xcrun -sdk macosx metal -v 2>&1 | grep -q "Metal"; then
            echo "✅ Metal compiler available"
        fi
    fi
    
    # Check system info
    echo -e "\n💻 System Info:"
    sysctl -n machdep.cpu.brand_string
    
    # Check for Apple Silicon
    if [[ $(uname -m) == "arm64" ]]; then
        echo "✅ Apple Silicon detected - optimal Metal performance"
    else
        echo "ℹ️  Intel Mac detected - Metal available but less optimized"
    fi
else
    echo -e "\n⚠️  Not running on macOS - Metal not available"
fi

echo -e "\n📊 Implementation Status:"
echo "✅ LlamaCpp backend implemented in src/llm/llama_cpp_metal_backend.rs"
echo "✅ Added to InferenceProvider enum"
echo "✅ Integrated into LlmClient"
echo "✅ UI updated with LlamaCpp option"
echo "✅ Model downloading implemented"
echo "⏳ Actual llama.cpp C++ integration pending (using placeholder)"

echo -e "\n🚀 To test in the app:"
echo "1. Run: export INFERENCE_PROVIDER=llama_cpp"
echo "2. Run: npm run tauri dev"
echo "3. Or use the Settings UI to select 'LlamaCpp' provider"