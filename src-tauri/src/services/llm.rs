use crate::error::Result;
use crate::llm::LlmClient;
use crate::models::{ProductivityInsights, ProductivityScore};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_productivity_insights(
    llm: State<'_, Arc<LlmClient>>,
    hours: usize,
) -> Result<ProductivityInsights> {
    // For now, return mock data until database is ready
    let activities = vec![];
    llm.generate_productivity_insights(&activities).await
}

#[tauri::command]
pub async fn get_productivity_score(
    llm: State<'_, Arc<LlmClient>>,
    hours: usize,
) -> Result<ProductivityScore> {
    // For now, return mock data until database is ready
    let activities = vec![];
    llm.generate_productivity_score(&activities).await
}

#[tauri::command]
pub async fn get_recommendations(
    llm: State<'_, Arc<LlmClient>>,
    hours: usize,
) -> Result<Vec<String>> {
    // For now, return mock data until database is ready
    let activities = vec![];
    llm.generate_recommendations(&activities).await
}