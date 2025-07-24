use crate::config::{Config, InferenceProvider};
use crate::llm::LlmClient;
use std::sync::Arc;
use tauri::{AppHandle, State, Emitter};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InferenceConfig {
    pub provider: String,
    pub ollama_model: String,
    pub candle_model_id: String,
    pub candle_model_revision: String,
    pub available_providers: Vec<String>,
}

#[tauri::command]
pub async fn get_inference_config(
    _app_handle: AppHandle,
) -> std::result::Result<InferenceConfig, String> {
    let config = Config::get();
    
    Ok(InferenceConfig {
        provider: format!("{:?}", config.services.inference_provider),
        ollama_model: config.services.ollama_model.clone(),
        candle_model_id: config.services.candle_model_id.clone(),
        candle_model_revision: config.services.candle_model_revision.clone(),
        available_providers: vec!["Ollama".to_string(), "Candle".to_string(), "Crane".to_string(), "Callm".to_string(), "LlamaCpp".to_string()],
    })
}

#[tauri::command]
pub async fn set_inference_provider(
    app_handle: AppHandle,
    provider: String,
    model_id: Option<String>,
) -> std::result::Result<(), String> {
    println!("[Inference] Setting provider to: {}", provider);
    
    // Parse the provider
    let inference_provider = match provider.to_lowercase().as_str() {
        "candle" => InferenceProvider::Candle,
        "crane" => InferenceProvider::Crane,
        "callm" => InferenceProvider::Callm,
        "ollama" => InferenceProvider::Ollama,
        "llamacpp" => InferenceProvider::LlamaCpp,
        _ => return Err(format!("Unknown provider: {}", provider)),
    };
    
    // Update the configuration using the config manager
    crate::services::config_manager::update_inference_config(
        &app_handle,
        inference_provider.clone(),
        model_id.clone(),
    )
    .await
    .map_err(|e| format!("Failed to update config: {}", e))?;
    
    println!("[Inference] Configuration saved successfully");
    
    // Also update environment variables for current session
    match provider.to_lowercase().as_str() {
        "candle" => {
            std::env::set_var("INFERENCE_PROVIDER", "candle");
            if let Some(id) = &model_id {
                std::env::set_var("CANDLE_MODEL_ID", id);
            }
        }
        "crane" => {
            std::env::set_var("INFERENCE_PROVIDER", "crane");
            if let Some(id) = &model_id {
                std::env::set_var("CANDLE_MODEL_ID", id); // Crane reuses Candle model configs
            }
        }
        "callm" => {
            std::env::set_var("INFERENCE_PROVIDER", "callm");
            if let Some(id) = &model_id {
                std::env::set_var("CANDLE_MODEL_ID", id); // Callm reuses Candle model configs
            }
        }
        "ollama" => {
            std::env::set_var("INFERENCE_PROVIDER", "ollama");
            if let Some(id) = &model_id {
                std::env::set_var("OLLAMA_MODEL", id);
            }
        }
        "llamacpp" => {
            std::env::set_var("INFERENCE_PROVIDER", "llamacpp");
            if let Some(id) = &model_id {
                std::env::set_var("CANDLE_MODEL_ID", id); // LlamaCpp also uses model_id
            }
        }
        _ => {}
    }
    
    // Note: The LLM client will pick up the new configuration on the next request
    // since it reads from Config::get() which uses environment variables we just set
    println!("[Inference] Inference provider configuration updated successfully!");
    println!("[Inference] Changes will take effect on the next LLM request.");
    
    Ok(())
}

#[tauri::command]
pub async fn get_inference_info(
    llm: State<'_, Arc<LlmClient>>,
) -> std::result::Result<crate::llm::InferenceInfo, String> {
    Ok(llm.get_inference_info().await)
}

