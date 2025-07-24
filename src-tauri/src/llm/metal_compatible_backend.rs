use crate::error::{AppError, Result};
use candle_core::{Device, DType, Tensor};
use candle_nn::{VarBuilder, Module};
use candle_transformers::models::{bert, distilbert, t5};
use hf_hub::{api::tokio::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use std::path::PathBuf;

/// Backend for models that work well with Metal acceleration
/// These models use LayerNorm instead of RMS norm, which is fully supported on Metal
pub struct MetalCompatibleBackend {
    model_type: ModelType,
    cache_dir: PathBuf,
    device: Device,
    tokenizer: Option<Tokenizer>,
}

pub enum ModelType {
    Bert(bert::BertModel),
    DistilBert(distilbert::DistilBertModel),
    T5Encoder(t5::T5EncoderModel),
}

impl MetalCompatibleBackend {
    pub async fn new(
        model_id: &str,
        cache_dir: PathBuf,
    ) -> Result<Self> {
        println!("[MetalCompatibleBackend] Initializing with model: {}", model_id);
        
        // Create cache directory
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| AppError::Llm(format!("Failed to create cache dir: {}", e)))?;
        
        // Use Metal device if available
        let device = if cfg!(target_os = "macos") {
            match Device::new_metal(0) {
                Ok(metal_device) => {
                    println!("[MetalCompatibleBackend] ✓ Metal device initialized successfully!");
                    metal_device
                }
                Err(e) => {
                    println!("[MetalCompatibleBackend] Metal not available: {}, using CPU", e);
                    Device::Cpu
                }
            }
        } else {
            Device::Cpu
        };
        
        println!("[MetalCompatibleBackend] Using device: {:?}", device);
        
        let mut backend = Self {
            model_type: ModelType::Bert(bert::BertModel::new(
                &bert::Config::default(), 
                VarBuilder::zeros(DType::F32, &device)
            ).unwrap()), // Placeholder, will be replaced
            cache_dir,
            device,
            tokenizer: None,
        };
        
        // Load the model
        backend.load_model(model_id).await?;
        
        Ok(backend)
    }
    
    async fn load_model(&mut self, model_id: &str) -> Result<()> {
        println!("[MetalCompatibleBackend] Loading model: {}", model_id);
        
        let api = Api::new()
            .map_err(|e| AppError::Llm(format!("Failed to create HF API: {}", e)))?;
        
        let repo = api.repo(Repo::with_revision(
            model_id.to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        
        // Download config
        println!("[MetalCompatibleBackend] Downloading config...");
        let config_path = repo.get("config.json").await
            .map_err(|e| AppError::Llm(format!("Failed to download config: {}", e)))?;
        
        // Download tokenizer
        println!("[MetalCompatibleBackend] Downloading tokenizer...");
        let tokenizer_path = repo.get("tokenizer.json").await
            .map_err(|e| AppError::Llm(format!("Failed to download tokenizer: {}", e)))?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AppError::Llm(format!("Failed to load tokenizer: {}", e)))?;
        self.tokenizer = Some(tokenizer);
        
        // Determine model type and load accordingly
        if model_id.contains("bert-base") || model_id.contains("sentence-transformers") {
            self.load_bert_model(&repo, &config_path).await?;
        } else if model_id.contains("distilbert") {
            self.load_distilbert_model(&repo, &config_path).await?;
        } else if model_id.contains("t5") || model_id.contains("flan-t5") {
            self.load_t5_model(&repo, &config_path).await?;
        } else {
            // Default to BERT-style model
            self.load_bert_model(&repo, &config_path).await?;
        }
        
        println!("[MetalCompatibleBackend] Model loaded successfully on {:?}!", self.device);
        
        Ok(())
    }
    
    async fn load_bert_model(&mut self, repo: &Repo, config_path: &std::path::Path) -> Result<()> {
        // Download model weights
        println!("[MetalCompatibleBackend] Loading BERT model...");
        let model_path = repo.get("model.safetensors").await
            .or_else(|_| repo.get("pytorch_model.bin"))
            .await
            .map_err(|e| AppError::Llm(format!("Failed to download model: {}", e)))?;
        
        // Load config
        let config_str = std::fs::read_to_string(config_path)
            .map_err(|e| AppError::Llm(format!("Failed to read config: {}", e)))?;
        let mut config: bert::Config = serde_json::from_str(&config_str)
            .map_err(|e| AppError::Llm(format!("Failed to parse config: {}", e)))?;
        
        // Adjust config for inference
        config.hidden_dropout_prob = 0.0;
        config.attention_probs_dropout_prob = 0.0;
        
        // Load weights
        let vb = if model_path.to_string_lossy().ends_with(".safetensors") {
            unsafe {
                VarBuilder::from_mmaped_safetensors(&[model_path], DType::F32, &self.device)
                    .map_err(|e| AppError::Llm(format!("Failed to load weights: {}", e)))?
            }
        } else {
            VarBuilder::from_pth(model_path, DType::F32, &self.device)
                .map_err(|e| AppError::Llm(format!("Failed to load weights: {}", e)))?
        };
        
        let model = bert::BertModel::new(&config, vb)
            .map_err(|e| AppError::Llm(format!("Failed to create BERT model: {}", e)))?;
        
        self.model_type = ModelType::Bert(model);
        Ok(())
    }
    
    async fn load_distilbert_model(&mut self, repo: &Repo, config_path: &std::path::Path) -> Result<()> {
        // Download model weights
        println!("[MetalCompatibleBackend] Loading DistilBERT model...");
        let model_path = repo.get("model.safetensors").await
            .or_else(|_| repo.get("pytorch_model.bin"))
            .await
            .map_err(|e| AppError::Llm(format!("Failed to download model: {}", e)))?;
        
        // Load config
        let config_str = std::fs::read_to_string(config_path)
            .map_err(|e| AppError::Llm(format!("Failed to read config: {}", e)))?;
        let config: distilbert::Config = serde_json::from_str(&config_str)
            .map_err(|e| AppError::Llm(format!("Failed to parse config: {}", e)))?;
        
        // Load weights
        let vb = if model_path.to_string_lossy().ends_with(".safetensors") {
            unsafe {
                VarBuilder::from_mmaped_safetensors(&[model_path], DType::F32, &self.device)
                    .map_err(|e| AppError::Llm(format!("Failed to load weights: {}", e)))?
            }
        } else {
            VarBuilder::from_pth(model_path, DType::F32, &self.device)
                .map_err(|e| AppError::Llm(format!("Failed to load weights: {}", e)))?
        };
        
        let model = distilbert::DistilBertModel::new(&config, vb)
            .map_err(|e| AppError::Llm(format!("Failed to create DistilBERT model: {}", e)))?;
        
        self.model_type = ModelType::DistilBert(model);
        Ok(())
    }
    
    async fn load_t5_model(&mut self, repo: &Repo, config_path: &std::path::Path) -> Result<()> {
        // Download model weights
        println!("[MetalCompatibleBackend] Loading T5 model...");
        let model_path = repo.get("model.safetensors").await
            .or_else(|_| repo.get("pytorch_model.bin"))
            .await
            .map_err(|e| AppError::Llm(format!("Failed to download model: {}", e)))?;
        
        // Load config
        let config_str = std::fs::read_to_string(config_path)
            .map_err(|e| AppError::Llm(format!("Failed to read config: {}", e)))?;
        let mut config: t5::Config = serde_json::from_str(&config_str)
            .map_err(|e| AppError::Llm(format!("Failed to parse config: {}", e)))?;
        
        // Adjust config for inference
        config.use_cache = false; // Disable cache for simplicity
        
        // Load weights
        let vb = if model_path.to_string_lossy().ends_with(".safetensors") {
            unsafe {
                VarBuilder::from_mmaped_safetensors(&[model_path], DType::F32, &self.device)
                    .map_err(|e| AppError::Llm(format!("Failed to load weights: {}", e)))?
            }
        } else {
            VarBuilder::from_pth(model_path, DType::F32, &self.device)
                .map_err(|e| AppError::Llm(format!("Failed to load weights: {}", e)))?
        };
        
        // For now, we'll use T5 encoder for embeddings
        let model = t5::T5EncoderModel::new(&config, vb)
            .map_err(|e| AppError::Llm(format!("Failed to create T5 model: {}", e)))?;
        
        self.model_type = ModelType::T5Encoder(model);
        Ok(())
    }
    
    pub async fn get_embeddings(&self, text: &str) -> Result<Vec<f32>> {
        let tokenizer = self.tokenizer.as_ref()
            .ok_or_else(|| AppError::Llm("Tokenizer not loaded".into()))?;
        
        // Tokenize input
        let encoding = tokenizer.encode(text, true)
            .map_err(|e| AppError::Llm(format!("Failed to encode: {}", e)))?;
        let tokens = encoding.get_ids();
        
        // Convert to tensor
        let input_ids = Tensor::new(tokens, &self.device)
            .map_err(|e| AppError::Llm(format!("Failed to create tensor: {}", e)))?
            .unsqueeze(0)?; // Add batch dimension
        
        // Get embeddings based on model type
        let embeddings = match &self.model_type {
            ModelType::Bert(model) => {
                let output = model.forward(&input_ids)?;
                // Use CLS token embedding (first token)
                output.i((0, 0))?
            }
            ModelType::DistilBert(model) => {
                let output = model.forward(&input_ids)?;
                // Use CLS token embedding
                output.i((0, 0))?
            }
            ModelType::T5Encoder(model) => {
                let output = model.forward(&input_ids)?;
                // Average pooling over sequence
                output.mean(1)?.squeeze(0)?
            }
        };
        
        // Convert to Vec<f32>
        let embeddings_vec = embeddings.to_vec1::<f32>()
            .map_err(|e| AppError::Llm(format!("Failed to convert embeddings: {}", e)))?;
        
        Ok(embeddings_vec)
    }
    
    pub async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        // For BERT/DistilBERT, we can't generate text, only embeddings
        // For T5, we would need the full encoder-decoder model
        match &self.model_type {
            ModelType::Bert(_) | ModelType::DistilBert(_) => {
                Err(AppError::Llm("BERT/DistilBERT models are for embeddings only, not text generation".into()))
            }
            ModelType::T5Encoder(_) => {
                Err(AppError::Llm("This T5 encoder is for embeddings only. Full T5 model needed for generation".into()))
            }
        }
    }
}

