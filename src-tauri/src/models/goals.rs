use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: Uuid,
    pub name: String,
    pub target_duration_minutes: u32,
    pub allowed_apps: Vec<String>,
    pub current_duration_minutes: u32,
    #[serde(default)]
    pub current_duration_seconds: u32,
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
    pub fn new(name: String, target_duration_minutes: u32, allowed_apps: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            target_duration_minutes,
            allowed_apps,
            current_duration_minutes: 0,
            current_duration_seconds: 0,
            is_active: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_progress(&mut self, additional_minutes: u32) {
        self.current_duration_minutes += additional_minutes;
        self.updated_at = Utc::now();
    }

    pub fn update_progress_seconds(&mut self, additional_seconds: u32) {
        // Accumulate seconds and convert to minutes when we reach 60 seconds
        self.current_duration_seconds += additional_seconds;
        if self.current_duration_seconds >= 60 {
            let additional_minutes = self.current_duration_seconds / 60;
            self.current_duration_minutes += additional_minutes;
            self.current_duration_seconds %= 60;
        }
        self.updated_at = Utc::now();
    }

    pub fn progress_percentage(&self) -> f32 {
        if self.target_duration_minutes > 0 {
            ((self.current_duration_minutes as f32 / self.target_duration_minutes as f32) * 100.0)
                .min(100.0)
        } else {
            0.0
        }
    }

    pub fn is_app_allowed(&self, app_name: &str) -> bool {
        self.allowed_apps
            .iter()
            .any(|allowed| allowed.to_lowercase() == app_name.to_lowercase())
    }

    pub fn is_completed(&self) -> bool {
        self.progress_percentage() >= 100.0
    }
}
