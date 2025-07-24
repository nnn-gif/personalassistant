use crate::error::{AppError, Result};
use candle_core::{Device, Tensor, DType};
use candle_transformers::models::quantized_llama::{ModelWeights as QLlamaWeights};
use candle_transformers::models::llama as model;
use candle_nn::VarBuilder;
use hf_hub::{api::tokio::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use rand::Rng;
use serde::Deserialize;

enum ModelType {
    Quantized(QLlamaWeights),
    Standard {
        model: model::Llama,
        cache: model::Cache,
    },
}

#[derive(Deserialize)]
struct LlamaConfigJson {
    hidden_size: usize,
    intermediate_size: usize,
    vocab_size: usize,
    num_hidden_layers: usize,
    num_attention_heads: usize,
    num_key_value_heads: Option<usize>,
    max_position_embeddings: usize,
    rms_norm_eps: f64,
    rope_theta: Option<f32>,
    bos_token_id: Option<u32>,
    eos_token_id: Option<u32>,
    tie_word_embeddings: Option<bool>,
    use_flash_attn: Option<bool>,
}

pub struct CraneBackend {
    model_id: String,
    model_revision: String,
    cache_dir: PathBuf,
    device: Device,
    device_type: String,
    model: Arc<Mutex<Option<ModelType>>>,
    tokenizer: Option<Tokenizer>,
    eos_token_id: Option<u32>,
    temperature: f64,
    top_p: f64,
    seed: u64,
    use_quantized: bool,
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
        
        // Check if we want to use Metal
        let use_metal = std::env::var("CANDLE_USE_METAL")
            .map(|v| v != "0")
            .unwrap_or(true);
        
        // For now, always use CPU due to Metal's lack of rms-norm support
        // Keep the Metal detection for future when support is added
        let (device, device_type, use_quantized) = if cfg!(target_os = "macos") && use_metal {
            match Device::new_metal(0) {
                Ok(_metal_device) => {
                    println!("[CraneBackend] ✓ Metal device detected");
                    println!("[CraneBackend] ⚠️  Metal lacks rms-norm support for Llama models");
                    println!("[CraneBackend] → Using CPU with quantized models instead");
                    // TODO: Enable Metal when rms-norm is implemented
                    // For now, always use CPU with quantized models
                    (Device::Cpu, "CPU (Metal available but not supported)".to_string(), true)
                }
                Err(e) => {
                    println!("[CraneBackend] Metal not available: {}", e);
                    (Device::Cpu, "CPU".to_string(), true)
                }
            }
        } else {
            println!("[CraneBackend] Using CPU with quantized models");
            (Device::Cpu, "CPU".to_string(), true)
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
            use_quantized,
        };
        
        // Try to load the model
        match backend.load_model().await {
            Ok(_) => {
                println!("[CraneBackend] Model loaded successfully");
                println!("[CraneBackend] Model loaded: {}", backend.model.lock().unwrap().is_some());
                println!("[CraneBackend] Tokenizer loaded: {}", backend.tokenizer.is_some());
                println!("[CraneBackend] Device: {}", backend.device_type);
                
                // Double check that both model and tokenizer are loaded
                if backend.model.lock().unwrap().is_none() || backend.tokenizer.is_none() {
                    eprintln!("[CraneBackend] ERROR: Model or tokenizer is None after successful load!");
                    eprintln!("[CraneBackend] Model: {:?}, Tokenizer: {:?}", 
                        backend.model.lock().unwrap().is_some(), 
                        backend.tokenizer.is_some()
                    );
                    return Err(AppError::Llm("Model initialization incomplete".into()));
                }
            }
            Err(e) => {
                eprintln!("[CraneBackend] Failed to load model on {}: {}", backend.device_type, e);
                eprintln!("[CraneBackend] Full error: {:?}", e);
                
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
                        eprintln!("[CraneBackend] Full CPU error: {:?}", cpu_err);
                        return Err(cpu_err); // Return error instead of using broken backend
                    } else {
                        println!("[CraneBackend] Model loaded successfully on CPU fallback");
                    }
                } else {
                    return Err(e); // Return error instead of using broken backend
                }
            }
        }
        
        Ok(backend)
    }
    
    async fn load_model(&mut self) -> Result<()> {
        println!("[CraneBackend] Loading model: {}", self.model_id);
        println!("[CraneBackend] Cache directory: {:?}", self.cache_dir);
        println!("[CraneBackend] Use quantized: {}", self.use_quantized);
        
        if self.use_quantized {
            self.load_quantized_model().await
        } else {
            self.load_standard_model().await
        }
    }
    
    async fn load_quantized_model(&mut self) -> Result<()> {
        println!("[CraneBackend] Loading quantized GGUF model...");
        
        // Crane uses optimized GGUF models with better quantization for faster inference
        
        let api = Api::new()
            .map_err(|e| AppError::Llm(format!("Failed to create HF API: {}", e)))?;
        println!("[CraneBackend] HF API created successfully");
        
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
        
        // Try to get tokenizer with fallback mechanism
        let tokenizer_path = match original_repo.get("tokenizer.json").await {
            Ok(path) => {
                println!("[CraneBackend] Tokenizer downloaded from original repo");
                path
            }
            Err(e) => {
                println!("[CraneBackend] Failed to download from original repo: {}, trying manual path", e);
                
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
                    println!("[CraneBackend] Found tokenizer at manual path: {:?}", manual_path);
                    manual_path
                } else {
                    println!("[CraneBackend] Tokenizer not found at manual path: {:?}", manual_path);
                    // Try downloading from GGUF repo as last resort
                    println!("[CraneBackend] Trying GGUF repo as fallback");
                    repo.get("tokenizer.json").await
                        .map_err(|e2| {
                            eprintln!("[CraneBackend] GGUF tokenizer download also failed: {}", e2);
                            AppError::Llm(format!(
                                "Failed to download tokenizer from both repos. Original: {}, GGUF: {}", e, e2
                            ))
                        })?
                }
            }
        };
        
        // Load tokenizer
        println!("[CraneBackend] Loading tokenizer from: {:?}", tokenizer_path);
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| {
                eprintln!("[CraneBackend] Failed to load tokenizer from {:?}: {}", tokenizer_path, e);
                AppError::Llm(format!("Failed to load tokenizer: {}", e))
            })?;
        println!("[CraneBackend] Tokenizer loaded successfully");
        
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
        println!("[CraneBackend] Attempting to load GGUF model on {}", self.device_type);
        
        // Try loading with current device
        let load_result = QLlamaWeights::from_gguf(model_content, &mut reader, &self.device);
        
        let model_weights = match load_result {
            Ok(weights) => {
                println!("[CraneBackend] ✓ Model loaded successfully on {}", self.device_type);
                weights
            }
            Err(e) if !self.device.is_cpu() => {
                println!("[CraneBackend] ✗ Failed to load on {}: {}", self.device_type, e);
                
                // Check if it's a Metal-specific error
                let error_str = format!("{}", e);
                if error_str.contains("Metal error") || error_str.contains("rms-norm") {
                    println!("[CraneBackend] Metal doesn't support this operation for quantized models");
                }
                
                println!("[CraneBackend] → Attempting CPU fallback...");
                
                // Reopen the file for CPU fallback
                let mut file = std::fs::File::open(&model_path)
                    .map_err(|e| AppError::Llm(format!("Failed to reopen model file: {}", e)))?;
                let mut reader = std::io::BufReader::new(&mut file);
                
                // Re-read GGUF content
                let model_content = candle_core::quantized::gguf_file::Content::read(&mut reader)
                    .map_err(|e| AppError::Llm(format!("Failed to re-parse GGUF file: {}", e)))?;
                
                // Update device to CPU
                self.device = Device::Cpu;
                self.device_type = "CPU (fallback)".to_string();
                
                QLlamaWeights::from_gguf(model_content, &mut reader, &Device::Cpu)
                    .map_err(|cpu_err| AppError::Llm(format!(
                        "Failed to load model on both Metal and CPU. Metal error: {}, CPU error: {}", 
                        e, cpu_err
                    )))?
            }
            Err(e) => return Err(AppError::Llm(format!("Failed to load model weights on CPU: {}", e)))
        };
        
        println!("[CraneBackend] Storing model weights...");
        *self.model.lock().unwrap() = Some(ModelType::Quantized(model_weights));
        println!("[CraneBackend] Model weights stored");
        
        println!("[CraneBackend] Storing tokenizer...");
        self.tokenizer = Some(tokenizer);
        self.eos_token_id = eos_token_id;
        
        println!("[CraneBackend] Final state check:");
        println!("[CraneBackend]   Model loaded: {}", self.model.lock().unwrap().is_some());
        println!("[CraneBackend]   Tokenizer loaded: {}", self.tokenizer.is_some());
        println!("[CraneBackend]   EOS token ID: {:?}", self.eos_token_id);
        println!("[CraneBackend] Model loaded successfully on {}", self.device_type);
        
        Ok(())
    }
    
    async fn load_standard_model(&mut self) -> Result<()> {
        println!("[CraneBackend] Loading non-quantized model for Metal support...");
        
        let api = Api::new()
            .map_err(|e| AppError::Llm(format!("Failed to create HF API: {}", e)))?;
        
        // For non-quantized models, we load the standard safetensors
        let (model_files, config_file, tokenizer_repo) = match self.model_id.as_str() {
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => (
                vec!["model.safetensors"],
                "config.json",
                "TinyLlama/TinyLlama-1.1B-Chat-v1.0"
            ),
            _ => {
                // For now, only support TinyLlama in non-quantized mode
                println!("[CraneBackend] Model {} not configured for non-quantized mode, falling back to TinyLlama", self.model_id);
                self.model_id = "TinyLlama/TinyLlama-1.1B-Chat-v1.0".to_string();
                (
                    vec!["model.safetensors"],
                    "config.json",
                    "TinyLlama/TinyLlama-1.1B-Chat-v1.0"
                )
            }
        };
        
        let repo = api.repo(Repo::with_revision(
            self.model_id.clone(),
            RepoType::Model,
            "main".to_string(),
        ));
        
        // Download config
        println!("[CraneBackend] Downloading config...");
        let config_path = repo.get(config_file).await
            .map_err(|e| AppError::Llm(format!("Failed to download config: {}", e)))?;
        
        // Download model files
        println!("[CraneBackend] Downloading model files...");
        let mut model_paths = Vec::new();
        for file in model_files {
            let path = repo.get(file).await
                .map_err(|e| AppError::Llm(format!("Failed to download {}: {}", file, e)))?;
            model_paths.push(path);
        }
        
        // Download tokenizer
        println!("[CraneBackend] Downloading tokenizer...");
        let tokenizer_path = match repo.get("tokenizer.json").await {
            Ok(path) => {
                println!("[CraneBackend] Tokenizer downloaded successfully");
                path
            }
            Err(e) => {
                println!("[CraneBackend] Failed to download tokenizer: {}, trying fallback", e);
                
                // Try manual path construction as fallback
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
                    println!("[CraneBackend] Found tokenizer at manual path");
                    manual_path
                } else {
                    return Err(AppError::Llm(format!("Failed to download tokenizer: {}", e)));
                }
            }
        };
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AppError::Llm(format!("Failed to load tokenizer: {}", e)))?;
        
        // Set EOS token
        let eos_token_id = tokenizer.token_to_id("</s>")
            .or_else(|| tokenizer.token_to_id("<|endoftext|>"));
        
        self.tokenizer = Some(tokenizer);
        self.eos_token_id = eos_token_id;
        
        // Load config
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| AppError::Llm(format!("Failed to read config: {}", e)))?;
        let config_json: LlamaConfigJson = serde_json::from_str(&config_str)
            .map_err(|e| AppError::Llm(format!("Failed to parse config: {}", e)))?;
        
        // Convert to candle config
        let config = model::Config {
            hidden_size: config_json.hidden_size,
            intermediate_size: config_json.intermediate_size,
            vocab_size: config_json.vocab_size,
            num_hidden_layers: config_json.num_hidden_layers,
            num_attention_heads: config_json.num_attention_heads,
            num_key_value_heads: config_json.num_key_value_heads.unwrap_or(config_json.num_attention_heads),
            max_position_embeddings: config_json.max_position_embeddings,
            rms_norm_eps: config_json.rms_norm_eps,
            rope_theta: config_json.rope_theta.unwrap_or(10000.0),
            bos_token_id: config_json.bos_token_id,
            eos_token_id: config_json.eos_token_id.map(|id| model::LlamaEosToks::Single(id)),
            rope_scaling: None,
            tie_word_embeddings: config_json.tie_word_embeddings.unwrap_or(false),
            use_flash_attn: config_json.use_flash_attn.unwrap_or(false),
        };
        
        println!("[CraneBackend] Config loaded: hidden_size={}, num_attention_heads={}", 
            config.hidden_size, config.num_attention_heads);
        
        // Load model weights
        println!("[CraneBackend] Loading model weights on {:?}...", self.device);
        let dtype = if self.device.is_cuda() || self.device.is_metal() {
            DType::F32 // Use F32 for GPU/Metal
        } else {
            DType::F32 // Use F32 for CPU as well for non-quantized
        };
        
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&model_paths, dtype, &self.device)
                .map_err(|e| AppError::Llm(format!("Failed to load weights: {}", e)))?
        };
        
        // Create the model
        let model = model::Llama::load(vb, &config)
            .map_err(|e| AppError::Llm(format!("Failed to create model: {}", e)))?;
        
        // Create cache
        let cache = model::Cache::new(false, DType::F32, &config, &self.device)
            .map_err(|e| AppError::Llm(format!("Failed to create cache: {}", e)))?;
        
        println!("[CraneBackend] Storing non-quantized model...");
        *self.model.lock().unwrap() = Some(ModelType::Standard { model, cache });
        
        println!("[CraneBackend] Non-quantized model loaded successfully on {}!", self.device_type);
        
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
            
            let logits = match model {
                ModelType::Quantized(m) => m.forward(&input, pos)
                    .map_err(|e| AppError::Llm(format!("Model forward pass failed at position {}: {}", pos, e)))?,
                ModelType::Standard { model: m, cache } => m.forward(&input, pos, cache)
                    .map_err(|e| AppError::Llm(format!("Model forward pass failed at position {}: {}", pos, e)))?,
            };
            
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
                
                let logits = match model {
                    ModelType::Quantized(m) => m.forward(&input, tokens.len() + gen_pos)
                        .map_err(|e| AppError::Llm(format!("Forward pass failed: {}", e)))?
                        .squeeze(0)
                        .map_err(|e| AppError::Llm(format!("Failed to squeeze: {}", e)))?
                        .squeeze(0)
                        .map_err(|e| AppError::Llm(format!("Failed to squeeze: {}", e)))?,
                    ModelType::Standard { model: m, cache } => m.forward(&input, tokens.len() + gen_pos, cache)
                        .map_err(|e| AppError::Llm(format!("Forward pass failed: {}", e)))?
                        .squeeze(0)
                        .map_err(|e| AppError::Llm(format!("Failed to squeeze: {}", e)))?
                        .squeeze(0)
                        .map_err(|e| AppError::Llm(format!("Failed to squeeze: {}", e)))?,
                };
                
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