/// Example usage for different models that work well with Metal
pub mod examples {
    use super::*;
    
    pub async fn bert_embeddings_example() -> Result<()> {
        // BERT for embeddings
        let model = MetalCompatibleBackend::new(
            "bert-base-uncased",
            PathBuf::from("./models/bert"),
        ).await?;
        
        let embeddings = model.get_embeddings("Hello, world!").await?;
        println!("BERT embeddings dimension: {}", embeddings.len());
        
        Ok(())
    }
    
    pub async fn sentence_transformers_example() -> Result<()> {
        // Sentence transformers (BERT-based)
        let model = MetalCompatibleBackend::new(
            "sentence-transformers/all-MiniLM-L6-v2",
            PathBuf::from("./models/minilm"),
        ).await?;
        
        let embeddings = model.get_embeddings("This is a test sentence.").await?;
        println!("Sentence embeddings dimension: {}", embeddings.len());
        
        Ok(())
    }
    
    pub async fn distilbert_example() -> Result<()> {
        // DistilBERT (smaller, faster BERT)
        let model = MetalCompatibleBackend::new(
            "distilbert-base-uncased",
            PathBuf::from("./models/distilbert"),
        ).await?;
        
        let embeddings = model.get_embeddings("DistilBERT is fast!").await?;
        println!("DistilBERT embeddings dimension: {}", embeddings.len());
        
        Ok(())
    }
}