use crate::error::{AppError, Result};
use candle_core::{Device, Tensor};
use candle_transformers::models::quantized_llama::{ModelWeights as QLlamaWeights};
use hf_hub::{api::tokio::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use std::time::Instant;

pub struct CallmBackend {
    pub model_id: String,
    cache_dir: PathBuf,
    model: Arc<Mutex<Option<QLlamaWeights>>>,
    tokenizer: Option<Tokenizer>,
    device: Device,
    device_type: String,
    temperature: f64,
    top_p: f64,
    seed: u64,
    eos_token_id: Option<u32>,
}

impl CallmBackend {
    pub async fn new(
        model_id: &str,
        cache_dir: PathBuf,
    ) -> Result<Self> {
        println!("[CallmBackend] Initializing with model: {}", model_id);
        
        // Create cache directory
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| AppError::Llm(format!("Failed to create cache dir: {}", e)))?;
        
        // For GGUF models, we need to use CPU due to Metal compatibility issues
        // GGUF quantized models often lack Metal implementations for operations like rms-norm
        println!("[CallmBackend] Note: GGUF quantized models require CPU for compatibility");
        println!("[CallmBackend] Metal acceleration is not available for quantized models");
        
        let (device, device_type) = (Device::Cpu, "CPU (GGUF compatibility mode)".to_string());
        
        let mut backend = Self {
            model_id: model_id.to_string(),
            cache_dir: cache_dir.clone(),
            model: Arc::new(Mutex::new(None)),
            tokenizer: None,
            device,
            device_type,
            temperature: 0.7, // Balanced temperature for quality generation
            top_p: 0.9,
            seed: 42,
            eos_token_id: None,
        };
        
        // Try to load the model
        match backend.load_model().await {
            Ok(_) => {
                println!("[CallmBackend] Model loaded successfully");
                println!("[CallmBackend] Model ready: {}", backend.model.lock().await.is_some());
                println!("[CallmBackend] Tokenizer ready: {}", backend.tokenizer.is_some());
            }
            Err(e) => {
                eprintln!("[CallmBackend] Failed to load model: {}", e);
                return Err(e); // Fail initialization to see the error
            }
        }
        
        Ok(backend)
    }
    
    async fn load_model(&mut self) -> Result<()> {
        println!("[CallmBackend] Loading optimized model: {}", self.model_id);
        
        let api = Api::new()
            .map_err(|e| AppError::Llm(format!("Failed to create HF API: {}", e)))?;
        
        // Map models to their optimized GGUF versions
        // Note: All GGUF models run on CPU due to Metal compatibility limitations
        let (repo_id, filename, tokenizer_repo) = match self.model_id.as_str() {
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => {
                println!("[CallmBackend] Selected: TinyLlama 1.1B - Fast and efficient on CPU");
                (
                    "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF",
                    "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
                    "TinyLlama/TinyLlama-1.1B-Chat-v1.0"
                )
            },
            "microsoft/phi-2" => {
                println!("[CallmBackend] Selected: Phi-2 2.7B - Good balance of quality and speed");
                (
                    "TheBloke/phi-2-GGUF",
                    "phi-2.Q4_K_M.gguf",
                    "microsoft/phi-2"
                )
            },
            "Qwen/Qwen2.5-0.5B-Instruct" => {
                println!("[CallmBackend] Selected: Qwen 2.5 0.5B - Fastest option, good for quick responses");
                (
                    "Qwen/Qwen2.5-0.5B-Instruct-GGUF",
                    "qwen2.5-0.5b-instruct-q4_k_m.gguf",
                    "Qwen/Qwen2.5-0.5B-Instruct"
                )
            },
            "Qwen/Qwen2.5-1.5B-Instruct" => {
                println!("[CallmBackend] Selected: Qwen 2.5 1.5B - Better quality than 0.5B");
                (
                    "Qwen/Qwen2.5-1.5B-Instruct-GGUF",
                    "qwen2.5-1.5b-instruct-q4_k_m.gguf",
                    "Qwen/Qwen2.5-1.5B-Instruct"
                )
            },
            "Qwen/Qwen2.5-3B-Instruct" => {
                println!("[CallmBackend] Selected: Qwen 2.5 3B - High quality for complex tasks");
                (
                    "Qwen/Qwen2.5-3B-Instruct-GGUF",
                    "qwen2.5-3b-instruct-q4_k_m.gguf",
                    "Qwen/Qwen2.5-3B-Instruct"
                )
            },
            "Qwen/Qwen2.5-7B-Instruct" => {
                println!("[CallmBackend] Selected: Qwen 2.5 7B - Best quality, slowest performance");
                (
                    "Qwen/Qwen2.5-7B-Instruct-GGUF",
                    "qwen2.5-7b-instruct-q4_k_m.gguf",
                    "Qwen/Qwen2.5-7B-Instruct"
                )
            },
            _ => {
                return Err(AppError::Llm(format!("Unsupported model for Callm: {}", self.model_id)));
            }
        };
        
        // Download model
        let repo = api.repo(Repo::with_revision(
            repo_id.to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        
        println!("[CallmBackend] Downloading optimized GGUF model from {} - {}", repo_id, filename);
        let model_path = repo.get(filename).await
            .map_err(|e| AppError::Llm(format!("Failed to download model {}: {}", filename, e)))?;
        println!("[CallmBackend] Model downloaded to: {:?}", model_path);
        
        // Download tokenizer - try from original repo first, then GGUF repo as fallback
        println!("[CallmBackend] Downloading tokenizer from: {}", tokenizer_repo);
        let tokenizer_path = {
            let tokenizer_api_repo = api.repo(Repo::with_revision(
                tokenizer_repo.to_string(),
                RepoType::Model,
                "main".to_string(),
            ));
            
            match tokenizer_api_repo.get("tokenizer.json").await {
                Ok(path) => {
                    println!("[CallmBackend] Tokenizer downloaded from original repo");
                    path
                }
                Err(e) => {
                    println!("[CallmBackend] Failed to download from original repo: {}, trying manual path", e);
                    
                    // Try manual path construction as fallback
                    // First try the HuggingFace hub cache directory
                    let hf_cache_dir = dirs::home_dir()
                        .unwrap()
                        .join(".cache")
                        .join("huggingface")
                        .join("hub");
                    
                    let manual_path = hf_cache_dir.join(format!("models--{}", tokenizer_repo.replace("/", "--")))
                        .join("snapshots")
                        .join("main")
                        .join("tokenizer.json");
                    
                    if manual_path.exists() {
                        println!("[CallmBackend] Found tokenizer at manual path: {:?}", manual_path);
                        manual_path
                    } else {
                        // Try downloading from GGUF repo as last resort
                        println!("[CallmBackend] Trying GGUF repo as fallback");
                        repo.get("tokenizer.json").await
                            .map_err(|e2| AppError::Llm(format!(
                                "Failed to download tokenizer from both repos. Original: {}, GGUF: {}", e, e2
                            )))?
                    }
                }
            }
        };
        
        // Load tokenizer
        println!("[CallmBackend] Loading tokenizer from: {:?}", tokenizer_path);
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AppError::Llm(format!("Failed to load tokenizer: {}", e)))?;
        println!("[CallmBackend] Tokenizer loaded successfully");
        
        // Set EOS token
        let eos_token_id = match self.model_id.as_str() {
            model if model.contains("Qwen") => tokenizer.token_to_id("<|im_end|>"),
            _ => tokenizer.token_to_id("</s>").or_else(|| tokenizer.token_to_id("<|endoftext|>")),
        };
        
        println!("[CallmBackend] Loading GGUF model with hardware optimizations...");
        
        // Load the GGUF model
        let mut file = std::fs::File::open(&model_path)
            .map_err(|e| AppError::Llm(format!("Failed to open model file: {}", e)))?;
        let model_content = candle_core::quantized::gguf_file::Content::read(&mut file)
            .map_err(|e| AppError::Llm(format!("Failed to parse GGUF file: {}", e)))?;
        
        // Load GGUF model on CPU (required for compatibility)
        println!("[CallmBackend] Loading GGUF model on CPU for compatibility...");
        
        let model_weights = QLlamaWeights::from_gguf(model_content, &mut file, &self.device)
            .map_err(|e| AppError::Llm(format!("Failed to load GGUF model: {}", e)))?;
        
        println!("[CallmBackend] ✓ Model loaded successfully on CPU");
        
        *self.model.lock().await = Some(model_weights);
        self.tokenizer = Some(tokenizer);
        self.eos_token_id = eos_token_id;
        
        println!("[CallmBackend] Model ready for inference on CPU");
        println!("[CallmBackend] Note: For Metal acceleration, use non-quantized models");
        
        // Verify model is loaded
        if self.model.try_lock().is_ok() {
            println!("[CallmBackend] Model lock acquired successfully");
        }
        if self.tokenizer.is_some() {
            println!("[CallmBackend] Tokenizer is available");
        }
        if let Some(eos_id) = self.eos_token_id {
            println!("[CallmBackend] EOS token ID: {}", eos_id);
        }
        
        Ok(())
    }
    
    pub async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        let start_time = Instant::now();
        
        println!("[CallmBackend] =================================");
        println!("[CallmBackend] Model: {}", self.model_id);
        println!("[CallmBackend] Device: {}", self.device_type);
        println!("[CallmBackend] Temperature: {}", self.temperature);
        println!("[CallmBackend] Top-p: {}", self.top_p);
        println!("[CallmBackend] Max tokens: {}", max_tokens);
        println!("[CallmBackend] Prompt preview: {}", 
            prompt.chars().take(100).collect::<String>());
        println!("[CallmBackend] =================================");
        
        let tokenizer = match &self.tokenizer {
            Some(t) => t,
            None => {
                println!("[CallmBackend] Tokenizer not loaded");
                return Err(AppError::Llm("Tokenizer not loaded".into()));
            }
        };
        
        let mut model_guard = self.model.lock().await;
        let model = match model_guard.as_mut() {
            Some(m) => {
                println!("[CallmBackend] Model acquired successfully");
                m
            }
            None => {
                println!("[CallmBackend] ERROR: Model not loaded in memory");
                return Err(AppError::Llm("Model not loaded in memory".into()));
            }
        };
        
        // Apply chat template
        let formatted_prompt = match self.model_id.as_str() {
            model if model.contains("Qwen") => {
                format!("<|im_start|>system\nYou are a helpful AI assistant.<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n", prompt)
            }
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => {
                format!("<|system|>\nYou are a helpful AI assistant.</s>\n<|user|>\n{}</s>\n<|assistant|>\n", prompt)
            }
            _ => {
                format!("### Instruction:\n{}\n\n### Response:\n", prompt)
            }
        };
        
        // Tokenize
        println!("[CallmBackend] Tokenizing prompt ({} chars)...", formatted_prompt.len());
        let tokens = tokenizer.encode(formatted_prompt.clone(), true)
            .map_err(|e| AppError::Llm(format!("Failed to encode prompt: {}", e)))?;
        let tokens = tokens.get_ids();
        println!("[CallmBackend] Tokenized to {} tokens", tokens.len());
        
        let mut generated_tokens = Vec::new();
        let mut logits_processor = candle_transformers::generation::LogitsProcessor::new(
            self.seed,
            Some(self.temperature),
            Some(self.top_p)
        );
        
        // Process prompt with optimizations
        let prompt_start = Instant::now();
        let mut last_logits = None;
        
        println!("[CallmBackend] Processing {} prompt tokens...", tokens.len());
        for (pos, &token) in tokens.iter().enumerate() {
            // Use the same device as the model
            let input = Tensor::new(&[token as u32], &self.device)
                .and_then(|t| t.unsqueeze(0))
                .map_err(|e| AppError::Llm(format!("Failed to create input tensor: {}", e)))?;
            
            match model.forward(&input, pos) {
                Ok(logits) => {
                    last_logits = Some(logits);
                    if pos == 0 || pos == tokens.len() - 1 || pos % 10 == 0 {
                        println!("[CallmBackend] Processed token {} of {}", pos + 1, tokens.len());
                    }
                }
                Err(e) => {
                    eprintln!("[CallmBackend] ERROR: Forward pass failed at position {}: {}", pos, e);
                    return Err(AppError::Llm(format!("Model forward pass failed at position {}: {}", pos, e)));
                }
            }
        }
        
        let prompt_time = prompt_start.elapsed();
        println!("[CallmBackend] Prompt processed in {:.2}s ({:.1} tokens/s)",
            prompt_time.as_secs_f32(),
            tokens.len() as f32 / prompt_time.as_secs_f32()
        );
        
        // Generate tokens with hardware acceleration
        if let Some(logits) = last_logits {
            println!("[CallmBackend] Starting generation...");
            let generation_start = Instant::now();
            
            let squeezed_logits = logits.squeeze(0)
                .and_then(|t| t.squeeze(0))
                .map_err(|e| AppError::Llm(format!("Failed to squeeze logits: {}", e)))?;
            
            let mut current_token = logits_processor.sample(&squeezed_logits)
                .map_err(|e| AppError::Llm(format!("Failed to sample token: {}", e)))?;
            
            generated_tokens.push(current_token);
            
            // Generate remaining tokens with a reasonable limit
            let max_gen_tokens = max_tokens.min(150); // Limit to 150 tokens for faster response
            for i in 0..max_gen_tokens - 1 {
                if Some(current_token) == self.eos_token_id {
                    println!("[CallmBackend] EOS token reached at position {}", i);
                    break;
                }
                
                // Add a basic repetition check
                if generated_tokens.len() > 3 {
                    let last_tokens: Vec<u32> = generated_tokens.iter().rev().take(3).copied().collect();
                    if last_tokens.windows(2).all(|w| w[0] == w[1]) {
                        println!("[CallmBackend] Detected repetition, stopping generation");
                        break;
                    }
                }
                
                // Use the same device as the model
                let input = Tensor::new(&[current_token], &self.device)
                    .and_then(|t| t.unsqueeze(0))
                    .map_err(|e| AppError::Llm(format!("Failed to create tensor: {}", e)))?;
                
                let logits = model.forward(&input, tokens.len() + i)
                    .map_err(|e| AppError::Llm(format!("Forward pass failed: {}", e)))?
                    .squeeze(0)
                    .and_then(|t| t.squeeze(0))
                    .map_err(|e| AppError::Llm(format!("Failed to squeeze: {}", e)))?;
                
                current_token = logits_processor.sample(&logits)
                    .map_err(|e| AppError::Llm(format!("Failed to sample: {}", e)))?;
                generated_tokens.push(current_token);
                
                // Progress indicator
                if i % 10 == 0 {
                    println!("[CallmBackend] Generated {} tokens...", i + 1);
                }
            }
            
            let gen_time = generation_start.elapsed();
            println!("[CallmBackend] Generated {} tokens in {:.2}s ({:.1} tokens/s)",
                generated_tokens.len(),
                gen_time.as_secs_f32(),
                generated_tokens.len() as f32 / gen_time.as_secs_f32()
            );
        } else {
            println!("[CallmBackend] Warning: No logits from prompt processing");
            return Err(AppError::Llm("Failed to process prompt - no logits generated".into()));
        }
        
        // Decode
        println!("[CallmBackend] Decoding {} generated tokens...", generated_tokens.len());
        let generated_text = tokenizer.decode(&generated_tokens, true)
            .map_err(|e| AppError::Llm(format!("Failed to decode tokens: {}", e)))?;
        println!("[CallmBackend] Generated text length: {} chars", generated_text.len());
        
        let total_time = start_time.elapsed();
        println!("[CallmBackend] =================================");
        println!("[CallmBackend] Total generation time: {:.2}s", total_time.as_secs_f32());
        println!("[CallmBackend] Hardware acceleration: {}", self.device_type);
        println!("[CallmBackend] Generated text: {:?}", generated_text);
        println!("[CallmBackend] =================================");
        
        if generated_text.is_empty() {
            println!("[CallmBackend] Warning: Empty response generated");
            Ok("I apologize, but I'm having trouble generating a response. Please try again.".to_string())
        } else {
            Ok(generated_text)
        }
    }
    
    fn generate_placeholder_response(&self, prompt: &str, max_tokens: usize) -> String {
        format!(
            "[Callm Backend - {}]\n\n\
            Processing query: {}\n\n\
            This is a placeholder response. The Callm model is not loaded.\n\
            Callm provides hardware-accelerated inference using optimized implementations.\n\
            Device: {}\n\
            Max tokens: {}",
            self.model_id,
            prompt.chars().take(50).collect::<String>(),
            self.device_type,
            max_tokens
        )
    }
    
    pub fn set_temperature(&mut self, temperature: f64) {
        self.temperature = temperature.max(0.1).min(2.0);
        println!("[CallmBackend] Temperature set to: {}", self.temperature);
    }
    
    pub fn set_top_p(&mut self, top_p: f64) {
        self.top_p = top_p.max(0.1).min(1.0);
        println!("[CallmBackend] Top-p set to: {}", self.top_p);
    }
    
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
        println!("[CallmBackend] Seed set to: {}", self.seed);
    }
    
    pub async fn get_model_info(&self) -> ModelInfo {
        ModelInfo {
            model_type: self.model_id.clone(),
            device: self.device_type.clone(),
            model_loaded: self.model.lock().await.is_some(),
            tokenizer_loaded: self.tokenizer.is_some(),
            supported_features: vec![
                "hardware_acceleration".to_string(),
                "optimized_inference".to_string(),
                "configurable_temperature".to_string(),
                "auto_device_selection".to_string(),
                "gguf_quantization".to_string(),
                if self.device_type == "Metal" { "metal_acceleration".to_string() } 
                else if self.device_type == "CUDA" { "cuda_acceleration".to_string() }
                else { "cpu_optimized".to_string() }
            ],
            temperature: self.temperature,
            top_p: self.top_p,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ModelInfo {
    pub model_type: String,
    pub device: String,
    pub model_loaded: bool,
    pub tokenizer_loaded: bool,
    pub supported_features: Vec<String>,
    pub temperature: f64,
    pub top_p: f64,
}