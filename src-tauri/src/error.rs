use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("LLM error: {0}")]
    Llm(String),
    
    #[error("Browser AI error: {0}")]
    BrowserAI(String),
    
    #[error("Activity tracking error: {0}")]
    ActivityTracking(String),
    
    #[error("Goal error: {0}")]
    Goal(String),
    
    #[error("Platform error: {0}")]
    Platform(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Audio error: {0}")]
    Audio(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;