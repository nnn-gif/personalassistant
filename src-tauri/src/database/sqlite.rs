use crate::error::{AppError, Result};
use crate::models::{Activity, Goal, ResearchTask, SavedResearchTask, ChatConversation, ChatMessage, ChatMode, ChatConversationSummary};
use chrono::{DateTime, Utc};
use dirs::data_dir;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::path::PathBuf;
use uuid::Uuid;

pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub async fn new() -> Result<Self> {
        let db_path = Self::get_db_path()?;

        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::Database(format!("Failed to create database directory: {}", e))
            })?;
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
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS goals (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                duration_minutes INTEGER,
                allowed_apps TEXT NOT NULL,
                progress_percentage REAL NOT NULL DEFAULT 0,
                time_spent_minutes INTEGER NOT NULL DEFAULT 0,
                time_spent_seconds INTEGER NOT NULL DEFAULT 0,
                is_active BOOLEAN NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create goals table: {}", e)))?;

        // Add seconds column to existing goals table if it doesn't exist
        sqlx::query("ALTER TABLE goals ADD COLUMN time_spent_seconds INTEGER DEFAULT 0")
            .execute(&self.pool)
            .await
            .ok(); // Ignore error if column already exists

        // Activities table
        sqlx::query(
            r#"
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
        "#,
        )
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
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS research_tasks (
                id TEXT PRIMARY KEY NOT NULL,
                task_id TEXT NOT NULL,
                task_data TEXT NOT NULL,
                tags TEXT NOT NULL,
                notes TEXT,
                saved_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create research_tasks table: {}", e)))?;

        // Audio recordings metadata table
        sqlx::query(
            r#"
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
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::Database(format!("Failed to create audio_recordings table: {}", e))
        })?;

        // Documents table for RAG system
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                file_path TEXT NOT NULL,
                goal_id TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (goal_id) REFERENCES goals(id) ON DELETE SET NULL
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create documents table: {}", e)))?;

        // Document chunks table for RAG system
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS document_chunks (
                id TEXT PRIMARY KEY NOT NULL,
                document_id TEXT NOT NULL,
                content TEXT NOT NULL,
                embedding TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                metadata TEXT NOT NULL,
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::Database(format!("Failed to create document_chunks table: {}", e))
        })?;

        // Create indices for better query performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_documents_goal_id ON documents(goal_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to create documents goal_id index: {}", e))
            })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_chunks_document_id ON document_chunks(document_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::Database(format!("Failed to create chunks document_id index: {}", e))
        })?;

        // Chat conversations table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS chat_conversations (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                mode TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                message_count INTEGER NOT NULL DEFAULT 0,
                last_message_at TEXT
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::Database(format!("Failed to create chat_conversations table: {}", e))
        })?;

        // Chat messages table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS chat_messages (
                id TEXT PRIMARY KEY NOT NULL,
                conversation_id TEXT NOT NULL,
                content TEXT NOT NULL,
                is_user BOOLEAN NOT NULL,
                mode TEXT NOT NULL,
                created_at TEXT NOT NULL,
                sources TEXT,
                context_used BOOLEAN,
                research_task_id TEXT,
                metadata TEXT,
                FOREIGN KEY (conversation_id) REFERENCES chat_conversations(id) ON DELETE CASCADE
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::Database(format!("Failed to create chat_messages table: {}", e))
        })?;

        // Create indices for chat tables
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_messages_conversation_id ON chat_messages(conversation_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to create chat_messages conversation_id index: {}", e))
            })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_messages_created_at ON chat_messages(created_at)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to create chat_messages created_at index: {}", e))
            })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_conversations_mode ON chat_conversations(mode)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to create chat_conversations mode index: {}", e))
            })?;

        Ok(())
    }

    // Goals operations
    pub async fn save_goal(&self, goal: &Goal) -> Result<()> {
        let allowed_apps_json = serde_json::to_string(&goal.allowed_apps)
            .map_err(|e| AppError::Database(format!("Failed to serialize allowed_apps: {}", e)))?;

        sqlx::query(r#"
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
        "#)
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
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to save goal: {}", e)))?;

        Ok(())
    }

    pub async fn get_all_goals(&self) -> Result<Vec<Goal>> {
        let rows = sqlx::query(
            "SELECT id, name, duration_minutes, allowed_apps, progress_percentage, time_spent_minutes, time_spent_seconds, is_active, created_at, updated_at FROM goals ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch goals: {}", e)))?;

        let mut goals = Vec::new();
        for row in rows {
            let allowed_apps: Vec<String> =
                serde_json::from_str(&row.get::<String, _>("allowed_apps")).map_err(|e| {
                    AppError::Database(format!("Failed to parse allowed_apps: {}", e))
                })?;

            goals.push(Goal {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .map_err(|e| AppError::Database(format!("Invalid UUID: {}", e)))?,
                name: row.get("name"),
                target_duration_minutes: row.get::<Option<i32>, _>("duration_minutes").unwrap_or(0)
                    as u32,
                allowed_apps,
                current_duration_minutes: row.get::<i32, _>("time_spent_minutes") as u32,
                current_duration_seconds: row
                    .get::<Option<i32>, _>("time_spent_seconds")
                    .unwrap_or(0) as u32,
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
            use crate::models::{
                AppCategory, AppUsage, InputMetrics, ProjectContext, ProjectType, SystemState,
            };

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
                project_context: row.get::<Option<String>, _>("project_name").map(|name| {
                    ProjectContext {
                        project_name: name,
                        project_path: String::new(),
                        project_type: ProjectType::Other("Unknown".to_string()),
                        git_branch: None,
                    }
                }),
                goal_id: row
                    .get::<Option<String>, _>("goal_id")
                    .and_then(|id| Uuid::parse_str(&id).ok()),
            });
        }

        Ok(activities)
    }

    pub async fn get_activities_by_date_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
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

    // Document operations for RAG system
    pub async fn save_document(&self, document: &crate::rag::Document) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO documents (id, title, content, file_path, goal_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(document.id.to_string())
        .bind(&document.title)
        .bind(&document.content)
        .bind(&document.file_path)
        .bind(document.goal_id.map(|id| id.to_string()))
        .bind(document.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to save document: {}", e)))?;

        Ok(())
    }

    pub async fn save_document_chunk(&self, chunk: &crate::rag::DocumentChunk) -> Result<()> {
        let embedding_json = serde_json::to_string(&chunk.embedding)
            .map_err(|e| AppError::Database(format!("Failed to serialize embedding: {}", e)))?;
        let metadata_json = serde_json::to_string(&chunk.metadata)
            .map_err(|e| AppError::Database(format!("Failed to serialize metadata: {}", e)))?;

        sqlx::query(r#"
            INSERT OR REPLACE INTO document_chunks (id, document_id, content, embedding, chunk_index, metadata)
            VALUES (?, ?, ?, ?, ?, ?)
        "#)
        .bind(chunk.id.to_string())
        .bind(chunk.document_id.to_string())
        .bind(&chunk.content)
        .bind(embedding_json)
        .bind(chunk.chunk_index as i64)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to save document chunk: {}", e)))?;

        Ok(())
    }

    pub async fn load_documents(&self, goal_id: Option<Uuid>) -> Result<Vec<crate::rag::Document>> {
        let rows = if let Some(goal_id) = goal_id {
            sqlx::query("SELECT id, title, content, file_path, goal_id, created_at FROM documents WHERE goal_id = ?")
                .bind(goal_id.to_string())
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query("SELECT id, title, content, file_path, goal_id, created_at FROM documents")
                .fetch_all(&self.pool)
                .await
        }.map_err(|e| AppError::Database(format!("Failed to load documents: {}", e)))?;

        let mut documents = Vec::new();

        for row in rows {
            let document_id_str: String = row.get("id");
            let document_id = Uuid::parse_str(&document_id_str)
                .map_err(|e| AppError::Database(format!("Invalid document ID: {}", e)))?;

            let goal_id_str: Option<String> = row.get("goal_id");
            let goal_id = if let Some(goal_str) = goal_id_str {
                Some(
                    Uuid::parse_str(&goal_str)
                        .map_err(|e| AppError::Database(format!("Invalid goal ID: {}", e)))?,
                )
            } else {
                None
            };

            let created_at_str: String = row.get("created_at");
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| AppError::Database(format!("Invalid created_at format: {}", e)))?
                .with_timezone(&Utc);

            // Load chunks for this document
            let chunks = self.load_document_chunks(document_id).await?;

            let document = crate::rag::Document {
                id: document_id,
                title: row.get("title"),
                content: row.get("content"),
                file_path: row.get("file_path"),
                goal_id,
                chunks,
                created_at,
            };

            documents.push(document);
        }

        Ok(documents)
    }

    pub async fn load_document_chunks(
        &self,
        document_id: Uuid,
    ) -> Result<Vec<crate::rag::DocumentChunk>> {
        let rows = sqlx::query("SELECT id, content, embedding, chunk_index, metadata FROM document_chunks WHERE document_id = ? ORDER BY chunk_index")
            .bind(document_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to load document chunks: {}", e)))?;

        let mut chunks = Vec::new();

        for row in rows {
            let chunk_id_str: String = row.get("id");
            let chunk_id = Uuid::parse_str(&chunk_id_str)
                .map_err(|e| AppError::Database(format!("Invalid chunk ID: {}", e)))?;

            let embedding_json: String = row.get("embedding");
            let embedding: Vec<f32> = serde_json::from_str(&embedding_json).map_err(|e| {
                AppError::Database(format!("Failed to deserialize embedding: {}", e))
            })?;

            let metadata_json: String = row.get("metadata");
            let metadata: std::collections::HashMap<String, String> =
                serde_json::from_str(&metadata_json).map_err(|e| {
                    AppError::Database(format!("Failed to deserialize metadata: {}", e))
                })?;

            let chunk = crate::rag::DocumentChunk {
                id: chunk_id,
                document_id,
                content: row.get("content"),
                embedding,
                chunk_index: row.get::<i64, _>("chunk_index") as usize,
                metadata,
            };

            chunks.push(chunk);
        }

        Ok(chunks)
    }

    pub async fn delete_document(&self, document_id: Uuid) -> Result<()> {
        // Delete chunks first (due to foreign key constraint)
        sqlx::query("DELETE FROM document_chunks WHERE document_id = ?")
            .bind(document_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete document chunks: {}", e)))?;

        // Delete document
        sqlx::query("DELETE FROM documents WHERE id = ?")
            .bind(document_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete document: {}", e)))?;

        Ok(())
    }

    // Chat operations
    pub async fn create_conversation(&self, conversation: &ChatConversation) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO chat_conversations (
                id, title, mode, created_at, updated_at, message_count, last_message_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(conversation.id.to_string())
        .bind(&conversation.title)
        .bind(conversation.mode.to_string())
        .bind(conversation.created_at.to_rfc3339())
        .bind(conversation.updated_at.to_rfc3339())
        .bind(conversation.message_count as i64)
        .bind(conversation.last_message_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create conversation: {}", e)))?;

        Ok(())
    }

    pub async fn save_message(&self, message: &ChatMessage) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO chat_messages (
                id, conversation_id, content, is_user, mode, created_at, 
                sources, context_used, research_task_id, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(message.id.to_string())
        .bind(message.conversation_id.to_string())
        .bind(&message.content)
        .bind(message.is_user)
        .bind(message.mode.to_string())
        .bind(message.created_at.to_rfc3339())
        .bind(&message.sources)
        .bind(message.context_used)
        .bind(&message.research_task_id)
        .bind(&message.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to save message: {}", e)))?;

        // Update conversation message count and last message time
        sqlx::query(
            r#"
            UPDATE chat_conversations 
            SET message_count = message_count + 1, 
                last_message_at = ?, 
                updated_at = ? 
            WHERE id = ?
            "#,
        )
        .bind(message.created_at.to_rfc3339())
        .bind(message.created_at.to_rfc3339())
        .bind(message.conversation_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to update conversation: {}", e)))?;

        Ok(())
    }

    pub async fn get_conversations(&self) -> Result<Vec<ChatConversationSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT id, title, mode, message_count, last_message_at, created_at
            FROM chat_conversations 
            ORDER BY COALESCE(last_message_at, created_at) DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to get conversations: {}", e)))?;

        let mut conversations = Vec::new();
        for row in rows {
            let mode_str: String = row.get("mode");
            let mode = match mode_str.as_str() {
                "general" => ChatMode::General,
                "knowledge" => ChatMode::Knowledge,
                "research" => ChatMode::Research,
                _ => ChatMode::General,
            };

            conversations.push(ChatConversationSummary {
                id: Uuid::parse_str(&row.get::<String, _>("id")).map_err(|e| {
                    AppError::Database(format!("Invalid UUID in conversation: {}", e))
                })?,
                title: row.get("title"),
                mode,
                message_count: row.get::<i64, _>("message_count") as u32,
                last_message_at: row
                    .get::<Option<String>, _>("last_message_at")
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))
                    .map_err(|e| AppError::Database(format!("Invalid created_at: {}", e)))?
                    .with_timezone(&Utc),
            });
        }

        Ok(conversations)
    }

    pub async fn get_conversation_messages(&self, conversation_id: Uuid) -> Result<Vec<ChatMessage>> {
        let rows = sqlx::query(
            r#"
            SELECT id, conversation_id, content, is_user, mode, created_at,
                   sources, context_used, research_task_id, metadata
            FROM chat_messages 
            WHERE conversation_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(conversation_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to get messages: {}", e)))?;

        let mut messages = Vec::new();
        for row in rows {
            let mode_str: String = row.get("mode");
            let mode = match mode_str.as_str() {
                "general" => ChatMode::General,
                "knowledge" => ChatMode::Knowledge,
                "research" => ChatMode::Research,
                _ => ChatMode::General,
            };

            messages.push(ChatMessage {
                id: Uuid::parse_str(&row.get::<String, _>("id")).map_err(|e| {
                    AppError::Database(format!("Invalid UUID in message: {}", e))
                })?,
                conversation_id: Uuid::parse_str(&row.get::<String, _>("conversation_id"))
                    .map_err(|e| AppError::Database(format!("Invalid conversation UUID: {}", e)))?,
                content: row.get("content"),
                is_user: row.get("is_user"),
                mode,
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))
                    .map_err(|e| AppError::Database(format!("Invalid created_at: {}", e)))?
                    .with_timezone(&Utc),
                sources: row.get("sources"),
                context_used: row.get("context_used"),
                research_task_id: row.get("research_task_id"),
                metadata: row.get("metadata"),
            });
        }

        Ok(messages)
    }

    pub async fn delete_conversation(&self, conversation_id: Uuid) -> Result<()> {
        // Messages will be deleted by CASCADE
        sqlx::query("DELETE FROM chat_conversations WHERE id = ?")
            .bind(conversation_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete conversation: {}", e)))?;

        Ok(())
    }

    pub async fn update_conversation_title(&self, conversation_id: Uuid, title: String) -> Result<()> {
        sqlx::query(
            "UPDATE chat_conversations SET title = ?, updated_at = ? WHERE id = ?",
        )
        .bind(title)
        .bind(Utc::now().to_rfc3339())
        .bind(conversation_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to update conversation title: {}", e)))?;

        Ok(())
    }
}
