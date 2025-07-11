use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub duration_seconds: i64,
    pub app_usage: AppUsage,
    pub input_metrics: InputMetrics,
    pub system_state: SystemState,
    pub project_context: Option<ProjectContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUsage {
    pub app_name: String,
    pub bundle_id: String,
    pub window_title: String,
    pub category: AppCategory,
    pub is_productive: bool,
    pub browser_url: Option<String>,
    pub editor_file: Option<String>,
    pub terminal_info: Option<TerminalInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppCategory {
    Development,
    Communication,
    SocialMedia,
    Entertainment,
    Productivity,
    System,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputMetrics {
    pub keystrokes: u32,
    pub mouse_clicks: u32,
    pub mouse_distance_pixels: f64,
    pub active_typing_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub idle_time_seconds: u32,
    pub is_screen_locked: bool,
    pub battery_percentage: Option<f32>,
    pub is_on_battery: bool,
    pub cpu_usage_percent: f32,
    pub memory_usage_mb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project_name: String,
    pub project_path: String,
    pub project_type: ProjectType,
    pub git_branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectType {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Go,
    Java,
    CSharp,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalInfo {
    pub current_directory: String,
    pub last_command: Option<String>,
}