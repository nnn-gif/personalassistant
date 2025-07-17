use crate::config::{Config, InferenceProvider};
use crate::llm::LlmClient;
use std::sync::Arc;
use tauri::{AppHandle, State};

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
        available_providers: vec!["Ollama".to_string(), "Candle".to_string()],
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
        "ollama" => InferenceProvider::Ollama,
        _ => return Err(format!("Unknown provider: {}", provider)),
    };
    
    // Update the configuration using the config manager
    crate::services::config_manager::update_inference_config(
        &app_handle,
        inference_provider,
        model_id.clone(),
    )
    .await
    .map_err(|e| format!("Failed to update config: {}", e))?;
    
    println!("[Inference] Configuration saved successfully");
    
    // Also update environment variables for current session
    match provider.to_lowercase().as_str() {
        "candle" => {
            std::env::set_var("INFERENCE_PROVIDER", "candle");
            if let Some(id) = model_id {
                std::env::set_var("CANDLE_MODEL_ID", id);
            }
        }
        "ollama" => {
            std::env::set_var("INFERENCE_PROVIDER", "ollama");
            if let Some(id) = model_id {
                std::env::set_var("OLLAMA_MODEL", id);
            }
        }
        _ => {}
    }
    
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
    // Return a list of popular models that work well with Candle
    Ok(vec![
        CandleModel {
            id: "microsoft/phi-2".to_string(),
            name: "Phi-2 (2.7B)".to_string(),
            description: "Small but capable model from Microsoft".to_string(),
            size: "5.5 GB".to_string(),
        },
        CandleModel {
            id: "TinyLlama/TinyLlama-1.1B-Chat-v1.0".to_string(),
            name: "TinyLlama 1.1B".to_string(),
            description: "Tiny but efficient chat model".to_string(),
            size: "2.2 GB".to_string(),
        },
        CandleModel {
            id: "mistralai/Mistral-7B-v0.1".to_string(),
            name: "Mistral 7B".to_string(),
            description: "High-quality 7B parameter model".to_string(),
            size: "14 GB".to_string(),
        },
    ])
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CandleModel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size: String,
}

#[tauri::command]
pub async fn get_config_path(
    app_handle: AppHandle,
) -> std::result::Result<String, String> {
    crate::services::config_manager::get_config_path(&app_handle)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to get config path: {}", e))
}