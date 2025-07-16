use crate::error::{AppError, Result};
use crate::models::Goal;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub async fn save_goal(pool: &SqlitePool, goal: &Goal) -> Result<()> {
    let allowed_apps_json = serde_json::to_string(&goal.allowed_apps)
        .map_err(|e| AppError::Database(format!("Failed to serialize allowed_apps: {}", e)))?;

    sqlx::query(
        r#"
        INSERT INTO goals (id, name, duration_minutes, allowed_apps, progress_percentage, 
                         time_spent_minutes, time_spent_seconds, is_active, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            duration_minutes = excluded.duration_minutes,
            allowed_apps = excluded.allowed_apps,
            progress_percentage = excluded.progress_percentage,
            time_spent_minutes = excluded.time_spent_minutes,
            time_spent_seconds = excluded.time_spent_seconds,
            is_active = excluded.is_active,
            updated_at = excluded.updated_at
    "#,
    )
    .bind(goal.id.to_string())
    .bind(&goal.name)
    .bind(goal.target_duration_minutes as i32)
    .bind(allowed_apps_json)
    .bind(goal.progress_percentage())
    .bind(goal.current_duration_minutes as i32)
    .bind(goal.current_duration_seconds as i32)
    .bind(goal.is_active)
    .bind(goal.created_at.to_rfc3339())
    .bind(goal.updated_at.to_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to save goal: {}", e)))?;

    Ok(())
}

pub async fn get_all_goals(pool: &SqlitePool) -> Result<Vec<Goal>> {
    let rows = sqlx::query(
        "SELECT id, name, duration_minutes, allowed_apps, progress_percentage, time_spent_minutes, time_spent_seconds, is_active, created_at, updated_at FROM goals ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to fetch goals: {}", e)))?;

    let mut goals = Vec::new();
    for row in rows {
        let allowed_apps: Vec<String> = serde_json::from_str(&row.get::<String, _>("allowed_apps"))
            .map_err(|e| AppError::Database(format!("Failed to parse allowed_apps: {}", e)))?;

        goals.push(Goal {
            id: Uuid::parse_str(&row.get::<String, _>("id"))
                .map_err(|e| AppError::Database(format!("Invalid UUID: {}", e)))?,
            name: row.get("name"),
            target_duration_minutes: row.get::<Option<i32>, _>("duration_minutes").unwrap_or(0)
                as u32,
            allowed_apps,
            current_duration_minutes: row.get::<i32, _>("time_spent_minutes") as u32,
            current_duration_seconds: row.get::<Option<i32>, _>("time_spent_seconds").unwrap_or(0)
                as u32,
            is_active: row.get("is_active"),
            created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))
                .map_err(|e| AppError::Database(format!("Invalid created_at date: {}", e)))?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))
                .map_err(|e| AppError::Database(format!("Invalid updated_at date: {}", e)))?
                .with_timezone(&Utc),
        });
    }

    Ok(goals)
}

pub async fn delete_goal(pool: &SqlitePool, goal_id: &Uuid) -> Result<()> {
    sqlx::query("DELETE FROM goals WHERE id = ?")
        .bind(goal_id.to_string())
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to delete goal: {}", e)))?;

    Ok(())
}
