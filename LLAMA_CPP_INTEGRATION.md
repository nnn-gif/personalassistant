# Integrating llama.cpp for Full Metal Support

## Available Rust Crates

### 1. llm (Most Popular)
```toml
[dependencies]
llm = "0.1"
```
- Rust-native implementation inspired by llama.cpp
- Supports GGUF format
- Has Metal support

### 2. llama-cpp-rs
```toml
[dependencies]
llama-cpp-rs = "0.3"
```
- Direct bindings to llama.cpp
- Full Metal support
- Requires llama.cpp to be installed

### 3. llama-rs 
```toml
[dependencies]
llama-rs = "0.1"
```
- Pure Rust implementation
- GGML/GGUF support
- Metal acceleration

### 4. whisper-rs (Example of successful Metal integration)
```toml
[dependencies]
whisper-rs = "0.11"
```
- Uses similar approach for Metal support
- Good reference implementation

## Building Your Own FFI Bindings

### Step 1: Build llama.cpp with Metal
```bash
git clone https://github.com/ggerganov/llama.cpp
cd llama.cpp
make LLAMA_METAL=1
```

### Step 2: Create Rust Bindings
```rust
// build.rs
fn main() {
    cc::Build::new()
        .cpp(true)
        .file("llama.cpp/llama.cpp")
        .flag("-DGGML_USE_METAL")
        .compile("llama");
        
    println!("cargo:rustc-link-lib=framework=Metal");
    println!("cargo:rustc-link-lib=framework=MetalPerformanceShaders");
}
```

### Step 3: FFI Wrapper
```rust
use std::ffi::{CString, CStr};

#[link(name = "llama")]
extern "C" {
    fn llama_backend_init(numa: bool);
    fn llama_model_load_from_file(
        path: *const c_char,
        params: llama_model_params,
    ) -> *mut llama_model;
}

pub struct LlamaModel {
    ptr: *mut llama_model,
}

impl LlamaModel {
    pub fn load(path: &str) -> Result<Self> {
        unsafe {
            llama_backend_init(false);
            let c_path = CString::new(path)?;
            let params = llama_model_params {
                n_gpu_layers: 99, // Use Metal for all layers
                ..Default::default()
            };
            let ptr = llama_model_load_from_file(c_path.as_ptr(), params);
            if ptr.is_null() {
                return Err("Failed to load model");
            }
            Ok(Self { ptr })
        }
    }
}
```

## Advantages of llama.cpp

1. **Complete Metal Support**
   - All operations implemented including layer_norm and rms_norm
   - Optimized Metal kernels
   - Excellent performance on Apple Silicon

2. **Wide Model Support**
   - Supports all GGUF models
   - Various quantization formats
   - Continuous updates for new models

3. **Production Ready**
   - Used by Ollama, LM Studio, and many others
   - Battle-tested code
   - Active development

## Integration Strategy

1. **Keep Candle for Future**
   - Once Metal support improves, Candle will be more Rust-idiomatic
   - Good for pure Rust ecosystem

2. **Add llama.cpp Backend**
   - Immediate Metal support
   - Full performance on Apple Silicon
   - Proven reliability

3. **User Choice**
   ```rust
   pub enum InferenceBackend {
       Candle,      // Pure Rust, CPU only for now
       Crane,       // Candle-based, CPU only for now  
       LlamaCpp,    // Full Metal support
   }
   ```

## Example Integration

```toml
# Cargo.toml
[dependencies]
llm = { version = "0.1", features = ["metal"] }
# or
llama-cpp-2 = "0.1"  # Newer crate with better API
```

```rust
use llm::{Model, ModelParameters, ModelKVMemoryType};

pub async fn create_metal_model(model_path: &Path) -> Result<Box<dyn Model>> {
    let params = ModelParameters {
        use_gpu: true,
        gpu_layers: Some(99), // All layers on Metal
        ..Default::default()
    };
    
    let model = llm::load_dynamic(
        Some(model_path),
        llm::ModelKVMemoryType::Float16,
        params,
    )?;
    
    Ok(model)
}
```

This approach gives you:
- Immediate Metal support
- No dependency on Ollama
- Full control over the inference
- Proven performance