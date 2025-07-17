# Candle Integration for Personal Assistant

This document explains how to use the Hugging Face Candle integration for local inference in the Personal Assistant app.

## Overview

The Personal Assistant now supports two inference providers:
1. **Ollama** - Cloud-based inference (default)
2. **Candle** - Local inference using Hugging Face models

## Current Status

The Candle integration is currently implemented as a **placeholder** that demonstrates the architecture and UI integration. The actual Candle model loading and inference is commented out to avoid build complexity, but the structure is in place for full implementation.

## Using the Inference Settings

1. Navigate to **Settings** in the sidebar
2. Select **Inference** tab
3. Choose between:
   - **Ollama**: Uses the Ollama service for inference
   - **Candle**: Would run models locally on your device

### Candle Models Available (When Fully Implemented)

- **Phi-2 (2.7B)**: Microsoft's small but capable model (~5.5 GB)
- **TinyLlama 1.1B**: Efficient chat model (~2.2 GB)
- **Mistral 7B**: High-quality 7B parameter model (~14 GB)

## Architecture

### Backend Structure

```rust
src-tauri/src/
├── llm/
│   ├── mod.rs              # Main LLM client with provider switching
│   └── candle_backend.rs   # Candle implementation (placeholder)
├── config.rs               # Configuration with InferenceProvider enum
└── services/
    └── inference.rs        # Tauri commands for managing providers
```

### Frontend Components

```typescript
src/components/
├── Settings.tsx            # Main settings page
└── settings/
    └── InferenceSettings.tsx  # Inference provider configuration
```

## Full Implementation Guide

To implement full Candle support:

1. **Enable Candle Dependencies** in `Cargo.toml`:
```toml
candle-core = "0.8"
candle-nn = "0.8"
candle-transformers = "0.8"
tokenizers = "0.20"
hf-hub = { version = "0.3", features = ["tokio"] }
```

2. **Update `candle_backend.rs`** with the actual implementation (see the commented example in the file)

3. **Key Components to Implement**:
   - Model downloading from Hugging Face Hub
   - Model loading with proper device selection (CPU/CUDA)
   - Tokenizer initialization
   - Text generation with sampling strategies
   - Proper error handling for model loading failures

## Configuration

### Environment Variables

- `INFERENCE_PROVIDER`: Set to "candle" or "ollama" (default: "ollama")
- `CANDLE_MODEL_ID`: Hugging Face model ID (default: "microsoft/phi-2")
- `CANDLE_MODEL_REVISION`: Model revision (default: "main")
- `CANDLE_CACHE_DIR`: Where to cache downloaded models

### Config File

You can also configure via `config.toml`:

```toml
[services]
inference_provider = "Candle"
candle_model_id = "microsoft/phi-2"
candle_model_revision = "main"
candle_cache_dir = "~/.cache/personalassistant/models"
```

## Benefits of Candle

1. **Privacy**: All inference runs locally on your device
2. **No Internet Required**: After initial model download
3. **Performance**: Optimized Rust implementation
4. **Flexibility**: Support for various model architectures
5. **Control**: Full control over model selection and configuration

## Current Limitations

1. The current implementation is a placeholder
2. Model downloading is not implemented
3. Actual inference returns placeholder text
4. No GPU acceleration in the placeholder

## Future Enhancements

1. Implement actual model loading and inference
2. Add progress tracking for model downloads
3. Support for more model architectures
4. Quantization support for smaller model sizes
5. Streaming text generation
6. Model caching and management UI

## Testing the Integration

Even with the placeholder implementation, you can:

1. Navigate to Settings → Inference
2. Switch between Ollama and Candle providers
3. Select different Candle models
4. Save configuration (requires app restart)
5. View current provider info in the UI

The UI is fully functional and ready for the complete Candle implementation.