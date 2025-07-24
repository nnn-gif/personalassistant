#!/bin/bash

echo "=== Diagnosing Crane Backend Issues ==="
echo

# Check if GGUF file exists
echo "1. Checking GGUF files..."
CACHE_DIR="$HOME/.cache/huggingface/hub"

echo "Qwen 3B GGUF:"
ls -la "$CACHE_DIR/models--Qwen--Qwen2.5-3B-Instruct-GGUF/snapshots/"*/qwen2.5-3b-instruct-q4_k_m.gguf 2>/dev/null || echo "  Not found"

echo
echo "2. Checking tokenizers..."
echo "Qwen 3B tokenizer:"
ls -la "$CACHE_DIR/models--Qwen--Qwen2.5-3B-Instruct/snapshots/"*/tokenizer.json 2>/dev/null || echo "  Not found"

echo
echo "3. Running app with debug logging..."
cd src-tauri
RUST_LOG=debug CANDLE_MODEL_ID="Qwen/Qwen2.5-3B-Instruct" INFERENCE_PROVIDER="crane" cargo run --release 2>&1 | grep -E "CraneBackend|ERROR|error" | head -100