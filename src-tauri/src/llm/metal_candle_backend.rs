use crate::error::{AppError, Result};
use candle_core::{Device, DType, Tensor};
use candle_transformers::models::llama as model;
use candle_nn::VarBuilder;
use hf_hub::{api::tokio::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use std::path::PathBuf;

pub struct MetalCandleBackend {
    model_id: String,
    cache_dir: PathBuf,
    device: Device,
    model: Option<model::Llama>,
    tokenizer: Option<Tokenizer>,
    temperature: f64,
}

impl MetalCandleBackend {
    pub async fn new(
        model_id: &str,
        cache_dir: PathBuf,
    ) -> Result<Self> {
        println!("[MetalCandleBackend] Initializing with model: {}", model_id);
        
        // Create cache directory
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| AppError::Llm(format!("Failed to create cache dir: {}", e)))?;
        
        // Try to use Metal device
        let device = if cfg!(target_os = "macos") {
            match Device::new_metal(0) {
                Ok(metal_device) => {
                    println!("[MetalCandleBackend] ✓ Metal device initialized successfully!");
                    metal_device
                }
                Err(e) => {
                    println!("[MetalCandleBackend] Metal not available: {}, using CPU", e);
                    Device::Cpu
                }
            }
        } else {
            Device::Cpu
        };
        
        println!("[MetalCandleBackend] Using device: {:?}", device);
        
        let mut backend = Self {
            model_id: model_id.to_string(),
            cache_dir,
            device,
            model: None,
            tokenizer: None,
            temperature: 0.7,
        };
        
        // Load the model
        match backend.load_model().await {
            Ok(_) => {
                println!("[MetalCandleBackend] Model loaded successfully");
            }
            Err(e) => {
                eprintln!("[MetalCandleBackend] Failed to load model: {}", e);
                return Err(e);
            }
        }
        
        Ok(backend)
    }
    
    async fn load_model(&mut self) -> Result<()> {
        println!("[MetalCandleBackend] Loading non-quantized model for Metal support");
        
        let api = Api::new()
            .map_err(|e| AppError::Llm(format!("Failed to create HF API: {}", e)))?;
        
        // For Metal, we need to use non-quantized models
        // Start with smaller models that fit in memory
        let (model_id, config_file, model_files) = match self.model_id.as_str() {
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => {
                // Use the non-quantized version
                ("TinyLlama/TinyLlama-1.1B-Chat-v1.0", "config.json", vec!["model.safetensors"])
            }
            _ => {
                println!("[MetalCandleBackend] Model {} not configured for Metal, using TinyLlama", self.model_id);
                ("TinyLlama/TinyLlama-1.1B-Chat-v1.0", "config.json", vec!["model.safetensors"])
            }
        };
        
        let repo = api.repo(Repo::with_revision(
            model_id.to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        
        // Download config
        println!("[MetalCandleBackend] Downloading config...");
        let config_path = repo.get(config_file).await
            .map_err(|e| AppError::Llm(format!("Failed to download config: {}", e)))?;
        
        // Download model files
        println!("[MetalCandleBackend] Downloading model files...");
        let mut model_paths = Vec::new();
        for file in model_files {
            let path = repo.get(file).await
                .map_err(|e| AppError::Llm(format!("Failed to download {}: {}", file, e)))?;
            model_paths.push(path);
        }
        
        // Download tokenizer
        println!("[MetalCandleBackend] Downloading tokenizer...");
        let tokenizer_path = repo.get("tokenizer.json").await
            .map_err(|e| AppError::Llm(format!("Failed to download tokenizer: {}", e)))?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AppError::Llm(format!("Failed to load tokenizer: {}", e)))?;
        self.tokenizer = Some(tokenizer);
        
        // Load config
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| AppError::Llm(format!("Failed to read config: {}", e)))?;
        let config: model::Config = serde_json::from_str(&config_str)
            .map_err(|e| AppError::Llm(format!("Failed to parse config: {}", e)))?;
        
        // Load model weights
        println!("[MetalCandleBackend] Loading model weights on {:?}...", self.device);
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&model_paths, DType::F32, &self.device)
                .map_err(|e| AppError::Llm(format!("Failed to load weights: {}", e)))?
        };
        
        let model = model::Llama::load(vb, &config)
            .map_err(|e| AppError::Llm(format!("Failed to create model: {}", e)))?;
        
        self.model = Some(model);
        
        println!("[MetalCandleBackend] Model loaded successfully on {:?}!", self.device);
        
        Ok(())
    }
    
    pub async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        println!("[MetalCandleBackend] Generating with Metal acceleration...");
        
        let tokenizer = self.tokenizer.as_ref()
            .ok_or_else(|| AppError::Llm("Tokenizer not loaded".into()))?;
        
        let model = self.model.as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".into()))?;
        
        // Tokenize input
        let tokens = tokenizer.encode(prompt, true)
            .map_err(|e| AppError::Llm(format!("Failed to encode: {}", e)))?;
        let tokens = tokens.get_ids();
        
        // Convert to tensor
        let input = Tensor::new(tokens, &self.device)
            .map_err(|e| AppError::Llm(format!("Failed to create tensor: {}", e)))?;
        
        // Generate tokens
        let mut generated = Vec::new();
        let mut logits_processor = candle_transformers::generation::LogitsProcessor::new(
            299792458, // seed
            Some(self.temperature),
            None
        );
        
        // Process prompt
        let output = model.forward(&input, 0)
            .map_err(|e| AppError::Llm(format!("Forward pass failed: {}", e)))?;
        
        // Sample next token
        let logits = output.squeeze(0)
            .map_err(|e| AppError::Llm(format!("Failed to squeeze: {}", e)))?;
        let next_token = logits_processor.sample(&logits)
            .map_err(|e| AppError::Llm(format!("Failed to sample: {}", e)))?;
        
        generated.push(next_token);
        
        // Continue generation
        for _ in 0..max_tokens.min(50) {
            let input = Tensor::new(&[next_token], &self.device)
                .map_err(|e| AppError::Llm(format!("Failed to create tensor: {}", e)))?;
            
            let output = model.forward(&input, tokens.len() + generated.len() - 1)
                .map_err(|e| AppError::Llm(format!("Forward pass failed: {}", e)))?;
            
            let logits = output.squeeze(0)
                .map_err(|e| AppError::Llm(format!("Failed to squeeze: {}", e)))?;
            let next_token = logits_processor.sample(&logits)
                .map_err(|e| AppError::Llm(format!("Failed to sample: {}", e)))?;
            
            generated.push(next_token);
            
            // Check for EOS
            if next_token == 2 { // EOS token
                break;
            }
        }
        
        // Decode
        let text = tokenizer.decode(&generated, true)
            .map_err(|e| AppError::Llm(format!("Failed to decode: {}", e)))?;
        
        println!("[MetalCandleBackend] Generated {} tokens on Metal!", generated.len());
        
        Ok(text)
    }
}