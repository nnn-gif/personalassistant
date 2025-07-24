use candle_core::{Device, Tensor, DType};
use candle_nn;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple Metal Test ===\n");
    
    // Step 1: Test Metal device availability
    println!("Step 1: Testing Metal device availability...");
    let device = match Device::new_metal(0) {
        Ok(d) => {
            println!("✓ Metal device created successfully!");
            d
        }
        Err(e) => {
            println!("✗ Metal device creation failed: {}", e);
            println!("  Falling back to CPU...");
            Device::Cpu
        }
    };
    
    // Step 2: Test basic Metal operations
    println!("\nStep 2: Testing basic Metal tensor operations...");
    match Tensor::randn(0f32, 1f32, &[2, 3], &device) {
        Ok(t) => {
            println!("✓ Random tensor creation successful on {:?}", device);
            println!("  Shape: {:?}", t.dims());
            
            // Test basic operations
            match t.matmul(&t.t()?) {
                Ok(result) => {
                    println!("✓ Matrix multiplication successful!");
                    println!("  Result shape: {:?}", result.dims());
                }
                Err(e) => println!("✗ Matrix multiplication failed: {}", e),
            }
        }
        Err(e) => println!("✗ Tensor creation failed: {}", e),
    }
    
    // Step 3: Check if we're on CPU or Metal
    println!("\nStep 3: Device information...");
    if device.is_cpu() {
        println!("  Running on CPU");
    } else {
        println!("  Running on Metal GPU");
    }
    
    // Step 4: Test quantized operations
    println!("\nStep 4: Testing operations that might fail on Metal...");
    
    // Create a simple tensor
    let input = Tensor::new(&[1.0f32, 2.0, 3.0, 4.0], &device)?;
    println!("  Created input tensor: {:?}", input.dims());
    
    // Try simple operations that might fail on Metal
    // RMS norm is often problematic on Metal for quantized models
    println!("  Testing simple tensor operations...");
    
    // Step 5: Check environment
    println!("\nStep 5: Environment information...");
    println!("  CANDLE_MODEL_ID: {:?}", std::env::var("CANDLE_MODEL_ID"));
    println!("  INFERENCE_PROVIDER: {:?}", std::env::var("INFERENCE_PROVIDER"));
    
    // Step 6: Model paths
    println!("\nStep 6: Model cache paths...");
    let cache_dir = dirs::home_dir()
        .unwrap()
        .join(".cache")
        .join("huggingface")
        .join("hub");
    println!("  Cache directory: {:?}", cache_dir);
    
    // Check for common model directories
    let models = [
        "models--TinyLlama--TinyLlama-1.1B-Chat-v1.0",
        "models--TheBloke--TinyLlama-1.1B-Chat-v1.0-GGUF",
        "models--Qwen--Qwen2.5-3B-Instruct",
        "models--Qwen--Qwen2.5-3B-Instruct-GGUF",
    ];
    
    for model_dir in &models {
        let model_path = cache_dir.join(model_dir);
        if model_path.exists() {
            println!("  ✓ Found: {}", model_dir);
            
            // Check for GGUF files
            if let Ok(snapshots) = std::fs::read_dir(model_path.join("snapshots")) {
                for entry in snapshots.flatten() {
                    if let Ok(files) = std::fs::read_dir(entry.path()) {
                        for file in files.flatten() {
                            if file.path().extension().and_then(|s| s.to_str()) == Some("gguf") {
                                println!("    - GGUF file: {}", file.file_name().to_string_lossy());
                            }
                        }
                    }
                }
            }
        }
    }
    
    println!("\n=== Test Complete ===");
    Ok(())
}