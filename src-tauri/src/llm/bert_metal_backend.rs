use crate::error::{AppError, Result};
use candle_core::{Device, Tensor, DType};
use candle_transformers::models::bert::{BertModel, Config};
use candle_nn::VarBuilder;
use hf_hub::{api::tokio::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
pub struct BertMetalBackend {
    model_id: String,
    cache_dir: PathBuf,
    device: Device,
    device_type: String,
    model: Arc<Mutex<Option<BertModel>>>,
    tokenizer: Option<Tokenizer>,
    config: Option<Config>,
}

impl BertMetalBackend {
    pub async fn new(
        model_id: &str,
        cache_dir: PathBuf,
    ) -> Result<Self> {
        println!("[BertMetalBackend] Initializing with model: {}", model_id);
        
        // Create cache directory
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| AppError::Llm(format!("Failed to create cache dir: {}", e)))?;
        
        // Check if we can use Metal
        let use_metal = std::env::var("CANDLE_USE_METAL")
            .map(|v| v != "0")
            .unwrap_or(true);
        
        let (device, device_type) = if cfg!(target_os = "macos") && use_metal {
            match Device::new_metal(0) {
                Ok(metal_device) => {
                    println!("[BertMetalBackend] ✓ Metal device initialized successfully!");
                    (metal_device, "Metal".to_string())
                }
                Err(e) => {
                    println!("[BertMetalBackend] Metal not available: {}, using CPU", e);
                    (Device::Cpu, "CPU".to_string())
                }
            }
        } else {
            (Device::Cpu, "CPU".to_string())
        };
        
        println!("[BertMetalBackend] Using device: {}", device_type);
        
        let mut backend = Self {
            model_id: model_id.to_string(),
            cache_dir,
            device,
            device_type,
            model: Arc::new(Mutex::new(None)),
            tokenizer: None,
            config: None,
        };
        
        // Load the model
        match backend.load_model().await {
            Ok(_) => {
                println!("[BertMetalBackend] Model loaded successfully on {}", backend.device_type);
            }
            Err(e) => {
                eprintln!("[BertMetalBackend] Failed to load model: {}", e);
                return Err(e);
            }
        }
        
        Ok(backend)
    }
    
    async fn load_model(&mut self) -> Result<()> {
        println!("[BertMetalBackend] Loading BERT model for Metal acceleration...");
        
        let api = Api::new()
            .map_err(|e| AppError::Llm(format!("Failed to create HF API: {}", e)))?;
        
        // Popular BERT models that work well
        let (model_id, is_sentence_transformer) = match self.model_id.as_str() {
            // Sentence transformers (great for embeddings)
            "sentence-transformers/all-MiniLM-L6-v2" => (self.model_id.as_str(), true),
            "sentence-transformers/all-mpnet-base-v2" => (self.model_id.as_str(), true),
            // Standard BERT models
            "bert-base-uncased" => (self.model_id.as_str(), false),
            "bert-base-cased" => (self.model_id.as_str(), false),
            "bert-base-multilingual-cased" => (self.model_id.as_str(), false),
            "distilbert-base-uncased" => (self.model_id.as_str(), false),
            _ => {
                println!("[BertMetalBackend] Using default: bert-base-uncased");
                ("bert-base-uncased", false)
            }
        };
        
        let repo = api.repo(Repo::with_revision(
            model_id.to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        
        // Download config
        println!("[BertMetalBackend] Downloading config...");
        let config_path = repo.get("config.json").await
            .map_err(|e| AppError::Llm(format!("Failed to download config: {}", e)))?;
        
        // Download model weights
        println!("[BertMetalBackend] Downloading model weights...");
        let weights_file = if is_sentence_transformer {
            "pytorch_model.bin" // Sentence transformers often use this
        } else {
            "model.safetensors" // Newer format
        };
        
        let model_path = match repo.get(weights_file).await {
            Ok(path) => path,
            Err(_) => {
                // Try alternative format
                let alt_file = if weights_file == "model.safetensors" {
                    "pytorch_model.bin"
                } else {
                    "model.safetensors"
                };
                repo.get(alt_file).await
                    .map_err(|e| AppError::Llm(format!("Failed to download model weights: {}", e)))?
            }
        };
        
        // Download tokenizer
        println!("[BertMetalBackend] Downloading tokenizer...");
        let tokenizer_path = repo.get("tokenizer.json").await
            .map_err(|e| AppError::Llm(format!("Failed to download tokenizer: {}", e)))?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AppError::Llm(format!("Failed to load tokenizer: {}", e)))?;
        self.tokenizer = Some(tokenizer);
        
        // Load config
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| AppError::Llm(format!("Failed to read config: {}", e)))?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| AppError::Llm(format!("Failed to parse config: {}", e)))?;
        
        // For sentence transformers, we might need mean pooling
        if is_sentence_transformer {
            println!("[BertMetalBackend] Configuring for sentence transformers (embeddings)");
        }
        
        self.config = Some(config.clone());
        
        println!("[BertMetalBackend] Config loaded - hidden_size: {}", config.hidden_size);
        
        // Load model weights
        println!("[BertMetalBackend] Loading model weights on {:?}...", self.device);
        
        let vb = if model_path.to_string_lossy().ends_with(".safetensors") {
            unsafe {
                VarBuilder::from_mmaped_safetensors(&[model_path], DType::F32, &self.device)
                    .map_err(|e| AppError::Llm(format!("Failed to load weights: {}", e)))?
            }
        } else {
            VarBuilder::from_pth(model_path, DType::F32, &self.device)
                .map_err(|e| AppError::Llm(format!("Failed to load weights: {}", e)))?
        };
        
        // Create the model
        let model = BertModel::load(vb, &config)
            .map_err(|e| AppError::Llm(format!("Failed to create model: {}", e)))?;
        
        *self.model.lock().unwrap() = Some(model);
        
        println!("[BertMetalBackend] Model loaded successfully on {}!", self.device_type);
        
        Ok(())
    }
    
    pub async fn generate_embeddings(&self, text: &str) -> Result<Vec<f32>> {
        println!("[BertMetalBackend] Generating embeddings on {}...", self.device_type);
        
        let tokenizer = self.tokenizer.as_ref()
            .ok_or_else(|| AppError::Llm("Tokenizer not loaded".into()))?;
        
        let model = self.model.lock().unwrap();
        let model = model.as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".into()))?;
        
        // Tokenize input
        let encoding = tokenizer.encode(text, true)
            .map_err(|e| AppError::Llm(format!("Failed to encode: {}", e)))?;
        let tokens = encoding.get_ids();
        
        // Convert to tensor
        let token_ids = Tensor::new(tokens, &self.device)
            .map_err(|e| AppError::Llm(format!("Failed to create tensor: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| AppError::Llm(format!("Failed to unsqueeze: {}", e)))?; // Add batch dimension
        
        let token_type_ids = token_ids.zeros_like()
            .map_err(|e| AppError::Llm(format!("Failed to create zeros: {}", e)))?;
        
        // Forward pass (BERT takes optional attention mask as 3rd param)
        let output = model.forward(&token_ids, &token_type_ids, None)
            .map_err(|e| AppError::Llm(format!("Forward pass failed: {}", e)))?;
        
        // For embeddings, we typically use mean pooling over the sequence
        let embeddings = output.mean(1)
            .map_err(|e| AppError::Llm(format!("Mean pooling failed: {}", e)))?; // Average over sequence length
        
        // Convert to Vec<f32>
        let embeddings_vec = embeddings.squeeze(0)
            .map_err(|e| AppError::Llm(format!("Squeeze failed: {}", e)))?
            .to_vec1::<f32>()
            .map_err(|e| AppError::Llm(format!("To vec failed: {}", e)))?;
        
        println!("[BertMetalBackend] Generated {} dimensional embeddings", embeddings_vec.len());
        
        Ok(embeddings_vec)
    }
    
    pub async fn generate(&self, prompt: &str, _max_tokens: usize) -> Result<String> {
        // BERT is not designed for text generation, it's for embeddings/classification
        // But we can demonstrate it works on Metal by returning embeddings info
        
        let embeddings = self.generate_embeddings(prompt).await?;
        
        Ok(format!(
            "[BERT on {}] Generated {}-dimensional embeddings for input. \
            BERT models are designed for embeddings and classification, not text generation. \
            First few values: {:?}...",
            self.device_type,
            embeddings.len(),
            &embeddings[..5.min(embeddings.len())]
        ))
    }
}

#[derive(Debug, serde::Serialize)]
pub struct BertModelInfo {
    pub model_type: String,
    pub device: String,
    pub model_loaded: bool,
    pub tokenizer_loaded: bool,
    pub hidden_size: usize,
    pub supports_metal: bool,
}