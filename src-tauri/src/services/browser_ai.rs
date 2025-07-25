use crate::browser_ai::BrowserAIAgent;
use crate::database::SqliteDatabase;
use crate::error::Result;
use crate::models::{ResearchTask, SavedResearchTask, BrowserAIProgressLight, BrowserAINewResult};
use chrono::Utc;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
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
    println!("Starting research for query: {query}");

    let (progress_tx, mut progress_rx) = mpsc::channel(100);
    let (result_tx, mut result_rx) = mpsc::channel(100);

    let mut agent = agent.lock().await;
    let task_id = agent.start_research(query, progress_tx, Some(result_tx)).await?;

    println!("Research task created with ID: {task_id}");

    let app_handle = app.clone();
    
    // Spawn task to forward lightweight progress events
    tauri::async_runtime::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            // Only log important status changes
            if matches!(progress.status, crate::models::TaskStatus::Searching | 
                                       crate::models::TaskStatus::Scraping | 
                                       crate::models::TaskStatus::Analyzing |
                                       crate::models::TaskStatus::Completed) {
                println!("Progress: {} - {}%", progress.current_operation.as_ref().unwrap_or(&"Working".to_string()), progress.percentage);
            }
            
            match app.emit("browser-ai-progress-light", &progress) {
                Ok(_) => {},
                Err(e) => println!("Failed to emit progress: {e:?}"),
            }
        }
    });
    
    // Spawn task to forward new result events
    tauri::async_runtime::spawn(async move {
        while let Some(result) = result_rx.recv().await {
            println!("New result found: {} from {}", result.result.title, result.subtask_query);
            match app_handle.emit("browser-ai-new-result", &result) {
                Ok(_) => {},
                Err(e) => println!("Failed to emit new result: {e:?}"),
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
    db: State<'_, Arc<Mutex<SqliteDatabase>>>,
    task_id: Uuid,
    tags: Vec<String>,
    notes: Option<String>,
) -> Result<SavedResearchTask> {
    println!("save_research called with task_id: {task_id}");

    let agent = agent.lock().await;
    let task = agent
        .get_task(&task_id)
        .ok_or_else(|| {
            eprintln!("Research task not found: {task_id}");
            crate::error::AppError::NotFound("Research task not found".into())
        })?
        .clone();

    println!("Found research task: {}", task.query);

    let saved_task = SavedResearchTask {
        id: Uuid::new_v4(),
        task,
        tags: tags.clone(),
        notes,
        saved_at: Utc::now(),
    };

    println!("Saving research task with tags: {tags:?}");

    // Save to database
    let db = db.lock().await;
    match db.save_research(&saved_task).await {
        Ok(_) => {
            println!("Research saved successfully with id: {}", saved_task.id);
            Ok(saved_task)
        }
        Err(e) => {
            eprintln!("Failed to save research: {e}");
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn get_saved_research(
    db: State<'_, Arc<Mutex<SqliteDatabase>>>,
    search_query: Option<String>,
) -> Result<Vec<SavedResearchTask>> {
    println!("get_saved_research called with query: {search_query:?}");

    let db = db.lock().await;

    let result = if let Some(_query) = search_query {
        // TODO: Implement search in SQLite
        db.get_all_research().await
    } else {
        db.get_all_research().await
    };

    match &result {
        Ok(tasks) => println!("Found {} saved research tasks", tasks.len()),
        Err(e) => eprintln!("Error getting saved research: {e}"),
    }

    result
}

#[tauri::command]
pub async fn delete_saved_research(
    db: State<'_, Arc<Mutex<SqliteDatabase>>>,
    id: Uuid,
) -> Result<()> {
    let db = db.lock().await;
    db.delete_research(&id).await
}
