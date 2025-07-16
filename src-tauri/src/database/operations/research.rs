use crate::error::{AppError, Result};
use crate::models::{ResearchTask, SavedResearchTask};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub async fn save_research(pool: &SqlitePool, task: &SavedResearchTask) -> Result<()> {
    let task_data_json = serde_json::to_string(&task.task)
        .map_err(|e| AppError::Database(format!("Failed to serialize research task: {}", e)))?;
    let tags_json = serde_json::to_string(&task.tags)
        .map_err(|e| AppError::Database(format!("Failed to serialize tags: {}", e)))?;

    sqlx::query(
        r#"
        INSERT INTO research_tasks (id, task_id, task_data, tags, notes, saved_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(id) DO UPDATE SET
            task_id = excluded.task_id,
            task_data = excluded.task_data,
            tags = excluded.tags,
            notes = excluded.notes,
            saved_at = excluded.saved_at
    "#,
    )
    .bind(task.id.to_string())
    .bind(task.task.id.to_string())
    .bind(task_data_json)
    .bind(tags_json)
    .bind(&task.notes)
    .bind(task.saved_at.to_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to save research: {}", e)))?;

    Ok(())
}

pub async fn get_all_research(pool: &SqlitePool) -> Result<Vec<SavedResearchTask>> {
    let rows = sqlx::query(
        "SELECT id, task_id, task_data, tags, notes, saved_at FROM research_tasks ORDER BY saved_at DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to fetch research tasks: {}", e)))?;

    let mut tasks = Vec::new();
    for row in rows {
        let task: ResearchTask = serde_json::from_str(&row.get::<String, _>("task_data"))
            .map_err(|e| AppError::Database(format!("Failed to parse research task: {}", e)))?;
        let tags: Vec<String> = serde_json::from_str(&row.get::<String, _>("tags"))
            .map_err(|e| AppError::Database(format!("Failed to parse tags: {}", e)))?;

        tasks.push(SavedResearchTask {
            id: Uuid::parse_str(&row.get::<String, _>("id"))
                .map_err(|e| AppError::Database(format!("Invalid UUID: {}", e)))?,
            task,
            tags,
            notes: row.get("notes"),
            saved_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("saved_at"))
                .map_err(|e| AppError::Database(format!("Invalid saved_at date: {}", e)))?
                .with_timezone(&Utc),
        });
    }

    Ok(tasks)
}

pub async fn delete_research(pool: &SqlitePool, id: &Uuid) -> Result<()> {
    sqlx::query("DELETE FROM research_tasks WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to delete research: {}", e)))?;

    Ok(())
}