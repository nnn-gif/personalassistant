use candle_core::{Device, Tensor, DType};
use candle_transformers::models::quantized_llama::ModelWeights as QLlamaWeights;
use hf_hub::{api::tokio::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Metal Inference Test ===\n");
    
    // Step 1: Test Metal device availability
    println!("Step 1: Testing Metal device availability...");
    let device = match Device::new_metal(0) {
        Ok(d) => {
            println!("✓ Metal device created successfully!");
            println!("  Device: {:?}", d.location());
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
            println!("✓ Random tensor creation successful on {:?}", device.location());
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
    
    // Step 3: Check model availability
    println!("\nStep 3: Checking model availability...");
    let model_id = "Qwen/Qwen2.5-3B-Instruct";
    let cache_dir = dirs::home_dir()
        .unwrap()
        .join(".cache")
        .join("huggingface")
        .join("hub");
    
    println!("  Model: {}", model_id);
    println!("  Cache dir: {:?}", cache_dir);
    
    // Check for GGUF model
    let gguf_repo = "Qwen/Qwen2.5-3B-Instruct-GGUF";
    let gguf_file = "qwen2.5-3b-instruct-q4_k_m.gguf";
    let gguf_repo_dir = cache_dir.join(format!("models--{}", gguf_repo.replace("/", "--")));
    
    println!("\n  Checking GGUF model...");
    println!("  GGUF repo dir: {:?}", gguf_repo_dir);
    
    let mut model_path = None;
    if gguf_repo_dir.exists() {
        println!("✓ GGUF repo directory exists");
        
        // Look for the model file
        let snapshots_dir = gguf_repo_dir.join("snapshots");
        if snapshots_dir.exists() {
            println!("✓ Snapshots directory exists");
            
            for entry in std::fs::read_dir(&snapshots_dir)? {
                let entry = entry?;
                let snapshot_path = entry.path();
                if snapshot_path.is_dir() {
                    let model_file = snapshot_path.join(gguf_file);
                    println!("  Checking: {:?}", model_file);
                    if model_file.exists() || (model_file.is_symlink() && model_file.metadata().is_ok()) {
                        println!("✓ Found GGUF model file!");
                        model_path = Some(model_file);
                        break;
                    }
                }
            }
        }
    } else {
        println!("✗ GGUF repo directory not found");
    }
    
    // Step 4: Download model if needed
    if model_path.is_none() {
        println!("\nStep 4: Downloading model...");
        let api = Api::new()?;
        let repo = api.repo(Repo::with_revision(
            gguf_repo.to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        
        println!("  Downloading {} from {}...", gguf_file, gguf_repo);
        match repo.get(gguf_file).await {
            Ok(path) => {
                println!("✓ Model downloaded to: {:?}", path);
                model_path = Some(path);
            }
            Err(e) => {
                println!("✗ Failed to download model: {}", e);
                return Err(e.into());
            }
        }
    }
    
    // Step 5: Test loading GGUF model with Metal
    if let Some(path) = model_path {
        println!("\nStep 5: Testing GGUF model loading...");
        println!("  Model path: {:?}", path);
        
        let mut file = std::fs::File::open(&path)?;
        let model_content = candle_core::quantized::gguf_file::Content::read(&mut file)?;
        
        println!("  GGUF metadata keys: {} entries", model_content.metadata.len());
        if let Some(arch) = model_content.metadata.get("general.architecture") {
            println!("  Architecture: {:?}", arch);
        }
        
        // Try loading with Metal
        println!("\n  Attempting to load with Metal...");
        match QLlamaWeights::from_gguf(model_content, &mut file, &device) {
            Ok(_weights) => {
                println!("✓ Model loaded successfully on {:?}!", device.location());
                
                // Test a simple forward pass
                println!("\n  Testing forward pass...");
                let input = Tensor::new(&[1u32], &device)?.unsqueeze(0)?;
                match _weights.forward(&input, 0) {
                    Ok(output) => {
                        println!("✓ Forward pass successful!");
                        println!("  Output shape: {:?}", output.dims());
                    }
                    Err(e) => {
                        println!("✗ Forward pass failed: {}", e);
                        if format!("{}", e).contains("Metal") || format!("{}", e).contains("rms-norm") {
                            println!("  This is a known Metal limitation with quantized models");
                        }
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to load model on Metal: {}", e);
                if format!("{}", e).contains("Metal") || format!("{}", e).contains("rms-norm") {
                    println!("  This is a known Metal limitation with quantized models");
                    println!("  You may need to use CPU for this model");
                }
            }
        }
    }
    
    // Step 6: Check tokenizer
    println!("\nStep 6: Checking tokenizer...");
    let api = Api::new()?;
    let tokenizer_repo = api.repo(Repo::with_revision(
        model_id.to_string(),
        RepoType::Model,
        "main".to_string(),
    ));
    
    println!("  Downloading tokenizer...");
    match tokenizer_repo.get("tokenizer.json").await {
        Ok(path) => {
            println!("✓ Tokenizer downloaded to: {:?}", path);
            
            match Tokenizer::from_file(&path) {
                Ok(tokenizer) => {
                    println!("✓ Tokenizer loaded successfully!");
                    
                    // Test tokenization
                    let test_text = "Hello, world!";
                    match tokenizer.encode(test_text, true) {
                        Ok(encoding) => {
                            println!("✓ Tokenization test successful!");
                            println!("  Input: \"{}\"", test_text);
                            println!("  Tokens: {} tokens", encoding.len());
                        }
                        Err(e) => println!("✗ Tokenization failed: {}", e),
                    }
                }
                Err(e) => println!("✗ Failed to load tokenizer: {}", e),
            }
        }
        Err(e) => println!("✗ Failed to download tokenizer: {}", e),
    }
    
    println!("\n=== Test Complete ===");
    Ok(())
}