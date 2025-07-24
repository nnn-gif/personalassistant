#[path = "src/llm/crane_backend.rs"]
mod crane_backend;
#[path = "src/error.rs"]
mod error;

use crane_backend::CraneBackend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Crane backend with Metal...");
    
    // Enable Metal
    std::env::set_var("CANDLE_USE_METAL", "1");
    std::env::set_var("CANDLE_MODEL_ID", "TinyLlama/TinyLlama-1.1B-Chat-v1.0");
    
    let cache_dir = dirs::home_dir()
        .unwrap()
        .join(".cache")
        .join("huggingface")
        .join("hub");
    
    println!("Cache directory: {:?}", cache_dir);
    println!("Creating Crane backend...");
    
    // Create the backend
    let backend = CraneBackend::new("TinyLlama/TinyLlama-1.1B-Chat-v1.0", cache_dir).await?;
    
    // Test generation
    println!("\nTesting generation...");
    let prompt = "What is the capital of France?";
    println!("Prompt: {}", prompt);
    
    let response = backend.generate(prompt, 50).await?;
    println!("Response: {}", response);
    
    // Test another prompt
    println!("\n---\nTesting another prompt...");
    let prompt2 = "Write a haiku about Metal acceleration";
    println!("Prompt: {}", prompt2);
    
    let response2 = backend.generate(prompt2, 50).await?;
    println!("Response: {}", response2);
    
    println!("\nTest completed successfully!");
    
    Ok(())
}