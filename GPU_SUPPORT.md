# GPU Support for LlamaCpp Backend

This document describes the GPU acceleration support for the LlamaCpp inference backend in the Personal Assistant application.

## Overview

The LlamaCpp backend now supports GPU acceleration on both macOS (Metal) and Windows (CUDA/Vulkan), with automatic fallback to Ollama if GPU initialization fails.

## Platform Support

### macOS
- **Technology**: Metal
- **Requirements**: macOS 10.13+ with Metal-capable GPU
- **Environment Variable**: `CANDLE_USE_METAL` (set to "0" to disable)
- **Status**: ✅ Fully implemented and tested

### Windows
- **Technology**: CPU-only (GPU support disabled due to build complexity)
- **Requirements**: None
- **GPU Acceleration**: Available through Ollama fallback
- **Status**: ✅ CPU support works reliably

### Linux/Other Platforms
- **Technology**: CPU-only
- **Status**: ✅ Automatically falls back to CPU mode

## Features

1. **Automatic GPU Detection**: The backend automatically detects the platform and available GPU acceleration
2. **Graceful Fallback**: If GPU initialization fails, the system automatically falls back to Ollama
3. **Performance Monitoring**: Token generation speed is logged for performance analysis
4. **Model Support**: Currently supports TinyLlama and Qwen models in GGUF format

## Configuration

### Using GPU Acceleration

By default, GPU acceleration is enabled on supported platforms. The backend will:
1. Detect the operating system
2. Check for GPU availability
3. Load all model layers to GPU (n_gpu_layers=999)
4. Fall back to CPU or Ollama if GPU initialization fails

### Disabling GPU Acceleration

To force CPU mode:
- **macOS**: Set environment variable `CANDLE_USE_METAL=0`
- **Windows**: CPU mode is the default

## Build Configuration

The `Cargo.toml` has been updated with platform-specific dependencies:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
llama_cpp = { version = "0.3", features = ["metal"] }

[target.'cfg(target_os = "windows")'.dependencies]
llama_cpp = { version = "0.3" }  # CPU-only for reliable builds

[target.'cfg(not(any(target_os = "macos", target_os = "windows")))'.dependencies]
llama_cpp = { version = "0.3" }
```

## Testing

Run the GPU detection tests:
```bash
cargo test test_llama_gpu
```

## Troubleshooting

### Windows GPU Acceleration
Windows users can get GPU acceleration through:
1. Use Ollama with GPU support (recommended)
2. The system will automatically fall back to Ollama if LlamaCpp fails
3. Ollama handles all GPU complexity internally

### macOS Metal Issues
1. Verify macOS version is 10.13+
2. Check that the GPU supports Metal
3. Ensure Xcode command line tools are installed

### Fallback to Ollama
If you see "Falling back to Ollama" in the logs, it means:
1. GPU initialization failed
2. Model download failed
3. Insufficient GPU memory

The system will continue to work using Ollama as the inference backend.