use crate::error::{AppError, Result};
use crate::models::{
    Activity, AppCategory, AppUsage, InputMetrics, ProjectContext, ProjectType, SystemState,
};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub async fn save_activity(pool: &SqlitePool, activity: &Activity) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO activities (id, timestamp, duration_seconds, app_name, window_title,
                              category, is_productive, keystrokes, mouse_clicks, 
                              mouse_distance_pixels, goal_id, project_name)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
    "#,
    )
    .bind(activity.id.to_string())
    .bind(activity.timestamp.to_rfc3339())
    .bind(activity.duration_seconds)
    .bind(&activity.app_usage.app_name)
    .bind(&activity.app_usage.window_title)
    .bind(format!("{:?}", activity.app_usage.category))
    .bind(activity.app_usage.is_productive)
    .bind(activity.input_metrics.keystrokes as i32)
    .bind(activity.input_metrics.mouse_clicks as i32)
    .bind(activity.input_metrics.mouse_distance_pixels)
    .bind(activity.goal_id.map(|id| id.to_string()))
    .bind(
        activity
            .project_context
            .as_ref()
            .map(|p| p.project_name.clone()),
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to save activity: {}", e)))?;

    Ok(())
}

pub async fn get_recent_activities(pool: &SqlitePool, limit: i32) -> Result<Vec<Activity>> {
    let rows = sqlx::query(
        "SELECT id, timestamp, duration_seconds, app_name, window_title, category, is_productive, keystrokes, mouse_clicks, mouse_distance_pixels, goal_id, project_name FROM activities ORDER BY timestamp DESC LIMIT ?"
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to fetch activities: {}", e)))?;

    let mut activities = Vec::new();
    for row in rows {
        activities.push(activity_from_row(&row)?);
    }

    Ok(activities)
}

pub async fn get_activities_by_date_range(
    pool: &SqlitePool,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<Activity>> {
    println!("[get_activities_by_date_range] Starting query for date range");
    println!("[get_activities_by_date_range] Start date: {}", start.to_rfc3339());
    println!("[get_activities_by_date_range] End date: {}", end.to_rfc3339());
    println!("[get_activities_by_date_range] Time range: {} hours", (end - start).num_hours());
    
    let query_start = std::time::Instant::now();
    
    let rows = sqlx::query(
        "SELECT id, timestamp, duration_seconds, app_name, window_title, category, is_productive, keystrokes, mouse_clicks, mouse_distance_pixels, goal_id, project_name FROM activities WHERE timestamp >= ? AND timestamp <= ? ORDER BY timestamp DESC"
    )
    .bind(start.to_rfc3339())
    .bind(end.to_rfc3339())
    .fetch_all(pool)
    .await
    .map_err(|e| {
        eprintln!("[get_activities_by_date_range] Database query failed: {}", e);
        AppError::Database(format!("Failed to fetch activities: {}", e))
    })?;

    let query_duration = query_start.elapsed();
    println!("[get_activities_by_date_range] Query executed in {:?}", query_duration);
    println!("[get_activities_by_date_range] Found {} rows", rows.len());

    let mut activities = Vec::new();
    let parse_start = std::time::Instant::now();
    
    for (index, row) in rows.iter().enumerate() {
        match activity_from_row(row) {
            Ok(activity) => {
                if index < 5 {
                    println!(
                        "[get_activities_by_date_range] Sample activity {}: {} - {} ({}s)",
                        index + 1,
                        activity.app_usage.app_name,
                        activity.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        activity.duration_seconds
                    );
                }
                activities.push(activity);
            }
            Err(e) => {
                eprintln!("[get_activities_by_date_range] Failed to parse row {}: {}", index, e);
                return Err(e);
            }
        }
    }

    let parse_duration = parse_start.elapsed();
    println!("[get_activities_by_date_range] Parsed {} activities in {:?}", activities.len(), parse_duration);
    
    // Summary statistics
    if !activities.is_empty() {
        let total_duration: u32 = activities.iter().map(|a| a.duration_seconds as u32).sum();
        let productive_count = activities.iter().filter(|a| a.app_usage.is_productive).count();
        println!("[get_activities_by_date_range] Total duration: {} seconds ({:.2} hours)", 
                 total_duration, total_duration as f64 / 3600.0);
        println!("[get_activities_by_date_range] Productive activities: {}/{} ({:.1}%)", 
                 productive_count, activities.len(), 
                 (productive_count as f64 / activities.len() as f64) * 100.0);
    }

    Ok(activities)
}

// Helper function to construct Activity from a database row
fn activity_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Activity> {
    let category = match row.get::<String, _>("category").as_str() {
        "Development" => AppCategory::Development,
        "Communication" => AppCategory::Communication,
        "SocialMedia" => AppCategory::SocialMedia,
        "Entertainment" => AppCategory::Entertainment,
        "Productivity" => AppCategory::Productivity,
        "System" => AppCategory::System,
        _ => AppCategory::Other,
    };

    Ok(Activity {
        id: Uuid::parse_str(&row.get::<String, _>("id"))
            .map_err(|e| AppError::Database(format!("Invalid UUID: {}", e)))?,
        timestamp: DateTime::parse_from_rfc3339(&row.get::<String, _>("timestamp"))
            .map_err(|e| AppError::Database(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&Utc),
        duration_seconds: row.get("duration_seconds"),
        app_usage: AppUsage {
            app_name: row.get("app_name"),
            bundle_id: String::new(), // Not stored, would need to add to schema
            window_title: row.get("window_title"),
            category,
            is_productive: row.get("is_productive"),
            browser_url: None,
            editor_file: None,
            terminal_info: None,
        },
        input_metrics: InputMetrics {
            keystrokes: row.get::<i32, _>("keystrokes") as u32,
            mouse_clicks: row.get::<i32, _>("mouse_clicks") as u32,
            mouse_distance_pixels: row.get("mouse_distance_pixels"),
            active_typing_seconds: 0, // Not stored, would need to add to schema
        },
        system_state: SystemState {
            idle_time_seconds: 0,
            is_screen_locked: false,
            battery_percentage: None,
            is_on_battery: false,
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0,
        },
        project_context: row
            .get::<Option<String>, _>("project_name")
            .map(|name| ProjectContext {
                project_name: name,
                project_path: String::new(),
                project_type: ProjectType::Other("Unknown".to_string()),
                git_branch: None,
            }),
        goal_id: row
            .get::<Option<String>, _>("goal_id")
            .and_then(|id| Uuid::parse_str(&id).ok()),
    })
}
