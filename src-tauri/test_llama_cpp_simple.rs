use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing LlamaCpp Backend");
    println!("========================");
    
    // First, let's just test that we can download a model
    use hf_hub::{api::tokio::Api, Repo, RepoType};
    
    let api = Api::new()?;
    let repo = api.repo(Repo::with_revision(
        "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF".to_string(),
        RepoType::Model,
        "main".to_string(),
    ));
    
    println!("Downloading TinyLlama GGUF model...");
    let model_path = repo.get("tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf").await?;
    println!("✅ Model downloaded to: {:?}", model_path);
    
    // Check if Metal is available
    let use_metal = cfg!(target_os = "macos");
    println!("Metal support: {}", if use_metal { "Available" } else { "Not available" });
    
    // Test our simplified backend
    use personalassistant_lib::llm::llama_cpp_metal_backend::LlamaCppMetalBackend;
    
    println!("\nInitializing LlamaCpp backend...");
    let cache_dir = dirs::cache_dir()
        .unwrap()
        .join("huggingface")
        .join("hub");
    
    let backend = LlamaCppMetalBackend::new(
        "TinyLlama/TinyLlama-1.1B-Chat-v1.0",
        cache_dir,
    ).await?;
    
    println!("\nTesting generation...");
    let response = backend.generate("What is 2+2?", 50).await?;
    println!("Response: {}", response);
    
    Ok(())
}