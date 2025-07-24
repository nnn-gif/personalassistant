#!/bin/bash

# Create cache directory
CACHE_DIR="$HOME/.cache/huggingface/hub"
mkdir -p "$CACHE_DIR"

# Download TinyLlama files
echo "Downloading TinyLlama model files..."

# Standard model files
MODEL_DIR="$CACHE_DIR/models--TinyLlama--TinyLlama-1.1B-Chat-v1.0/snapshots/main"
mkdir -p "$MODEL_DIR"

cd "$MODEL_DIR"

# Download only essential files to test
echo "Downloading config.json..."
curl -L "https://huggingface.co/TinyLlama/TinyLlama-1.1B-Chat-v1.0/resolve/main/config.json" -o config.json

echo "Downloading tokenizer.json..."
curl -L "https://huggingface.co/TinyLlama/TinyLlama-1.1B-Chat-v1.0/resolve/main/tokenizer.json" -o tokenizer.json

# Download GGUF version
GGUF_DIR="$CACHE_DIR/models--TheBloke--TinyLlama-1.1B-Chat-v1.0-GGUF/snapshots/main"
mkdir -p "$GGUF_DIR"

cd "$GGUF_DIR"

echo "Downloading GGUF model (this may take a while)..."
curl -L "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf" -o tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf

echo "Download complete!"
echo "Files downloaded to:"
echo "  Standard model: $MODEL_DIR"
echo "  GGUF model: $GGUF_DIR"