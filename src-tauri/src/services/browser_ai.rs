use crate::browser_ai::BrowserAIAgent;
use crate::error::Result;
use crate::models::{BrowserAIProgress, ResearchTask, SavedResearchTask};
use chrono::Utc;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

#[tauri::command]
pub async fn test_research() -> Result<String> {
    println!("Test research command called!");
    Ok("Research system is working".to_string())
}

#[tauri::command]
pub async fn start_research(
    app: AppHandle,
    agent: State<'_, Arc<Mutex<BrowserAIAgent>>>,
    query: String,
) -> Result<Uuid> {
    println!("Starting research for query: {}", query);
    
    let (tx, mut rx) = mpsc::channel(100);
    
    let mut agent = agent.lock().await;
    let task_id = agent.start_research(query, tx).await?;
    
    println!("Research task created with ID: {}", task_id);
    
    // Spawn task to forward progress events to frontend
    tauri::async_runtime::spawn(async move {
        while let Some(progress) = rx.recv().await {
            println!("Emitting progress: {:?}", progress.status);
            match app.emit("browser-ai-progress", &progress) {
                Ok(_) => println!("Progress event emitted successfully"),
                Err(e) => println!("Failed to emit progress: {:?}", e),
            }
        }
    });
    
    Ok(task_id)
}

#[tauri::command]
pub async fn get_research_status(
    agent: State<'_, Arc<Mutex<BrowserAIAgent>>>,
    task_id: Uuid,
) -> Result<Option<ResearchTask>> {
    let agent = agent.lock().await;
    Ok(agent.get_task(&task_id).cloned())
}

#[tauri::command]
pub async fn save_research(
    agent: State<'_, Arc<Mutex<BrowserAIAgent>>>,
    task_id: Uuid,
    tags: Vec<String>,
    notes: Option<String>,
) -> Result<SavedResearchTask> {
    let agent = agent.lock().await;
    let task = agent.get_task(&task_id)
        .ok_or_else(|| crate::error::AppError::NotFound("Research task not found".into()))?
        .clone();
    
    let saved_task = SavedResearchTask {
        id: Uuid::new_v4(),
        task,
        tags,
        notes,
        saved_at: Utc::now(),
    };
    
    // Return the saved task without database for now
    Ok(saved_task)
}

#[tauri::command]
pub async fn get_saved_research(
    search_query: Option<String>,
) -> Result<Vec<SavedResearchTask>> {
    // Return empty for now until database is initialized
    Ok(vec![])
}