#[tauri::command]
pub async fn get_candle_models() -> std::result::Result<Vec<CandleModel>, String> {
    use crate::config::Config;
    use hf_hub::Cache;
    
    let config = Config::get();
    let cache_dir = std::path::Path::new(&config.services.candle_cache_dir);
    let hf_cache = Cache::new(cache_dir.to_path_buf());
    
    // List of models
    let models = vec![
        ("Qwen/Qwen2.5-0.5B-Instruct", "Qwen2.5 0.5B 🚀", "Smallest & fastest Qwen model", "350 MB"),
        ("Qwen/Qwen2.5-1.5B-Instruct", "Qwen2.5 1.5B", "Balanced performance Qwen model", "1.0 GB"),
        ("Qwen/Qwen2.5-3B-Instruct", "Qwen2.5 3B", "High-quality Qwen model for complex tasks", "2.0 GB"),
        ("Qwen/Qwen2.5-7B-Instruct", "Qwen2.5 7B", "Most capable Qwen model with excellent performance", "4.5 GB"),
        ("TinyLlama/TinyLlama-1.1B-Chat-v1.0", "TinyLlama 1.1B", "Tiny but efficient chat model", "650 MB"),
        ("microsoft/phi-2", "Phi-2 (2.7B)", "Small but capable model from Microsoft", "1.5 GB"),
    ];
    
    let mut candle_models = Vec::new();
    
    for (id, name, desc, size) in models {
        // Check if GGUF versions exist in cache (for both Candle and Crane)
        let mut downloaded = false;
        let mut download_path = None;
        
        println!("[get_candle_models] Checking download status for {}", id);
        
        // Check for GGUF versions used by Crane/Candle
        let (gguf_repo, gguf_file) = match id {
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => ("TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF", "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf"),
            "Qwen/Qwen2.5-0.5B-Instruct" => ("Qwen/Qwen2.5-0.5B-Instruct-GGUF", "qwen2.5-0.5b-instruct-q4_k_m.gguf"),
            "Qwen/Qwen2.5-1.5B-Instruct" => ("Qwen/Qwen2.5-1.5B-Instruct-GGUF", "qwen2.5-1.5b-instruct-q4_k_m.gguf"),
            "Qwen/Qwen2.5-3B-Instruct" => ("Qwen/Qwen2.5-3B-Instruct-GGUF", "qwen2.5-3b-instruct-q4_k_m.gguf"),
            "Qwen/Qwen2.5-7B-Instruct" => ("Qwen/Qwen2.5-7B-Instruct-GGUF", "qwen2.5-7b-instruct-q4_k_m.gguf"),
            "microsoft/phi-2" => ("TheBloke/phi-2-GGUF", "phi-2.Q4_K_M.gguf"),
            _ => {
                // For other models, check the original repo for GGUF files
                let repo = hf_cache.repo(hf_hub::Repo::model(id.to_string()));
                if let Some(path) = repo.get("ggml-model-q4_0.gguf") {
                    if path.exists() {
                        downloaded = true;
                        download_path = Some(path.to_string_lossy().to_string());
                        println!("[get_candle_models] Found ggml-model-q4_0.gguf for {}", id);
                    }
                }
                ("", "")
            }
        };
        
        if !gguf_repo.is_empty() && !downloaded {
            println!("[get_candle_models] Checking GGUF repo: {} for file: {}", gguf_repo, gguf_file);
            
            // Try direct path construction as a fallback
            let repo_dir = cache_dir.join(format!("models--{}", gguf_repo.replace("/", "--")));
            println!("[get_candle_models] Looking for repo directory: {:?}", repo_dir);
            
            if repo_dir.exists() {
                // Look for snapshots directory
                let snapshots_dir = repo_dir.join("snapshots");
                if snapshots_dir.exists() {
                    println!("[get_candle_models] Found snapshots directory");
                    // Check each snapshot for the file
                    if let Ok(entries) = std::fs::read_dir(&snapshots_dir) {
                        for entry in entries.flatten() {
                            let snapshot_path = entry.path();
                            if snapshot_path.is_dir() {
                                let file_path = snapshot_path.join(gguf_file);
                                println!("[get_candle_models] Checking: {:?}", file_path);
                                if file_path.exists() || (file_path.is_symlink() && file_path.metadata().is_ok()) {
                                    downloaded = true;
                                    download_path = Some(file_path.to_string_lossy().to_string());
                                    println!("[get_candle_models] Found GGUF file at: {:?}", file_path);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            
            // Also try the hf_cache API
            if !downloaded {
                let repo = hf_cache.repo(hf_hub::Repo::model(gguf_repo.to_string()));
                if let Some(path) = repo.get(gguf_file) {
                // The path might be a symlink, so we need to check if the target exists
                let exists = if path.is_symlink() {
                    match std::fs::read_link(&path) {
                        Ok(target) => {
                            // Check if the symlink target exists
                            let full_target = if target.is_absolute() {
                                target
                            } else {
                                path.parent().unwrap_or(std::path::Path::new(".")).join(target)
                            };
                            full_target.exists()
                        }
                        Err(_) => false,
                    }
                } else {
                    path.exists()
                };
                
                if exists {
                    downloaded = true;
                    download_path = Some(path.to_string_lossy().to_string());
                    println!("[get_candle_models] Found GGUF file at: {:?}", path);
                } else {
                    println!("[get_candle_models] GGUF file path exists but file not found: {:?}", path);
                }
                } else {
                    println!("[get_candle_models] No path found for GGUF file: {}", gguf_file);
                }
            }
        }
        
        // Also check if the tokenizer exists
        if downloaded {
            let tokenizer_repo = hf_cache.repo(hf_hub::Repo::model(id.to_string()));
            if let Some(tokenizer_path) = tokenizer_repo.get("tokenizer.json") {
                if !tokenizer_path.exists() {
                    println!("[get_candle_models] Warning: Model {} downloaded but tokenizer missing", id);
                }
            }
        }
        
        candle_models.push(CandleModel {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            size: size.to_string(),
            downloaded,
            download_path: download_path.clone(),
        });
        
        println!("[get_candle_models] Model {} - Downloaded: {}, Path: {:?}", id, downloaded, download_path);
    }
    
    println!("[get_candle_models] Total models: {}, Downloaded: {}", 
        candle_models.len(), 
        candle_models.iter().filter(|m| m.downloaded).count()
    );
    
    Ok(candle_models)
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CandleModel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size: String,
    pub downloaded: bool,
    pub download_path: Option<String>,
}

#[tauri::command]
pub async fn get_config_path(
    app_handle: AppHandle,
) -> std::result::Result<String, String> {
    crate::services::config_manager::get_config_path(&app_handle)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to get config path: {}", e))
}

#[derive(Clone, serde::Serialize)]
struct DownloadProgress {
    model_id: String,
    status: String,
    progress: f32,
    message: String,
}

#[tauri::command]
pub async fn download_model(
    model_id: String,
    app_handle: AppHandle,
) -> std::result::Result<String, String> {
    use hf_hub::{api::tokio::Api, Repo, RepoType};
    
    println!("[Inference] Downloading model: {}", model_id);
    
    // Emit initial progress
    app_handle.emit("download-progress", DownloadProgress {
        model_id: model_id.clone(),
        status: "starting".to_string(),
        progress: 0.0,
        message: "Initializing download...".to_string(),
    }).ok();
    
    let api = Api::new()
        .map_err(|e| format!("Failed to create HF API: {}", e))?;
    
    // Download files based on model
    let mut downloaded_files = Vec::new();
    
    match model_id.as_str() {
        "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => {
            // For TinyLlama, use TheBloke's GGUF version
            let gguf_repo = api.repo(Repo::with_revision(
                "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF".to_string(),
                RepoType::Model,
                "main".to_string(),
            ));
            
            println!("[Inference] Downloading GGUF model...");
            app_handle.emit("download-progress", DownloadProgress {
                model_id: model_id.clone(),
                status: "downloading".to_string(),
                progress: 10.0,
                message: "Downloading GGUF model file (this may take a few minutes)...".to_string(),
            }).ok();
            
            match gguf_repo.get("tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf").await {
                Ok(path) => {
                    println!("[Inference] Downloaded GGUF to {:?}", path);
                    downloaded_files.push(path.to_string_lossy().to_string());
                    
                    app_handle.emit("download-progress", DownloadProgress {
                        model_id: model_id.clone(),
                        status: "downloading".to_string(),
                        progress: 80.0,
                        message: "GGUF model downloaded, fetching tokenizer...".to_string(),
                    }).ok();
                }
                Err(e) => {
                    eprintln!("[Inference] Failed to download GGUF: {}", e);
                    app_handle.emit("download-progress", DownloadProgress {
                        model_id: model_id.clone(),
                        status: "error".to_string(),
                        progress: 0.0,
                        message: format!("Failed to download GGUF: {}", e),
                    }).ok();
                }
            }
            
            // Download tokenizer from original repo
            let tokenizer_repo = api.repo(Repo::with_revision(
                model_id.clone(),
                RepoType::Model,
                "main".to_string(),
            ));
            
            println!("[Inference] Downloading tokenizer...");
            match tokenizer_repo.get("tokenizer.json").await {
                Ok(path) => {
                    println!("[Inference] Downloaded tokenizer to {:?}", path);
                    downloaded_files.push(path.to_string_lossy().to_string());
                    
                    app_handle.emit("download-progress", DownloadProgress {
                        model_id: model_id.clone(),
                        status: "completed".to_string(),
                        progress: 100.0,
                        message: "Model and tokenizer downloaded successfully!".to_string(),
                    }).ok();
                }
                Err(e) => {
                    eprintln!("[Inference] Failed to download tokenizer: {}", e);
                }
            }
        }
        model if model.starts_with("Qwen/Qwen2.5") => {
            // For Qwen models, use their GGUF versions
            let (gguf_repo_id, gguf_filename) = match model_id.as_str() {
                "Qwen/Qwen2.5-0.5B-Instruct" => ("Qwen/Qwen2.5-0.5B-Instruct-GGUF", "qwen2.5-0.5b-instruct-q4_k_m.gguf"),
                "Qwen/Qwen2.5-1.5B-Instruct" => ("Qwen/Qwen2.5-1.5B-Instruct-GGUF", "qwen2.5-1.5b-instruct-q4_k_m.gguf"),
                "Qwen/Qwen2.5-3B-Instruct" => ("Qwen/Qwen2.5-3B-Instruct-GGUF", "qwen2.5-3b-instruct-q4_k_m.gguf"),
                "Qwen/Qwen2.5-7B-Instruct" => ("Qwen/Qwen2.5-7B-Instruct-GGUF", "qwen2.5-7b-instruct-q4_k_m.gguf"),
                _ => (model_id.as_str(), "ggml-model-q4_0.gguf")
            };
            
            let gguf_repo = api.repo(Repo::with_revision(
                gguf_repo_id.to_string(),
                RepoType::Model,
                "main".to_string(),
            ));
            
            println!("[Inference] Downloading GGUF model from {}...", gguf_repo_id);
            app_handle.emit("download-progress", DownloadProgress {
                model_id: model_id.clone(),
                status: "downloading".to_string(),
                progress: 10.0,
                message: format!("Downloading {} model file...", gguf_filename),
            }).ok();
            
            match gguf_repo.get(gguf_filename).await {
                Ok(path) => {
                    println!("[Inference] Downloaded GGUF to {:?}", path);
                    downloaded_files.push(path.to_string_lossy().to_string());
                    
                    app_handle.emit("download-progress", DownloadProgress {
                        model_id: model_id.clone(),
                        status: "downloading".to_string(),
                        progress: 80.0,
                        message: "Model downloaded, fetching tokenizer...".to_string(),
                    }).ok();
                }
                Err(e) => {
                    eprintln!("[Inference] Failed to download GGUF: {}", e);
                    app_handle.emit("download-progress", DownloadProgress {
                        model_id: model_id.clone(),
                        status: "error".to_string(),
                        progress: 0.0,
                        message: format!("Failed to download GGUF: {}", e),
                    }).ok();
                }
            }
            
            // Download tokenizer from original repo
            let tokenizer_repo = api.repo(Repo::with_revision(
                model_id.clone(),
                RepoType::Model,
                "main".to_string(),
            ));
            
            println!("[Inference] Downloading tokenizer...");
            match tokenizer_repo.get("tokenizer.json").await {
                Ok(path) => {
                    println!("[Inference] Downloaded tokenizer to {:?}", path);
                    downloaded_files.push(path.to_string_lossy().to_string());
                    
                    app_handle.emit("download-progress", DownloadProgress {
                        model_id: model_id.clone(),
                        status: "completed".to_string(),
                        progress: 100.0,
                        message: "Model and tokenizer downloaded successfully!".to_string(),
                    }).ok();
                }
                Err(e) => {
                    eprintln!("[Inference] Failed to download tokenizer: {}", e);
                    // For GGUF models, tokenizer might be embedded, so still mark as success
                    app_handle.emit("download-progress", DownloadProgress {
                        model_id: model_id.clone(),
                        status: "completed".to_string(),
                        progress: 100.0,
                        message: "Model downloaded successfully! (Tokenizer download failed but may be embedded in GGUF)".to_string(),
                    }).ok();
                }
            }
        }
        "microsoft/phi-2" => {
            // For Phi-2, use TheBloke's GGUF version
            let gguf_repo = api.repo(Repo::with_revision(
                "TheBloke/phi-2-GGUF".to_string(),
                RepoType::Model,
                "main".to_string(),
            ));
            
            println!("[Inference] Downloading GGUF model...");
            app_handle.emit("download-progress", DownloadProgress {
                model_id: model_id.clone(),
                status: "downloading".to_string(),
                progress: 10.0,
                message: "Downloading Phi-2 GGUF model file...".to_string(),
            }).ok();
            
            match gguf_repo.get("phi-2.Q4_K_M.gguf").await {
                Ok(path) => {
                    println!("[Inference] Downloaded GGUF to {:?}", path);
                    downloaded_files.push(path.to_string_lossy().to_string());
                    
                    app_handle.emit("download-progress", DownloadProgress {
                        model_id: model_id.clone(),
                        status: "downloading".to_string(),
                        progress: 80.0,
                        message: "Model downloaded, fetching tokenizer...".to_string(),
                    }).ok();
                }
                Err(e) => {
                    eprintln!("[Inference] Failed to download GGUF: {}", e);
                    app_handle.emit("download-progress", DownloadProgress {
                        model_id: model_id.clone(),
                        status: "error".to_string(),
                        progress: 0.0,
                        message: format!("Failed to download GGUF: {}", e),
                    }).ok();
                }
            }
            
            // Download tokenizer from original repo
            let tokenizer_repo = api.repo(Repo::with_revision(
                model_id.clone(),
                RepoType::Model,
                "main".to_string(),
            ));
            
            println!("[Inference] Downloading tokenizer...");
            match tokenizer_repo.get("tokenizer.json").await {
                Ok(path) => {
                    println!("[Inference] Downloaded tokenizer to {:?}", path);
                    downloaded_files.push(path.to_string_lossy().to_string());
                    
                    app_handle.emit("download-progress", DownloadProgress {
                        model_id: model_id.clone(),
                        status: "completed".to_string(),
                        progress: 100.0,
                        message: "Model and tokenizer downloaded successfully!".to_string(),
                    }).ok();
                }
                Err(e) => {
                    eprintln!("[Inference] Failed to download tokenizer: {}", e);
                }
            }
        }
        _ => {
            // For other models, try to get GGUF from the original repo
            let repo = api.repo(Repo::with_revision(
                model_id.clone(),
                RepoType::Model,
                "main".to_string(),
            ));
            
            println!("[Inference] Downloading GGUF model...");
            app_handle.emit("download-progress", DownloadProgress {
                model_id: model_id.clone(),
                status: "downloading".to_string(),
                progress: 10.0,
                message: "Downloading GGUF model file...".to_string(),
            }).ok();
            
            match repo.get("ggml-model-q4_0.gguf").await {
                Ok(path) => {
                    println!("[Inference] Downloaded GGUF to {:?}", path);
                    downloaded_files.push(path.to_string_lossy().to_string());
                    
                    app_handle.emit("download-progress", DownloadProgress {
                        model_id: model_id.clone(),
                        status: "downloading".to_string(),
                        progress: 80.0,
                        message: "Model downloaded, fetching tokenizer...".to_string(),
                    }).ok();
                }
                Err(e) => {
                    eprintln!("[Inference] Failed to download GGUF: {}", e);
                }
            }
            
            println!("[Inference] Downloading tokenizer...");
            match repo.get("tokenizer.json").await {
                Ok(path) => {
                    println!("[Inference] Downloaded tokenizer to {:?}", path);
                    downloaded_files.push(path.to_string_lossy().to_string());
                }
                Err(e) => {
                    eprintln!("[Inference] Failed to download tokenizer: {}", e);
                }
            }
        }
    };
    
    if downloaded_files.is_empty() {
        app_handle.emit("download-progress", DownloadProgress {
            model_id: model_id.clone(),
            status: "error".to_string(),
            progress: 0.0,
            message: "Failed to download model files".to_string(),
        }).ok();
        Err(format!("Failed to download any files for model {}", model_id))
    } else {
        app_handle.emit("download-progress", DownloadProgress {
            model_id: model_id.clone(),
            status: "completed".to_string(),
            progress: 100.0,
            message: format!("Successfully downloaded {} files", downloaded_files.len()),
        }).ok();
        Ok(format!("Downloaded {} files for {}", downloaded_files.len(), model_id))
    }
}