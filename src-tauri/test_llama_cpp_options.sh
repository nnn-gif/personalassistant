#!/bin/bash

echo "Checking available llama.cpp Rust crates..."
echo

# Search for llama-related crates
echo "=== Searching crates.io for llama.cpp bindings ==="
cargo search llama | head -20

echo
echo "=== Searching for llm crate (Rust native with Metal support) ==="
cargo search llm | grep -E "^llm " | head -5

echo
echo "=== Searching for GGML/GGUF support ==="
cargo search ggml | head -10

echo
echo "=== Popular options ==="
echo "1. llm - Pure Rust implementation with Metal support"
echo "2. kalosm-llama - Modern llama.cpp bindings" 
echo "3. llama-cpp-2 - Updated bindings to llama.cpp"
echo "4. Build custom FFI bindings to llama.cpp"

echo
echo "=== Checking if llama.cpp is installed ==="
if command -v llama-cli &> /dev/null; then
    echo "✓ llama.cpp CLI found at: $(which llama-cli)"
else
    echo "✗ llama.cpp CLI not found"
    echo "  To install: brew install llama.cpp"
fi