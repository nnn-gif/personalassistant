use std::time::Instant;
use std::path::PathBuf;
// use llama_cpp::{LlamaModel, LlamaParams, SessionParams};
use hf_hub::{api::tokio::Api, Repo, RepoType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("llama_cpp is temporarily disabled for faster builds");
    Ok(())
    
    /*
    println!("🚀 Testing LlamaCpp Backend with Metal Support");
    println!("================================================");
    
    // Test configuration
    let model_id = "TinyLlama/TinyLlama-1.1B-Chat-v1.0";
    let cache_dir = PathBuf::from(
        dirs::cache_dir()
            .unwrap()
            .join("huggingface")
            .join("hub")
    );
    
    println!("📦 Model: {}", model_id);
    println!("📁 Cache Directory: {:?}", cache_dir);
    println!();
    
    // Check if Metal should be used
    let use_metal = cfg!(target_os = "macos") && 
        std::env::var("CANDLE_USE_METAL").map(|v| v != "0").unwrap_or(true);
    
    println!("🔧 Metal Support: {}", if use_metal { "✅ Enabled" } else { "❌ Disabled" });
    
    // Download GGUF model
    println!("\n📥 Downloading GGUF model...");
    let api = Api::new()?;
    
    let (repo_id, filename) = match model_id {
        "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => (
            "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF",
            "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf"
        ),
        _ => {
            eprintln!("❌ Model {} not configured", model_id);
            return Err("Model not configured".into());
        }
    };
    
    let repo = api.repo(Repo::with_revision(
        repo_id.to_string(),
        RepoType::Model,
        "main".to_string(),
    ));
    
    println!("📦 Downloading {} from {}", filename, repo_id);
    let model_path = repo.get(filename).await?;
    println!("✅ Model downloaded to: {:?}", model_path);
    
    // Load model with Metal support
    println!("\n🔧 Loading model with llama.cpp...");
    let mut params = LlamaParams::default();
    
    if use_metal {
        params.n_gpu_layers = 999; // Use Metal for all layers
        println!("⚡ Configured to use Metal for all layers");
    } else {
        params.n_gpu_layers = 0;
        println!("💻 Using CPU mode");
    }
    
    params.use_mmap = true;
    params.use_mlock = false;
    
    let start = Instant::now();
    let model = LlamaModel::load_from_file(&model_path, params)?;
    println!("✅ Model loaded in {:.2}s", start.elapsed().as_secs_f32());
    
    // Test generation
    println!("\n🤖 Testing generation...");
    let test_prompts = vec![
        "What is the capital of France?",
        "Write a haiku about Metal acceleration.",
        "Explain quantum computing in simple terms.",
    ];
    
    for prompt in test_prompts {
        println!("\n📝 Prompt: {}", prompt);
        println!("⏳ Generating response...");
        
        // Create a session
        let mut session_params = SessionParams::default();
        session_params.n_ctx = 2048;
        session_params.n_batch = 512;
        
        let mut session = model.create_session(session_params)?;
        
        // Apply chat template
        let formatted_prompt = format!(
            "<|system|>\nYou are a helpful AI assistant.</s>\n<|user|>\n{}</s>\n<|assistant|>\n",
            prompt
        );
        
        // Feed the prompt
        session.advance_context(&formatted_prompt)?;
        
        // Generate tokens
        let gen_start = Instant::now();
        let mut output = String::new();
        let mut tokens_generated = 0;
        
        // Set completion parameters
        let mut completion_params = session.completion_params();
        completion_params.temperature = 0.7;
        completion_params.top_p = 0.9;
        
        let completions = session.get_completions_async(completion_params, 100);
        
        for completion in completions {
            match completion {
                Ok(text) => {
                    output.push_str(&text);
                    tokens_generated += 1;
                }
                Err(e) => {
                    eprintln!("❌ Error during generation: {:?}", e);
                    break;
                }
            }
        }
        
        let elapsed = gen_start.elapsed();
        println!("✅ Generated {} tokens in {:.2}s", tokens_generated, elapsed.as_secs_f32());
        println!("⚡ Speed: {:.1} tokens/s", tokens_generated as f32 / elapsed.as_secs_f32());
        println!("📄 Response: {}", output.chars().take(200).collect::<String>());
        if output.len() > 200 {
            println!("... (truncated)");
        }
    }
    
    println!("\n✨ Test completed!");
    Ok(())
    */
}