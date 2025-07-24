// Since crane_backend is private, we'll test through the public API
use personalassistant_lib::config::{Config, InferenceProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing Crane Backend ===\n");
    
    // Test with different models
    let models = vec![
        "TinyLlama/TinyLlama-1.1B-Chat-v1.0",
        "Qwen/Qwen2.5-3B-Instruct",
    ];
    
    for model_id in models {
        println!("\n--- Testing model: {} ---", model_id);
        
        let cache_dir = dirs::home_dir()
            .unwrap()
            .join(".cache")
            .join("huggingface")
            .join("hub");
        
        match CraneBackend::new(model_id, cache_dir).await {
            Ok(backend) => {
                println!("✓ Backend created successfully!");
                
                // Test generation
                let prompt = "What is 2+2?";
                println!("\nPrompt: {}", prompt);
                
                match backend.generate(prompt, 50).await {
                    Ok(response) => {
                        println!("✓ Generation successful!");
                        println!("Response: {}", response);
                    }
                    Err(e) => {
                        println!("✗ Generation failed: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to create backend: {}", e);
            }
        }
        
        println!("\n{}", "=".repeat(50));
    }
    
    Ok(())
}