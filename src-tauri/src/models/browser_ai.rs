use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchTask {
    pub id: Uuid,
    pub query: String,
    pub status: TaskStatus,
    pub subtasks: Vec<ResearchSubtask>,
    pub results: Vec<ResearchResult>,
    pub conclusion: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    SplittingTasks,
    Searching,
    Scraping,
    Analyzing,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSubtask {
    pub id: Uuid,
    pub query: String,
    pub status: TaskStatus,
    pub search_results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchResult {
    pub id: Uuid,
    pub subtask_id: Uuid,
    pub url: String,
    pub title: String,
    pub content: String,
    pub relevance_score: f32,
    pub scraped_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAIProgress {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub current_subtask: Option<String>,
    pub completed_subtasks: usize,
    pub total_subtasks: usize,
    pub percentage: f32,
    pub current_operation: Option<String>,
    pub subtasks_progress: Vec<SubtaskProgress>,
    pub intermediate_results: Vec<ResearchResult>,
    pub phase_details: Option<PhaseDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskProgress {
    pub id: Uuid,
    pub query: String,
    pub status: TaskStatus,
    pub current_operation: Option<String>,
    pub search_results_count: usize,
    pub scraped_pages_count: usize,
    pub results: Vec<ResearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseDetails {
    pub phase: String,
    pub details: String,
    pub estimated_completion: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedResearchTask {
    pub id: Uuid,
    pub task: ResearchTask,
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub saved_at: DateTime<Utc>,
}
