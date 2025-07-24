# Metal Inference Fixes Summary

## Issues Found and Fixed

### 1. Metal Support is Working ✓
- Metal device creation works fine on macOS
- Basic tensor operations work on Metal
- The issue was NOT with Metal itself

### 2. Real Issues Were:

#### A. Tokenizer Download Failures
- **Problem**: HuggingFace Hub API failing with "relative URL without a base" error
- **Solution**: Added fallback mechanism in both Crane and Callm backends:
  1. Try downloading via HF Hub API
  2. If fails, check manual path construction
  3. If still fails, try GGUF repo as last resort

#### B. Model Download Detection
- **Problem**: Downloaded models showing as not available
- **Solution**: Fixed cache directory path and added proper symlink handling

#### C. Model Persistence
- **Problem**: Changing model in settings didn't actually change the model being used
- **Solution**: LLM client now reads from environment variables dynamically

#### D. Error Handling
- **Problem**: Backends returning Ok() even when model loading failed
- **Solution**: Improved error handling with CPU fallback for Metal-specific failures

## Metal Limitations with GGUF Models

- GGUF quantized models may lack Metal implementations for some operations (e.g., rms-norm)
- Backends now automatically fallback to CPU when Metal operations fail
- This is a known limitation of quantized models, not a bug

## Files Modified

1. `src/llm/crane_backend.rs` - Added tokenizer fallback mechanism
2. `src/llm/callm_backend.rs` - Added tokenizer fallback mechanism
3. `src/llm/mod.rs` - Fixed model persistence by reading env vars dynamically
4. `src/services/inference.rs` - Fixed model download detection

## Test Scripts Created

1. `test_metal_simple.rs` - Tests basic Metal functionality
2. `debug_crane.rs` - Debug script for model loading issues
3. `download_tokenizers.sh` - Script to manually download tokenizers

## Next Steps

1. The backends should now work properly with Metal (with CPU fallback when needed)
2. Model switching should work correctly
3. Downloaded models should be detected properly

## Usage

To use local inference with Metal acceleration:
1. Select Crane or Callm as inference provider
2. Choose a model (e.g., Qwen 3B, TinyLlama)
3. The backend will use Metal when possible, CPU as fallback