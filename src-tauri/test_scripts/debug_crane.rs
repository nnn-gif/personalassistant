use hf_hub::{api::tokio::Api, Repo, RepoType, Cache};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Debug Crane Model Loading ===\n");
    
    let model_id = "Qwen/Qwen2.5-3B-Instruct";
    let cache_dir = dirs::home_dir()
        .unwrap()
        .join(".cache")
        .join("huggingface")
        .join("hub");
    
    println!("Model: {}", model_id);
    println!("Cache dir: {:?}", cache_dir);
    
    // Check GGUF file
    let gguf_repo = "Qwen/Qwen2.5-3B-Instruct-GGUF";
    let gguf_file = "qwen2.5-3b-instruct-q4_k_m.gguf";
    
    println!("\n1. Checking GGUF file...");
    let hf_cache = Cache::new(cache_dir.clone());
    let repo = hf_cache.repo(hf_hub::Repo::model(gguf_repo.to_string()));
    
    if let Some(path) = repo.get(gguf_file) {
        println!("  GGUF path from cache: {:?}", path);
        println!("  Exists: {}", path.exists());
        if path.is_symlink() {
            println!("  Is symlink: true");
            if let Ok(target) = std::fs::read_link(&path) {
                println!("  Symlink target: {:?}", target);
                let full_target = if target.is_absolute() {
                    target
                } else {
                    path.parent().unwrap_or(std::path::Path::new(".")).join(target)
                };
                println!("  Full target path: {:?}", full_target);
                println!("  Target exists: {}", full_target.exists());
            }
        }
        
        // Try to open the file
        match std::fs::File::open(&path) {
            Ok(_) => println!("  ✓ Can open GGUF file"),
            Err(e) => println!("  ✗ Cannot open GGUF file: {}", e),
        }
    } else {
        println!("  ✗ GGUF file not found in cache");
    }
    
    // Check tokenizer
    println!("\n2. Checking tokenizer...");
    let tokenizer_repo = hf_cache.repo(hf_hub::Repo::model(model_id.to_string()));
    
    if let Some(path) = tokenizer_repo.get("tokenizer.json") {
        println!("  Tokenizer path: {:?}", path);
        println!("  Exists: {}", path.exists());
        
        // Try to load tokenizer
        match tokenizers::Tokenizer::from_file(&path) {
            Ok(_) => println!("  ✓ Can load tokenizer"),
            Err(e) => println!("  ✗ Cannot load tokenizer: {}", e),
        }
    } else {
        println!("  ✗ Tokenizer not found in cache");
        
        // Try downloading
        println!("\n3. Attempting to download tokenizer...");
        let api = Api::new()?;
        let api_repo = api.repo(Repo::with_revision(
            model_id.to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        
        match api_repo.get("tokenizer.json").await {
            Ok(path) => {
                println!("  ✓ Downloaded tokenizer to: {:?}", path);
            }
            Err(e) => {
                println!("  ✗ Failed to download tokenizer: {}", e);
            }
        }
    }
    
    Ok(())
}