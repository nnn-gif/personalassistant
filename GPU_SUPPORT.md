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
- **Technology**: CUDA and Vulkan
- **Requirements**: 
  - NVIDIA GPU with CUDA support (for CUDA acceleration)
  - Any GPU with Vulkan support (for Vulkan acceleration)
- **Environment Variable**: `LLAMA_CUDA_FORCE_DISABLE` (set to "1" to disable)
- **Status**: ✅ Implemented (requires testing on Windows hardware)

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
- **Windows**: Set environment variable `LLAMA_CUDA_FORCE_DISABLE=1`

## Build Configuration

The `Cargo.toml` has been updated with platform-specific dependencies:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
llama_cpp = { version = "0.3", features = ["metal"] }

[target.'cfg(target_os = "windows")'.dependencies]
llama_cpp = { version = "0.3", features = ["cuda", "vulkan"] }

[target.'cfg(not(any(target_os = "macos", target_os = "windows")))'.dependencies]
llama_cpp = { version = "0.3" }
```

## Testing

Run the GPU detection tests:
```bash
cargo test test_llama_gpu
```

## Troubleshooting

### Windows GPU Not Detected
1. Ensure NVIDIA drivers are installed (for CUDA)
2. Check that CUDA toolkit is installed
3. Verify Vulkan runtime is available
4. Check environment variables

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