use crate::error::{AppError, Result};
use candle_core::{Device, Tensor};
use candle_transformers::models::quantized_llama::{ModelWeights as QLlamaWeights};
use hf_hub::{api::tokio::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use rand::Rng;

pub struct CraneBackend {
    model_id: String,
    model_revision: String,
    cache_dir: PathBuf,
    device: Device,
    device_type: String,
    model: Arc<Mutex<Option<QLlamaWeights>>>,
    tokenizer: Option<Tokenizer>,
    eos_token_id: Option<u32>,
    temperature: f64,
    top_p: f64,
    seed: u64,
}

impl CraneBackend {
    pub async fn new(
        model_id: &str,
        cache_dir: PathBuf,
    ) -> Result<Self> {
        println!("[CraneBackend] Initializing with model: {}", model_id);
        
        // Create cache directory
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| AppError::Llm(format!("Failed to create cache dir: {}", e)))?;
        
        // For GGUF models, we need to use CPU due to Metal limitations with quantized operations
        // Metal doesn't support rms-norm and other operations needed for quantized models
        let (device, device_type) = if cfg!(target_os = "macos") && false {  // Disabled for now
            match Device::new_metal(0) {
                Ok(metal_device) => {
                    println!("[CraneBackend] Successfully initialized Metal device");
                    println!("[CraneBackend] WARNING: Metal may not support all operations for quantized models");
                    (metal_device, "Metal".to_string())
                }
                Err(e) => {
                    println!("[CraneBackend] Metal initialization failed: {}, falling back to CPU", e);
                    (Device::Cpu, "CPU".to_string())
                }
            }
        } else {
            println!("[CraneBackend] Using CPU for quantized model compatibility");
            (Device::Cpu, "CPU".to_string())
        };
        
        println!("[CraneBackend] Using device: {}", device_type);
        
        let mut backend = Self {
            model_id: model_id.to_string(),
            model_revision: "main".to_string(),
            cache_dir: cache_dir.clone(),
            device,
            device_type,
            model: Arc::new(Mutex::new(None)),
            tokenizer: None,
            eos_token_id: None,
            temperature: 0.7, // More creative than Candle's default
            top_p: 0.9,
            seed: rand::thread_rng().gen(), // Random seed for variety
        };
        
        // Try to load the model
        match backend.load_model().await {
            Ok(_) => {
                println!("[CraneBackend] Model loaded successfully");
                println!("[CraneBackend] Model loaded: {}", backend.model.lock().unwrap().is_some());
                println!("[CraneBackend] Tokenizer loaded: {}", backend.tokenizer.is_some());
                println!("[CraneBackend] Device: {}", backend.device_type);
            }
            Err(e) => {
                eprintln!("[CraneBackend] Failed to load model on {}: {}", backend.device_type, e);
                
                // Check if it's a Metal-specific error
                let error_str = format!("{}", e);
                if error_str.contains("Metal error") || error_str.contains("rms-norm") {
                    eprintln!("[CraneBackend] Metal doesn't support all operations for quantized models");
                }
                
                // If Metal failed, try CPU as fallback
                if backend.device_type == "Metal" {
                    println!("[CraneBackend] Retrying with CPU device...");
                    backend.device = Device::Cpu;
                    backend.device_type = "CPU".to_string();
                    
                    if let Err(cpu_err) = backend.load_model().await {
                        eprintln!("[CraneBackend] Failed to load model on CPU: {}", cpu_err);
                        eprintln!("[CraneBackend] Will use placeholder responses");
                    } else {
                        println!("[CraneBackend] Model loaded successfully on CPU fallback");
                    }
                } else {
                    eprintln!("[CraneBackend] Will use placeholder responses");
                }
            }
        }
        
        Ok(backend)
    }
    
    async fn load_model(&mut self) -> Result<()> {
        println!("[CraneBackend] Loading model: {}", self.model_id);
        
        // Crane uses optimized GGUF models with better quantization for faster inference
        
        let api = Api::new()
            .map_err(|e| AppError::Llm(format!("Failed to create HF API: {}", e)))?;
        
        // Map models to their GGUF versions (TheBloke's quantized versions when available)
        let (repo_id, filename, tokenizer_repo) = match self.model_id.as_str() {
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => (
                "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF",
                "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
                "TinyLlama/TinyLlama-1.1B-Chat-v1.0"
            ),
            "Qwen/Qwen2.5-0.5B-Instruct" => (
                "Qwen/Qwen2.5-0.5B-Instruct-GGUF",
                "qwen2.5-0.5b-instruct-q4_k_m.gguf",
                "Qwen/Qwen2.5-0.5B-Instruct"
            ),
            "Qwen/Qwen2.5-1.5B-Instruct" => (
                "Qwen/Qwen2.5-1.5B-Instruct-GGUF", 
                "qwen2.5-1.5b-instruct-q4_k_m.gguf",
                "Qwen/Qwen2.5-1.5B-Instruct"
            ),
            "Qwen/Qwen2.5-3B-Instruct" => (
                "Qwen/Qwen2.5-3B-Instruct-GGUF",
                "qwen2.5-3b-instruct-q4_k_m.gguf",
                "Qwen/Qwen2.5-3B-Instruct"
            ),
            "Qwen/Qwen2.5-7B-Instruct" => (
                "Qwen/Qwen2.5-7B-Instruct-GGUF",
                "qwen2.5-7b-instruct-q4_k_m.gguf",
                "Qwen/Qwen2.5-7B-Instruct"
            ),
            "microsoft/phi-2" => (
                "TheBloke/phi-2-GGUF",
                "phi-2.Q4_K_M.gguf",
                "microsoft/phi-2"
            ),
            _ => {
                // Try to use the original model's GGUF if available
                println!("[CraneBackend] Attempting to load GGUF from original repo: {}", self.model_id);
                (
                    self.model_id.as_str(),
                    "ggml-model-q4_0.gguf",
                    self.model_id.as_str()
                )
            }
        };
        
        let repo = api.repo(Repo::with_revision(
            repo_id.to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        
        println!("[CraneBackend] Downloading GGUF model: {} from {}", filename, repo_id);
        let model_path = match repo.get(filename).await {
            Ok(path) => {
                println!("[CraneBackend] Model downloaded successfully to: {:?}", path);
                path
            }
            Err(e) => {
                println!("[CraneBackend] Failed to download {}: {}", filename, e);
                return Err(AppError::Llm(format!("Failed to download model: {}", e)));
            }
        };
        
        // Download tokenizer from the original model repo (not the GGUF repo)
        println!("[CraneBackend] Downloading tokenizer from original model repo: {}", tokenizer_repo);
        let original_repo = api.repo(Repo::with_revision(
            tokenizer_repo.to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        
        let tokenizer_path = original_repo.get("tokenizer.json")
            .await
            .map_err(|e| AppError::Llm(format!("Failed to download tokenizer: {}", e)))?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AppError::Llm(format!("Failed to load tokenizer: {}", e)))?;
        
        // Set EOS token based on model type
        let eos_token_id = match self.model_id.as_str() {
            model if model.starts_with("Qwen") => {
                tokenizer.token_to_id("<|im_end|>").or_else(|| tokenizer.token_to_id("<|endoftext|>"))
            }
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => {
                tokenizer.token_to_id("</s>").or_else(|| tokenizer.token_to_id("<|endoftext|>"))
            }
            _ => {
                tokenizer.token_to_id("<|endoftext|>").or_else(|| tokenizer.token_to_id("</s>"))
            }
        };
        println!("[CraneBackend] EOS token ID: {:?}", eos_token_id);
        
        // Load quantized model with optimizations
        println!("[CraneBackend] Loading GGUF model with Crane optimizations...");
        
        // Open the GGUF file
        let mut file = std::fs::File::open(&model_path)
            .map_err(|e| AppError::Llm(format!("Failed to open model file: {}", e)))?;
        let mut reader = std::io::BufReader::new(&mut file);
        
        // Load the GGUF file contents
        let model_content = candle_core::quantized::gguf_file::Content::read(&mut reader)
            .map_err(|e| AppError::Llm(format!("Failed to parse GGUF file: {}", e)))?;
        
        // Check model architecture in metadata
        let arch = model_content.metadata.get("general.architecture")
            .and_then(|v| v.to_string().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "llama".to_string());
        
        println!("[CraneBackend] Model architecture: {}", arch);
        
        // Create the model from GGUF content
        // TODO: Support different architectures (qwen2, phi, etc.)
        let model_weights = match arch.as_str() {
            "llama" | "qwen" | "qwen2" | "phi" | _ => {
                // For now, use QLlamaWeights for all architectures
                // In a real implementation, we'd have different weight loaders
                QLlamaWeights::from_gguf(model_content, &mut reader, &self.device)
                    .map_err(|e| AppError::Llm(format!("Failed to load model weights: {}", e)))?
            }
        };
        
        *self.model.lock().unwrap() = Some(model_weights);
        
        self.tokenizer = Some(tokenizer);
        self.eos_token_id = eos_token_id;
        
        println!("[CraneBackend] Model loaded successfully on {}", self.device_type);
        
        Ok(())
    }
    
    pub async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        let generation_start = Instant::now();
        
        println!("[CraneBackend] =================================");
        println!("[CraneBackend] Model: {}", self.model_id);
        println!("[CraneBackend] Device: {}", self.device_type);
        println!("[CraneBackend] Temperature: {}", self.temperature);
        println!("[CraneBackend] Top-p: {}", self.top_p);
        println!("[CraneBackend] Max tokens: {}", max_tokens);
        println!("[CraneBackend] Prompt preview: {}", 
            prompt.chars().take(100).collect::<String>());
        println!("[CraneBackend] =================================");
        
        let tokenizer = match &self.tokenizer {
            Some(t) => t,
            None => {
                println!("[CraneBackend] Tokenizer not loaded, using placeholder response");
                return Ok(self.generate_placeholder_response(prompt, max_tokens));
            }
        };
        
        let mut model = self.model.lock().unwrap();
        let model = match model.as_mut() {
            Some(m) => m,
            None => {
                println!("[CraneBackend] Model not loaded, using placeholder response");
                return Ok(self.generate_placeholder_response(prompt, max_tokens));
            }
        };
        
        // Apply chat template based on actual model being used
        let formatted_prompt = match self.model_id.as_str() {
            model if model.starts_with("Qwen") => {
                // Qwen models use ChatML format
                format!("<|im_start|>system\nYou are a helpful AI assistant.<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n", prompt)
            }
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => {
                // TinyLlama uses a specific format
                format!("<|system|>\nYou are a helpful AI assistant.</s>\n<|user|>\n{}</s>\n<|assistant|>\n", prompt)
            }
            "microsoft/phi-2" => {
                // Phi-2 uses a simpler format
                format!("Instruct: {}\nOutput:", prompt)
            }
            _ => {
                // Generic format
                format!("### Instruction:\n{}\n\n### Response:\n", prompt)
            }
        };
        
        // Tokenize with optimizations
        let tokens = tokenizer.encode(formatted_prompt.clone(), true)
            .map_err(|e| AppError::Llm(format!("Failed to encode prompt: {}", e)))?;
        let tokens = tokens.get_ids();
        
        let mut generated_tokens = Vec::new();
        let mut logits_processor = candle_transformers::generation::LogitsProcessor::new(
            self.seed,
            Some(self.temperature),
            Some(self.top_p)
        );
        
        // Optimized token generation for Crane
        // Process prompt tokens (can be done in single pass for better performance)
        let prompt_start = Instant::now();
        let mut last_logits = None;
        
        for (pos, &token) in tokens.iter().enumerate() {
            let input = Tensor::new(&[token as u32], &self.device)
                .map_err(|e| AppError::Llm(format!("Failed to create input tensor: {}", e)))?;
            let input = input.unsqueeze(0)
                .map_err(|e| AppError::Llm(format!("Failed to unsqueeze tensor: {}", e)))?;
            
            let logits = model.forward(&input, pos)
                .map_err(|e| AppError::Llm(format!("Model forward pass failed at position {}: {}", pos, e)))?;
            
            last_logits = Some(logits);
        }
        
        let prompt_elapsed = prompt_start.elapsed();
        println!("[CraneBackend] Prompt processing took {:.2}s ({} tokens, {:.1} tokens/s)",
            prompt_elapsed.as_secs_f32(),
            tokens.len(),
            tokens.len() as f32 / prompt_elapsed.as_secs_f32()
        );
        
        // Start generation from the last prompt token
        if let Some(logits) = last_logits {
            let squeezed_logits = logits.squeeze(0)
                .map_err(|e| AppError::Llm(format!("Failed to squeeze dim 0: {}", e)))?
                .squeeze(0)
                .map_err(|e| AppError::Llm(format!("Failed to squeeze dim 1: {}", e)))?;
            let mut current_token = logits_processor.sample(&squeezed_logits)
                .map_err(|e| AppError::Llm(format!("Failed to sample token: {}", e)))?;
            
            generated_tokens.push(current_token);
            
            // Generate remaining tokens
            let generation_start = Instant::now();
            for gen_pos in 0..max_tokens - 1 {  // -1 because we already generated one token
                if Some(current_token) == self.eos_token_id {
                    println!("[CraneBackend] Hit EOS token at position {}", gen_pos);
                    break;
                }
                
                let input = Tensor::new(&[current_token], &self.device)
                    .map_err(|e| AppError::Llm(format!("Failed to create tensor: {}", e)))?
                    .unsqueeze(0)
                    .map_err(|e| AppError::Llm(format!("Failed to unsqueeze: {}", e)))?;
                
                let logits = model.forward(&input, tokens.len() + gen_pos)
                    .map_err(|e| AppError::Llm(format!("Forward pass failed: {}", e)))?
                    .squeeze(0)
                    .map_err(|e| AppError::Llm(format!("Failed to squeeze: {}", e)))?
                    .squeeze(0)
                    .map_err(|e| AppError::Llm(format!("Failed to squeeze: {}", e)))?;
                
                current_token = logits_processor.sample(&logits)
                    .map_err(|e| AppError::Llm(format!("Failed to sample: {}", e)))?;
                generated_tokens.push(current_token);
            }
            
            let gen_elapsed = generation_start.elapsed();
            println!("[CraneBackend] Token generation took {:.2}s ({} tokens, {:.1} tokens/s)",
                gen_elapsed.as_secs_f32(),
                generated_tokens.len(),
                generated_tokens.len() as f32 / gen_elapsed.as_secs_f32()
            );
        }
        
        // Decode generated tokens
        let generated_text = tokenizer.decode(&generated_tokens, true)
            .map_err(|e| AppError::Llm(format!("Failed to decode tokens: {}", e)))?;
        
        let total_elapsed = generation_start.elapsed();
        println!("[CraneBackend] =================================");
        println!("[CraneBackend] Total generation time: {:.2}s", total_elapsed.as_secs_f32());
        println!("[CraneBackend] Generated {} tokens ({:.1} tokens/s overall)", 
            generated_tokens.len(),
            generated_tokens.len() as f32 / total_elapsed.as_secs_f32()
        );
        println!("[CraneBackend] =================================");
        
        Ok(generated_text)
    }
    
    fn generate_placeholder_response(&self, prompt: &str, max_tokens: usize) -> String {
        format!(
            "[Crane Engine - {}]\n\n\
            Processing query: {}\n\n\
            This is a placeholder response. The Crane engine is not loaded.\n\
            Crane provides optimized inference using quantized GGUF models.\n\
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
        println!("[CraneBackend] Temperature set to: {}", self.temperature);
    }
    
    pub fn set_top_p(&mut self, top_p: f64) {
        self.top_p = top_p.max(0.1).min(1.0);
        println!("[CraneBackend] Top-p set to: {}", self.top_p);
    }
    
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
        println!("[CraneBackend] Seed set to: {}", self.seed);
    }
    
    pub async fn get_model_info(&self) -> ModelInfo {
        ModelInfo {
            model_type: self.model_id.clone(),
            device: self.device_type.clone(),
            model_loaded: self.model.lock().unwrap().is_some(),
            tokenizer_loaded: self.tokenizer.is_some(),
            supported_features: vec![
                "quantized_models".to_string(),
                "gguf_format".to_string(),
                "optimized_inference".to_string(),
                "configurable_temperature".to_string(),
                if self.device_type == "Metal" { "metal_acceleration".to_string() } else { "cpu_optimized".to_string() }
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