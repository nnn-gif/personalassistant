use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub services: ServicesConfig,
    pub tracking: TrackingConfig,
    pub rag: RagConfig,
    pub audio: AudioConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    pub ollama_url: String,
    pub qdrant_url: String,
    pub ollama_model: String,
    pub ollama_embedding_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingConfig {
    pub enabled: bool,
    pub tracking_interval_ms: u64,
    pub idle_threshold_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub use_qdrant: bool,
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub enable_transcription: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub db_name: String,
    pub enable_migrations: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            services: ServicesConfig {
                ollama_url: std::env::var("OLLAMA_URL")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string()),
                qdrant_url: std::env::var("QDRANT_URL")
                    .unwrap_or_else(|_| "http://localhost:6333".to_string()),
                ollama_model: std::env::var("OLLAMA_MODEL")
                    .unwrap_or_else(|_| "llama3.2:1b".to_string()),
                ollama_embedding_model: std::env::var("OLLAMA_EMBEDDING_MODEL")
                    .unwrap_or_else(|_| "nomic-embed-text:latest".to_string()),
            },
            tracking: TrackingConfig {
                enabled: std::env::var("TRACKING_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                tracking_interval_ms: std::env::var("TRACKING_INTERVAL_MS")
                    .unwrap_or_else(|_| "5000".to_string())
                    .parse()
                    .unwrap_or(5000),
                idle_threshold_ms: std::env::var("IDLE_THRESHOLD_MS")
                    .unwrap_or_else(|_| "300000".to_string())
                    .parse()
                    .unwrap_or(300000),
            },
            rag: RagConfig {
                chunk_size: std::env::var("RAG_CHUNK_SIZE")
                    .unwrap_or_else(|_| "1000".to_string())
                    .parse()
                    .unwrap_or(1000),
                chunk_overlap: std::env::var("RAG_CHUNK_OVERLAP")
                    .unwrap_or_else(|_| "200".to_string())
                    .parse()
                    .unwrap_or(200),
                use_qdrant: std::env::var("USE_QDRANT")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                collection_name: std::env::var("QDRANT_COLLECTION_NAME")
                    .unwrap_or_else(|_| "documents".to_string()),
            },
            audio: AudioConfig {
                sample_rate: std::env::var("AUDIO_SAMPLE_RATE")
                    .unwrap_or_else(|_| "44100".to_string())
                    .parse()
                    .unwrap_or(44100),
                channels: std::env::var("AUDIO_CHANNELS")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()
                    .unwrap_or(1),
                enable_transcription: std::env::var("ENABLE_TRANSCRIPTION")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            },
            database: DatabaseConfig {
                db_name: std::env::var("DATABASE_NAME")
                    .unwrap_or_else(|_| "personal_assistant.db".to_string()),
                enable_migrations: std::env::var("ENABLE_DB_MIGRATIONS")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            },
        }
    }
}

