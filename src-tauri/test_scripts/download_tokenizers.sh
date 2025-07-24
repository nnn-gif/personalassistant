#!/bin/bash

# Download tokenizers for Qwen models
echo "Downloading tokenizers for Qwen models..."

CACHE_DIR="$HOME/.cache/huggingface/hub"

# Download Qwen 3B tokenizer
echo "Downloading Qwen 3B tokenizer..."
MODEL_DIR="$CACHE_DIR/models--Qwen--Qwen2.5-3B-Instruct"
mkdir -p "$MODEL_DIR/snapshots/main"
curl -L "https://huggingface.co/Qwen/Qwen2.5-3B-Instruct/resolve/main/tokenizer.json" \
     -o "$MODEL_DIR/snapshots/main/tokenizer.json"

# Download Qwen 0.5B tokenizer
echo "Downloading Qwen 0.5B tokenizer..."
MODEL_DIR="$CACHE_DIR/models--Qwen--Qwen2.5-0.5B-Instruct"
mkdir -p "$MODEL_DIR/snapshots/main"
curl -L "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct/resolve/main/tokenizer.json" \
     -o "$MODEL_DIR/snapshots/main/tokenizer.json"

# Download Qwen 1.5B tokenizer
echo "Downloading Qwen 1.5B tokenizer..."
MODEL_DIR="$CACHE_DIR/models--Qwen--Qwen2.5-1.5B-Instruct"
mkdir -p "$MODEL_DIR/snapshots/main"
curl -L "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct/resolve/main/tokenizer.json" \
     -o "$MODEL_DIR/snapshots/main/tokenizer.json"

# Download TinyLlama tokenizer
echo "Downloading TinyLlama tokenizer..."
MODEL_DIR="$CACHE_DIR/models--TinyLlama--TinyLlama-1.1B-Chat-v1.0"
mkdir -p "$MODEL_DIR/snapshots/main"
curl -L "https://huggingface.co/TinyLlama/TinyLlama-1.1B-Chat-v1.0/resolve/main/tokenizer.json" \
     -o "$MODEL_DIR/snapshots/main/tokenizer.json"

echo "All tokenizers downloaded!"