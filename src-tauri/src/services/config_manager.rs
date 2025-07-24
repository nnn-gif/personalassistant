use crate::config::{Config, InferenceProvider};
use crate::error::Result;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Updates the inference provider configuration and saves it to disk
pub async fn update_inference_config(
    app_handle: &AppHandle,
    provider: InferenceProvider,
    model_id: Option<String>,
) -> Result<()> {
    // Get the config directory
    let config_dir = app_handle.path()
        .app_config_dir()
        .map_err(|e| crate::error::AppError::Config(format!("Failed to get config dir: {}", e)))?;
    
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| crate::error::AppError::Config(format!("Failed to create config dir: {}", e)))?;
    
    let config_path = config_dir.join("config.toml");
    
    // Read existing config or use default
    let mut config = if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)
            .map_err(|e| crate::error::AppError::Config(format!("Failed to read config: {}", e)))?;
        toml::from_str::<Config>(&contents)
            .unwrap_or_else(|_| Config::default())
    } else {
        Config::default()
    };
    
    // Update the configuration
    config.services.inference_provider = provider;
    
    match provider {
        InferenceProvider::Candle => {
            if let Some(id) = model_id {
                config.services.candle_model_id = id;
            }
        }
        InferenceProvider::Crane => {
            // Crane uses the same model config as Candle
            if let Some(id) = model_id {
                config.services.candle_model_id = id;
            }
        }
        InferenceProvider::Ollama => {
            if let Some(id) = model_id {
                config.services.ollama_model = id;
            }
        }
        InferenceProvider::Callm => {
            // Callm uses the same model config as Candle
            if let Some(id) = model_id {
                config.services.candle_model_id = id;
            }
        }
        InferenceProvider::LlamaCpp => {
            // LlamaCpp uses the same model config as Candle
            if let Some(id) = model_id {
                config.services.candle_model_id = id;
            }
        }
    }
    
    // Save the configuration
    let toml_string = toml::to_string_pretty(&config)
        .map_err(|e| crate::error::AppError::Config(format!("Failed to serialize config: {}", e)))?;
    
    std::fs::write(&config_path, toml_string)
        .map_err(|e| crate::error::AppError::Config(format!("Failed to write config: {}", e)))?;
    
    println!("[ConfigManager] Saved config to: {}", config_path.display());
    
    Ok(())
}

/// Gets the path to the config file
pub fn get_config_path(app_handle: &AppHandle) -> Result<PathBuf> {
    let config_dir = app_handle.path()
        .app_config_dir()
        .map_err(|e| crate::error::AppError::Config(format!("Failed to get config dir: {}", e)))?;
    
    Ok(config_dir.join("config.toml"))
}