impl Config {
    pub fn load(app_handle: &AppHandle) -> Result<(), String> {
        let mut config = Config::default();
        
        // Try to load from config file
        if let Some(config_dir) = app_handle.path().app_config_dir().ok() {
            let config_path = config_dir.join("config.toml");
            if config_path.exists() {
                match std::fs::read_to_string(&config_path) {
                    Ok(contents) => {
                        match toml::from_str::<Config>(&contents) {
                            Ok(file_config) => {
                                // Merge file config with env config (env takes precedence)
                                config = Self::merge_configs(file_config, config);
                            }
                            Err(e) => {
                                eprintln!("Failed to parse config.toml: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read config.toml: {}", e);
                    }
                }
            }
        }
        
        CONFIG.set(config).map_err(|_| "Config already initialized".to_string())?;
        Ok(())
    }
    
    pub fn get() -> &'static Config {
        CONFIG.get().expect("Config not initialized")
    }
    
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // Validate URLs
        if !self.services.ollama_url.starts_with("http://") && !self.services.ollama_url.starts_with("https://") {
            errors.push("Invalid Ollama URL format".to_string());
        }
        
        if !self.services.qdrant_url.starts_with("http://") && !self.services.qdrant_url.starts_with("https://") {
            errors.push("Invalid Qdrant URL format".to_string());
        }
        
        // Validate numeric values
        if self.tracking.tracking_interval_ms == 0 {
            errors.push("Tracking interval must be greater than 0".to_string());
        }
        
        if self.rag.chunk_size == 0 {
            errors.push("RAG chunk size must be greater than 0".to_string());
        }
        
        if self.rag.chunk_overlap >= self.rag.chunk_size {
            errors.push("RAG chunk overlap must be less than chunk size".to_string());
        }
        
        if self.audio.sample_rate == 0 {
            errors.push("Audio sample rate must be greater than 0".to_string());
        }
        
        if self.audio.channels == 0 {
            errors.push("Audio channels must be greater than 0".to_string());
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    fn merge_configs(file_config: Config, env_config: Config) -> Config {
        // Environment variables take precedence over file config
        Config {
            services: ServicesConfig {
                ollama_url: if std::env::var("OLLAMA_URL").is_ok() {
                    env_config.services.ollama_url
                } else {
                    file_config.services.ollama_url
                },
                qdrant_url: if std::env::var("QDRANT_URL").is_ok() {
                    env_config.services.qdrant_url
                } else {
                    file_config.services.qdrant_url
                },
                ollama_model: if std::env::var("OLLAMA_MODEL").is_ok() {
                    env_config.services.ollama_model
                } else {
                    file_config.services.ollama_model
                },
                ollama_embedding_model: if std::env::var("OLLAMA_EMBEDDING_MODEL").is_ok() {
                    env_config.services.ollama_embedding_model
                } else {
                    file_config.services.ollama_embedding_model
                },
            },
            tracking: TrackingConfig {
                enabled: if std::env::var("TRACKING_ENABLED").is_ok() {
                    env_config.tracking.enabled
                } else {
                    file_config.tracking.enabled
                },
                tracking_interval_ms: if std::env::var("TRACKING_INTERVAL_MS").is_ok() {
                    env_config.tracking.tracking_interval_ms
                } else {
                    file_config.tracking.tracking_interval_ms
                },
                idle_threshold_ms: if std::env::var("IDLE_THRESHOLD_MS").is_ok() {
                    env_config.tracking.idle_threshold_ms
                } else {
                    file_config.tracking.idle_threshold_ms
                },
            },
            rag: RagConfig {
                chunk_size: if std::env::var("RAG_CHUNK_SIZE").is_ok() {
                    env_config.rag.chunk_size
                } else {
                    file_config.rag.chunk_size
                },
                chunk_overlap: if std::env::var("RAG_CHUNK_OVERLAP").is_ok() {
                    env_config.rag.chunk_overlap
                } else {
                    file_config.rag.chunk_overlap
                },
                use_qdrant: if std::env::var("USE_QDRANT").is_ok() {
                    env_config.rag.use_qdrant
                } else {
                    file_config.rag.use_qdrant
                },
                collection_name: if std::env::var("QDRANT_COLLECTION_NAME").is_ok() {
                    env_config.rag.collection_name
                } else {
                    file_config.rag.collection_name
                },
            },
            audio: AudioConfig {
                sample_rate: if std::env::var("AUDIO_SAMPLE_RATE").is_ok() {
                    env_config.audio.sample_rate
                } else {
                    file_config.audio.sample_rate
                },
                channels: if std::env::var("AUDIO_CHANNELS").is_ok() {
                    env_config.audio.channels
                } else {
                    file_config.audio.channels
                },
                enable_transcription: if std::env::var("ENABLE_TRANSCRIPTION").is_ok() {
                    env_config.audio.enable_transcription
                } else {
                    file_config.audio.enable_transcription
                },
            },
            database: DatabaseConfig {
                db_name: if std::env::var("DATABASE_NAME").is_ok() {
                    env_config.database.db_name
                } else {
                    file_config.database.db_name
                },
                enable_migrations: if std::env::var("ENABLE_DB_MIGRATIONS").is_ok() {
                    env_config.database.enable_migrations
                } else {
                    file_config.database.enable_migrations
                },
            },
        }
    }
    
    pub fn save_to_file(&self, app_handle: &AppHandle) -> Result<(), String> {
        let config_dir = app_handle.path()
            .app_config_dir()
            .map_err(|e| format!("Failed to get config dir: {}", e))?;
        
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config dir: {}", e))?;
        
        let config_path = config_dir.join("config.toml");
        let toml_string = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        std::fs::write(config_path, toml_string)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.services.ollama_url, "http://localhost:11434");
        assert_eq!(config.services.qdrant_url, "http://localhost:6333");
        assert_eq!(config.tracking.tracking_interval_ms, 5000);
        assert_eq!(config.rag.chunk_size, 1000);
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());
        
        config.services.ollama_url = "invalid-url".to_string();
        assert!(config.validate().is_err());
        
        config = Config::default();
        config.rag.chunk_overlap = 2000;
        assert!(config.validate().is_err());
    }
}