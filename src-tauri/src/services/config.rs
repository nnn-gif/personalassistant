use crate::config::Config;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub theme: String,
    pub language: String,
    pub notifications_enabled: bool,
    pub auto_start_tracking: bool,
    pub window_opacity: f32,
    pub default_view: String,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            language: "en".to_string(),
            notifications_enabled: true,
            auto_start_tracking: false,
            window_opacity: 1.0,
            default_view: "dashboard".to_string(),
        }
    }
}

#[tauri::command]
pub async fn get_config() -> Result<Config> {
    Ok(Config::get().clone())
}

#[tauri::command]
pub async fn update_config(
    app_handle: AppHandle,
    config: Config,
) -> Result<()> {
    // Validate the new configuration
    config.validate().map_err(|errors| {
        crate::error::AppError::Config(format!("Validation errors: {:?}", errors))
    })?;
    
    // Save to file
    config.save_to_file(&app_handle)
        .map_err(|e| crate::error::AppError::Config(e))?;
    
    // Note: In a real application, you'd want to reload the configuration
    // This would require making the CONFIG static mutable or using a different approach
    
    Ok(())
}

#[tauri::command]
pub async fn get_user_preferences(app_handle: AppHandle) -> Result<UserPreferences> {
    let prefs_dir = app_handle.path()
        .app_config_dir()
        .map_err(|e| crate::error::AppError::Config(format!("Failed to get config dir: {}", e)))?;
    
    let prefs_path = prefs_dir.join("preferences.json");
    
    if prefs_path.exists() {
        let contents = std::fs::read_to_string(&prefs_path)
            .map_err(|e| crate::error::AppError::Config(format!("Failed to read preferences: {}", e)))?;
        
        let prefs: UserPreferences = serde_json::from_str(&contents)
            .map_err(|e| crate::error::AppError::Config(format!("Failed to parse preferences: {}", e)))?;
        
        Ok(prefs)
    } else {
        Ok(UserPreferences::default())
    }
}

#[tauri::command]
pub async fn update_user_preferences(
    app_handle: AppHandle,
    preferences: UserPreferences,
) -> Result<()> {
    let prefs_dir = app_handle.path()
        .app_config_dir()
        .map_err(|e| crate::error::AppError::Config(format!("Failed to get config dir: {}", e)))?;
    
    // Ensure directory exists
    std::fs::create_dir_all(&prefs_dir)
        .map_err(|e| crate::error::AppError::Config(format!("Failed to create config dir: {}", e)))?;
    
    let prefs_path = prefs_dir.join("preferences.json");
    let json = serde_json::to_string_pretty(&preferences)
        .map_err(|e| crate::error::AppError::Config(format!("Failed to serialize preferences: {}", e)))?;
    
    std::fs::write(prefs_path, json)
        .map_err(|e| crate::error::AppError::Config(format!("Failed to write preferences: {}", e)))?;
    
    Ok(())
}

#[tauri::command]
pub async fn reset_preferences(app_handle: AppHandle) -> Result<UserPreferences> {
    let default_prefs = UserPreferences::default();
    update_user_preferences(app_handle, default_prefs.clone()).await?;
    Ok(default_prefs)
}