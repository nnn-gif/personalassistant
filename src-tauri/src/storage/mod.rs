use crate::error::{Result, AppError};
use crate::models::{SavedResearchTask, Goal};
use std::fs;
use std::path::PathBuf;
use dirs::data_dir;

pub struct LocalStorage {
    data_dir: PathBuf,
    goals_dir: PathBuf,
}

impl LocalStorage {
    pub fn new() -> Result<Self> {
        let base_dir = data_dir()
            .ok_or_else(|| AppError::Storage("Could not find data directory".to_string()))?
            .join("personalassistant");
            
        let data_dir = base_dir.join("research");
        let goals_dir = base_dir.join("goals");
        
        // Create directories if they don't exist
        fs::create_dir_all(&data_dir)
            .map_err(|e| AppError::Storage(format!("Failed to create data directory: {}", e)))?;
        fs::create_dir_all(&goals_dir)
            .map_err(|e| AppError::Storage(format!("Failed to create goals directory: {}", e)))?;
        
        Ok(Self { data_dir, goals_dir })
    }
    
    pub fn save_research(&self, task: &SavedResearchTask) -> Result<()> {
        let file_path = self.data_dir.join(format!("{}.json", task.id));
        let json = serde_json::to_string_pretty(task)
            .map_err(|e| AppError::Storage(format!("Failed to serialize research: {}", e)))?;
        
        fs::write(&file_path, json)
            .map_err(|e| AppError::Storage(format!("Failed to write research file: {}", e)))?;
        
        Ok(())
    }
    
    pub fn get_saved_research(&self, search_query: Option<&str>) -> Result<Vec<SavedResearchTask>> {
        let mut results = Vec::new();
        
        let entries = fs::read_dir(&self.data_dir)
            .map_err(|e| AppError::Storage(format!("Failed to read data directory: {}", e)))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| AppError::Storage(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)
                    .map_err(|e| AppError::Storage(format!("Failed to read file: {}", e)))?;
                
                if let Ok(task) = serde_json::from_str::<SavedResearchTask>(&content) {
                    // Filter by search query if provided
                    if let Some(query) = search_query {
                        let query_lower = query.to_lowercase();
                        let matches = task.task.query.to_lowercase().contains(&query_lower)
                            || task.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
                            || task.notes.as_ref().map_or(false, |n| n.to_lowercase().contains(&query_lower));
                        
                        if matches {
                            results.push(task);
                        }
                    } else {
                        results.push(task);
                    }
                }
            }
        }
        
        // Sort by saved date (newest first)
        results.sort_by(|a, b| b.saved_at.cmp(&a.saved_at));
        
        Ok(results)
    }
    
    pub fn delete_research(&self, id: &uuid::Uuid) -> Result<()> {
        let file_path = self.data_dir.join(format!("{}.json", id));
        
        if file_path.exists() {
            fs::remove_file(&file_path)
                .map_err(|e| AppError::Storage(format!("Failed to delete research file: {}", e)))?;
        }
        
        Ok(())
    }
    
    // Goals storage methods
    pub fn save_goals(&self, goals: &Vec<Goal>) -> Result<()> {
        let file_path = self.goals_dir.join("goals.json");
        let json = serde_json::to_string_pretty(goals)
            .map_err(|e| AppError::Storage(format!("Failed to serialize goals: {}", e)))?;
        
        fs::write(&file_path, json)
            .map_err(|e| AppError::Storage(format!("Failed to write goals file: {}", e)))?;
        
        Ok(())
    }
    
    pub fn load_goals(&self) -> Result<Vec<Goal>> {
        let file_path = self.goals_dir.join("goals.json");
        
        if !file_path.exists() {
            return Ok(Vec::new());
        }
        
        let content = fs::read_to_string(&file_path)
            .map_err(|e| AppError::Storage(format!("Failed to read goals file: {}", e)))?;
        
        let goals = serde_json::from_str::<Vec<Goal>>(&content)
            .map_err(|e| AppError::Storage(format!("Failed to parse goals: {}", e)))?;
        
        Ok(goals)
    }
}