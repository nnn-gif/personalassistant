use crate::error::{AppError, Result};
use crate::models::{Activity, Goal, SavedResearchTask, ResearchTask};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions, Row};
use std::path::PathBuf;
use dirs::data_dir;
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub async fn new() -> Result<Self> {
        let db_path = Self::get_db_path()?;
        
        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::Database(format!("Failed to create database directory: {}", e)))?;
        }
        
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .map_err(|e| AppError::Database(format!("Failed to connect to database: {}", e)))?;
        
        let db = Self { pool };
        db.create_tables().await?;
        
        Ok(db)
    }
    
    fn get_db_path() -> Result<PathBuf> {
        let data_dir = data_dir()
            .ok_or_else(|| AppError::Database("Could not find data directory".to_string()))?;
        Ok(data_dir.join("personalassistant").join("database.db"))
    }
    
    async fn create_tables(&self) -> Result<()> {
        // Goals table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS goals (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                duration_minutes INTEGER,
                allowed_apps TEXT NOT NULL,
                progress_percentage REAL NOT NULL DEFAULT 0,
                time_spent_minutes INTEGER NOT NULL DEFAULT 0,
                is_active BOOLEAN NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create goals table: {}", e)))?;
        
        // Activities table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS activities (
                id TEXT PRIMARY KEY NOT NULL,
                timestamp TEXT NOT NULL,
                duration_seconds INTEGER NOT NULL,
                app_name TEXT NOT NULL,
                window_title TEXT NOT NULL,
                category TEXT NOT NULL,
                is_productive BOOLEAN NOT NULL,
                keystrokes INTEGER NOT NULL DEFAULT 0,
                mouse_clicks INTEGER NOT NULL DEFAULT 0,
                mouse_distance_pixels REAL NOT NULL DEFAULT 0,
                goal_id TEXT,
                project_name TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (goal_id) REFERENCES goals(id) ON DELETE SET NULL
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create activities table: {}", e)))?;
        
        // Create index for better query performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_activities_timestamp ON activities(timestamp)")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create activities index: {}", e)))?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_activities_goal_id ON activities(goal_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create goal_id index: {}", e)))?;
        
        // Research tasks table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS research_tasks (
                id TEXT PRIMARY KEY NOT NULL,
                task_id TEXT NOT NULL,
                task_data TEXT NOT NULL,
                tags TEXT NOT NULL,
                notes TEXT,
                saved_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create research_tasks table: {}", e)))?;
        
        // Audio recordings metadata table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS audio_recordings (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                file_path TEXT NOT NULL,
                duration_seconds REAL NOT NULL,
                file_size_bytes INTEGER NOT NULL,
                sample_rate INTEGER NOT NULL,
                channels INTEGER NOT NULL,
                transcription TEXT,
                started_at TEXT NOT NULL,
                ended_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create audio_recordings table: {}", e)))?;
        
        Ok(())
    }
    
    // Goals operations
    pub async fn save_goal(&self, goal: &Goal) -> Result<()> {
        let allowed_apps_json = serde_json::to_string(&goal.allowed_apps)
            .map_err(|e| AppError::Database(format!("Failed to serialize allowed_apps: {}", e)))?;
        
        sqlx::query(r#"
            INSERT INTO goals (id, name, duration_minutes, allowed_apps, progress_percentage, 
                             time_spent_minutes, is_active, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                duration_minutes = excluded.duration_minutes,
                allowed_apps = excluded.allowed_apps,
                progress_percentage = excluded.progress_percentage,
                time_spent_minutes = excluded.time_spent_minutes,
                is_active = excluded.is_active,
                updated_at = excluded.updated_at
        "#)
        .bind(goal.id.to_string())
        .bind(&goal.name)
        .bind(goal.target_duration_minutes as i32)
        .bind(allowed_apps_json)
        .bind(goal.progress_percentage())
        .bind(goal.current_duration_minutes as i32)
        .bind(goal.is_active)
        .bind(goal.created_at.to_rfc3339())
        .bind(goal.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to save goal: {}", e)))?;
        
        Ok(())
    }
    
    pub async fn get_all_goals(&self) -> Result<Vec<Goal>> {
        let rows = sqlx::query(
            "SELECT id, name, duration_minutes, allowed_apps, progress_percentage, time_spent_minutes, is_active, created_at, updated_at FROM goals ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
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
                target_duration_minutes: row.get::<Option<i32>, _>("duration_minutes").unwrap_or(0) as u32,
                allowed_apps,
                current_duration_minutes: row.get::<i32, _>("time_spent_minutes") as u32,
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
    
    pub async fn delete_goal(&self, goal_id: &Uuid) -> Result<()> {
        sqlx::query("DELETE FROM goals WHERE id = ?")
            .bind(goal_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete goal: {}", e)))?;
        
        Ok(())
    }
    
    // Activities operations
    pub async fn save_activity(&self, activity: &Activity) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO activities (id, timestamp, duration_seconds, app_name, window_title,
                                  category, is_productive, keystrokes, mouse_clicks, 
                                  mouse_distance_pixels, goal_id, project_name)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        "#)
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
        .bind(activity.project_context.as_ref().map(|p| p.project_name.clone()))
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to save activity: {}", e)))?;
        
        Ok(())
    }
    
    pub async fn get_recent_activities(&self, limit: i32) -> Result<Vec<Activity>> {
        let rows = sqlx::query(
            "SELECT id, timestamp, duration_seconds, app_name, window_title, category, is_productive, keystrokes, mouse_clicks, mouse_distance_pixels, goal_id, project_name FROM activities ORDER BY timestamp DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch activities: {}", e)))?;
        
        let mut activities = Vec::new();
        for row in rows {
            use crate::models::{AppUsage, AppCategory, InputMetrics, SystemState, ProjectContext, ProjectType};
            
            let category = match row.get::<String, _>("category").as_str() {
                "Development" => AppCategory::Development,
                "Communication" => AppCategory::Communication,
                "SocialMedia" => AppCategory::SocialMedia,
                "Entertainment" => AppCategory::Entertainment,
                "Productivity" => AppCategory::Productivity,
                "System" => AppCategory::System,
                _ => AppCategory::Other,
            };
            
            activities.push(Activity {
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
                project_context: row.get::<Option<String>, _>("project_name").map(|name| ProjectContext {
                    project_name: name,
                    project_path: String::new(),
                    project_type: ProjectType::Other("Unknown".to_string()),
                    git_branch: None,
                }),
                goal_id: row.get::<Option<String>, _>("goal_id").and_then(|id| Uuid::parse_str(&id).ok()),
            });
        }
        
        Ok(activities)
    }
    
    pub async fn get_activities_by_date_range(
        &self, 
        start: DateTime<Utc>, 
        end: DateTime<Utc>
    ) -> Result<Vec<Activity>> {
        let _rows = sqlx::query(
            "SELECT id, timestamp, duration_seconds, app_name, window_title, category, is_productive, keystrokes, mouse_clicks, mouse_distance_pixels, goal_id, project_name FROM activities WHERE timestamp >= ? AND timestamp <= ? ORDER BY timestamp DESC"
        )
        .bind(start.to_rfc3339())
        .bind(end.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch activities: {}", e)))?;
        
        // Convert rows to activities using same logic as get_recent_activities
        // For now returning empty, but could implement full conversion
        Ok(vec![])
    }
    
    // Research operations
    pub async fn save_research(&self, task: &SavedResearchTask) -> Result<()> {
        let task_data_json = serde_json::to_string(&task.task)
            .map_err(|e| AppError::Database(format!("Failed to serialize research task: {}", e)))?;
        let tags_json = serde_json::to_string(&task.tags)
            .map_err(|e| AppError::Database(format!("Failed to serialize tags: {}", e)))?;
        
        sqlx::query(r#"
            INSERT INTO research_tasks (id, task_id, task_data, tags, notes, saved_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                task_id = excluded.task_id,
                task_data = excluded.task_data,
                tags = excluded.tags,
                notes = excluded.notes,
                saved_at = excluded.saved_at
        "#)
        .bind(task.id.to_string())
        .bind(task.task.id.to_string())
        .bind(task_data_json)
        .bind(tags_json)
        .bind(&task.notes)
        .bind(task.saved_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to save research: {}", e)))?;
        
        Ok(())
    }
    
    pub async fn get_all_research(&self) -> Result<Vec<SavedResearchTask>> {
        let rows = sqlx::query(
            "SELECT id, task_id, task_data, tags, notes, saved_at FROM research_tasks ORDER BY saved_at DESC"
        )
        .fetch_all(&self.pool)
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
    
    pub async fn delete_research(&self, id: &Uuid) -> Result<()> {
        sqlx::query("DELETE FROM research_tasks WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete research: {}", e)))?;
        
        Ok(())
    }
    
    // Migration helper to import existing JSON data
    pub async fn import_from_storage(&self, storage: &crate::storage::LocalStorage) -> Result<()> {
        // Import goals
        let goals = storage.load_goals()?;
        for goal in goals {
            self.save_goal(&goal).await?;
        }
        
        // Import research tasks
        let research_tasks = storage.get_saved_research(None)?;
        for task in research_tasks {
            self.save_research(&task).await?;
        }
        
        println!("Successfully imported data from file storage to SQLite");
        Ok(())
    }
}