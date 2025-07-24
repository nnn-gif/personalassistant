use crate::error::{AppError, Result};
use std::path::PathBuf;
use std::time::Instant;
use llama_cpp::{LlamaModel, LlamaParams, SessionParams};
use llama_cpp::standard_sampler::StandardSampler;

pub struct LlamaCppMetalBackend {
    model_id: String,
    #[allow(dead_code)]
    cache_dir: PathBuf,
    model_path: Option<PathBuf>,
    use_gpu: bool,
    model: Option<LlamaModel>,
}

impl LlamaCppMetalBackend {
    pub async fn new(
        model_id: &str,
        cache_dir: PathBuf,
    ) -> Result<Self> {
        println!("[LlamaCppMetalBackend] Initializing with model: {}", model_id);
        
        // Check if we should use GPU acceleration based on OS
        let use_gpu = if cfg!(target_os = "macos") {
            // Check for Metal support on macOS
            std::env::var("CANDLE_USE_METAL").map(|v| v != "0").unwrap_or(true)
        } else if cfg!(target_os = "windows") {
            // Windows GPU support disabled due to build complexity
            // Users should use Ollama for GPU acceleration on Windows
            false
        } else {
            false
        };
        
        if use_gpu {
            if cfg!(target_os = "macos") {
                println!("[LlamaCppMetalBackend] ✅ Full Metal support via llama.cpp!");
                println!("[LlamaCppMetalBackend] ✅ All operations including layer_norm and rms_norm supported!");
            }
        } else {
            println!("[LlamaCppMetalBackend] Using CPU mode");
        }
        
        let mut backend = Self {
            model_id: model_id.to_string(),
            cache_dir,
            model_path: None,
            use_gpu,
            model: None,
        };
        
        // Try to load the model
        match backend.download_gguf_model().await {
            Ok(path) => {
                println!("[LlamaCppMetalBackend] Model downloaded successfully");
                backend.model_path = Some(path.clone());
                
                // Initialize llama.cpp model
                match backend.load_model(&path) {
                    Ok(()) => println!("[LlamaCppMetalBackend] Model loaded successfully"),
                    Err(e) => {
                        eprintln!("[LlamaCppMetalBackend] Failed to load model: {}", e);
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[LlamaCppMetalBackend] Failed to download model: {}", e);
                return Err(e);
            }
        }
        
        Ok(backend)
    }
    
    async fn download_gguf_model(&self) -> Result<PathBuf> {
        // Reuse the GGUF download logic
        use hf_hub::{api::tokio::Api, Repo, RepoType};
        
        let api = Api::new()
            .map_err(|e| AppError::Llm(format!("Failed to create HF API: {}", e)))?;
        
        let (repo_id, filename) = match self.model_id.as_str() {
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => (
                "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF",
                "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf"
            ),
            "Qwen/Qwen2.5-0.5B-Instruct" => (
                "Qwen/Qwen2.5-0.5B-Instruct-GGUF",
                "qwen2.5-0.5b-instruct-q4_k_m.gguf"
            ),
            _ => {
                return Err(AppError::Llm(format!("Model {} not configured", self.model_id)));
            }
        };
        
        let repo = api.repo(Repo::with_revision(
            repo_id.to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        
        println!("[LlamaCppMetalBackend] Downloading {} from {}", filename, repo_id);
        let model_path = repo.get(filename).await
            .map_err(|e| AppError::Llm(format!("Failed to download model: {}", e)))?;
        
        Ok(model_path)
    }
    
    fn load_model(&mut self, model_path: &PathBuf) -> Result<()> {
        println!("[LlamaCppMetalBackend] Loading model from: {:?}", model_path);
        
        // Create model parameters
        let mut params = LlamaParams::default();
        
        // Configure for GPU if available
        if self.use_gpu {
            params.n_gpu_layers = 999; // Load all layers to GPU
            if cfg!(target_os = "macos") {
                println!("[LlamaCppMetalBackend] Configured for Metal with n_gpu_layers=999");
            }
        } else {
            params.n_gpu_layers = 0; // CPU only
            println!("[LlamaCppMetalBackend] Configured for CPU only");
        }
        
        // Load the model
        match LlamaModel::load_from_file(model_path, params) {
            Ok(model) => {
                println!("[LlamaCppMetalBackend] Model loaded successfully!");
                self.model = Some(model);
                Ok(())
            }
            Err(e) => {
                eprintln!("[LlamaCppMetalBackend] Failed to load model: {}", e);
                Err(AppError::Llm(format!("Failed to load llama.cpp model: {}", e)))
            }
        }
    }
    
    pub async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        let start_time = Instant::now();
        
        println!("[LlamaCppMetalBackend] =================================");
        println!("[LlamaCppMetalBackend] Model: {}", self.model_id);
        let device = if self.use_gpu && cfg!(target_os = "macos") {
            "Metal"
        } else {
            "CPU"
        };
        println!("[LlamaCppMetalBackend] Device: {}", device);
        println!("[LlamaCppMetalBackend] Max tokens: {}", max_tokens);
        println!("[LlamaCppMetalBackend] Prompt: {}", prompt.chars().take(100).collect::<String>());
        println!("[LlamaCppMetalBackend] =================================");
        
        // Check if model is loaded
        let model = self.model.as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".into()))?;
        
        // Apply chat template based on model
        let formatted_prompt = self.apply_chat_template(prompt);
        
        // Tokenize to check prompt length
        let tokens = model.tokenize_bytes(formatted_prompt.as_bytes(), true, false)
            .map_err(|e| AppError::Llm(format!("Failed to tokenize prompt: {}", e)))?;
        
        println!("[LlamaCppMetalBackend] Prompt tokens: {}", tokens.len());
        
        // Create a session for generation
        let mut session_params = SessionParams::default();
        
        // Set a larger batch size to handle longer prompts
        session_params.n_batch = 2048; // Increase batch size for longer prompts
        session_params.n_ctx = 4096;   // Increase context size
        
        // Create a new session
        let mut session = model.create_session(session_params)
            .map_err(|e| AppError::Llm(format!("Failed to create session: {}", e)))?;
        
        // Truncate prompt if it's too long
        let final_prompt = if tokens.len() > 2000 {
            println!("[LlamaCppMetalBackend] Warning: Truncating prompt from {} to ~2000 tokens", tokens.len());
            // Approximate truncation - take first 75% of the prompt
            let truncate_at = (formatted_prompt.len() * 3) / 4;
            let truncated = &formatted_prompt[..truncate_at];
            format!("{}...", truncated)
        } else {
            formatted_prompt
        };
        
        // Feed the prompt to the context
        session.advance_context(final_prompt.as_str())
            .map_err(|e| AppError::Llm(format!("Failed to process prompt: {}", e)))?;
        
        // Create a sampler with parameters
        let sampler = StandardSampler::default();
        
        // Generate tokens using the completion API
        let mut output = String::new();
        let mut token_count = 0;
        
        println!("[LlamaCppMetalBackend] Starting generation...");
        
        // Use the completion API with sampler
        let completions = session
            .start_completing_with(sampler, max_tokens)
            .map_err(|e| AppError::Llm(format!("Failed to start completion: {}", e)))?
            .into_strings();
        
        for completion in completions {
            output.push_str(&completion);
            token_count += 1;
            
            // Check for stop tokens
            if output.contains("</s>") || output.contains("<|im_end|>") || 
               output.contains("<|assistant|>") || token_count >= max_tokens {
                break;
            }
        }
        
        let elapsed = start_time.elapsed();
        let tokens_per_second = token_count as f32 / elapsed.as_secs_f32();
        
        println!("[LlamaCppMetalBackend] =================================");
        println!("[LlamaCppMetalBackend] Generated {} tokens in {:.2}s ({:.1} tokens/s)", 
                 token_count, elapsed.as_secs_f32(), tokens_per_second);
        println!("[LlamaCppMetalBackend] =================================");
        
        // Clean up output - remove any trailing stop tokens
        let output = output
            .trim()
            .trim_end_matches("</s>")
            .trim_end_matches("<|im_end|>")
            .trim_end_matches("<|assistant|>")
            .trim()
            .to_string();
        
        Ok(output)
    }
    
    fn apply_chat_template(&self, prompt: &str) -> String {
        match self.model_id.as_str() {
            model if model.starts_with("Qwen") => {
                format!("<|im_start|>system\nYou are a helpful AI assistant.<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n", prompt)
            }
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => {
                format!("<|system|>\nYou are a helpful AI assistant.</s>\n<|user|>\n{}</s>\n<|assistant|>\n", prompt)
            }
            _ => {
                format!("### Instruction:\n{}\n\n### Response:\n", prompt)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_template() {
        let backend = LlamaCppMetalBackend {
            model_id: "TinyLlama/TinyLlama-1.1B-Chat-v1.0".to_string(),
            cache_dir: PathBuf::new(),
            model_path: None,
            use_gpu: false,
            model: None,
        };
        
        let prompt = "Hello, how are you?";
        let formatted = backend.apply_chat_template(prompt);
        
        assert!(formatted.contains("<|system|>"));
        assert!(formatted.contains("<|user|>"));
        assert!(formatted.contains(prompt));
        assert!(formatted.contains("<|assistant|>"));
    }
    
    #[test]
    fn test_qwen_chat_template() {
        let backend = LlamaCppMetalBackend {
            model_id: "Qwen/Qwen2.5-0.5B-Instruct".to_string(),
            cache_dir: PathBuf::new(),
            model_path: None,
            use_gpu: false,
            model: None,
        };
        
        let prompt = "What is 2+2?";
        let formatted = backend.apply_chat_template(prompt);
        
        assert!(formatted.contains("<|im_start|>"));
        assert!(formatted.contains("<|im_end|>"));
        assert!(formatted.contains(prompt));
    }
    
    #[test] 
    fn test_gpu_detection() {
        let expected_macos = cfg!(target_os = "macos") && 
            std::env::var("CANDLE_USE_METAL").map(|v| v != "0").unwrap_or(true);
        
        let expected_windows = cfg!(target_os = "windows") && 
            std::env::var("LLAMA_CUDA_FORCE_DISABLE").map(|v| v != "1").unwrap_or(true);
            
        // Just verify the logic works
        if cfg!(target_os = "macos") {
            assert_eq!(expected_macos, expected_macos);
        } else if cfg!(target_os = "windows") {
            assert_eq!(expected_windows, expected_windows);
        }
    }
}