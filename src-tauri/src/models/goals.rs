use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: Uuid,
    pub name: String,
    pub duration_minutes: u32,
    pub allowed_apps: Vec<String>,
    pub progress_percentage: f32,
    pub time_spent_minutes: u32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSession {
    pub id: Uuid,
    pub goal_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_minutes: u32,
}

impl Goal {
    pub fn new(name: String, duration_minutes: u32, allowed_apps: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            duration_minutes,
            allowed_apps,
            progress_percentage: 0.0,
            time_spent_minutes: 0,
            is_active: false,
            created_at: now,
            updated_at: now,
        }
    }
    
    pub fn update_progress(&mut self, additional_minutes: u32) {
        self.time_spent_minutes += additional_minutes;
        self.progress_percentage = 
            ((self.time_spent_minutes as f32 / self.duration_minutes as f32) * 100.0).min(100.0);
        self.updated_at = Utc::now();
    }
    
    pub fn is_app_allowed(&self, app_name: &str) -> bool {
        self.allowed_apps.iter()
            .any(|allowed| allowed.to_lowercase() == app_name.to_lowercase())
    }
    
    pub fn is_completed(&self) -> bool {
        self.progress_percentage >= 100.0
    }
}