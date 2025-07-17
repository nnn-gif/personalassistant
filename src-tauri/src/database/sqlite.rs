use crate::config::Config;
use crate::error::{AppError, Result};
use crate::models::{
    Activity, ChatConversation, ChatConversationSummary, ChatMessage, Goal, SavedResearchTask,
};
use crate::rag::{Document, DocumentChunk};
use chrono::{DateTime, Utc};
use dirs::data_dir;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::PathBuf;
use uuid::Uuid;

use super::operations;

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
        let config = Config::get();
        let data_dir = data_dir()
            .ok_or_else(|| AppError::Database("Could not find data directory".to_string()))?;
        Ok(data_dir.join("personalassistant").join(&config.database.db_name))
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
                goal_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                message_count INTEGER NOT NULL DEFAULT 0,
                last_message_at TEXT,
                FOREIGN KEY (goal_id) REFERENCES goals(id) ON DELETE SET NULL
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
        .map_err(|e| AppError::Database(format!("Failed to create chat_messages table: {}", e)))?;

        // Create indices for chat tables
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_messages_conversation_id ON chat_messages(conversation_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to create chat_messages conversation_id index: {}", e))
            })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_chat_messages_created_at ON chat_messages(created_at)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::Database(format!(
                "Failed to create chat_messages created_at index: {}",
                e
            ))
        })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_chat_conversations_mode ON chat_conversations(mode)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::Database(format!(
                "Failed to create chat_conversations mode index: {}",
                e
            ))
        })?;

        Ok(())
    }

    // Goals operations
    pub async fn save_goal(&self, goal: &Goal) -> Result<()> {
        operations::goals::save_goal(&self.pool, goal).await
    }

    pub async fn get_all_goals(&self) -> Result<Vec<Goal>> {
        operations::goals::get_all_goals(&self.pool).await
    }

    pub async fn delete_goal(&self, goal_id: &Uuid) -> Result<()> {
        operations::goals::delete_goal(&self.pool, goal_id).await
    }

    // Activities operations
    pub async fn save_activity(&self, activity: &Activity) -> Result<()> {
        operations::activities::save_activity(&self.pool, activity).await
    }

    pub async fn get_recent_activities(&self, limit: i32) -> Result<Vec<Activity>> {
        operations::activities::get_recent_activities(&self.pool, limit).await
    }

    pub async fn get_activities_by_date_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Activity>> {
        println!("[SqliteDatabase::get_activities_by_date_range] Called with:");
        println!("  Start: {} ({})", start.to_rfc3339(), start.format("%Y-%m-%d %H:%M:%S"));
        println!("  End: {} ({})", end.to_rfc3339(), end.format("%Y-%m-%d %H:%M:%S"));
        
        let result = operations::activities::get_activities_by_date_range(&self.pool, start, end).await;
        
        match &result {
            Ok(activities) => {
                println!("[SqliteDatabase::get_activities_by_date_range] Successfully retrieved {} activities", activities.len());
            }
            Err(e) => {
                eprintln!("[SqliteDatabase::get_activities_by_date_range] Failed to get activities: {}", e);
            }
        }
        
        result
    }

    // Research operations
    pub async fn save_research(&self, task: &SavedResearchTask) -> Result<()> {
        operations::research::save_research(&self.pool, task).await
    }

    pub async fn get_all_research(&self) -> Result<Vec<SavedResearchTask>> {
        operations::research::get_all_research(&self.pool).await
    }

    pub async fn delete_research(&self, id: &Uuid) -> Result<()> {
        operations::research::delete_research(&self.pool, id).await
    }

    // Document operations for RAG system
    pub async fn save_document(&self, document: &Document) -> Result<()> {
        operations::rag::save_document(&self.pool, document).await
    }

    pub async fn save_document_chunk(&self, chunk: &DocumentChunk) -> Result<()> {
        operations::rag::save_document_chunk(&self.pool, chunk).await
    }

    pub async fn load_documents(&self, goal_id: Option<Uuid>) -> Result<Vec<Document>> {
        operations::rag::load_documents(&self.pool, goal_id).await
    }

    pub async fn load_document_chunks(&self, document_id: Uuid) -> Result<Vec<DocumentChunk>> {
        operations::rag::load_document_chunks(&self.pool, document_id).await
    }

    pub async fn delete_document(&self, document_id: Uuid) -> Result<()> {
        operations::rag::delete_document(&self.pool, document_id).await
    }

    // Chat operations
    pub async fn create_conversation(&self, conversation: &ChatConversation) -> Result<()> {
        operations::chat::create_conversation(&self.pool, conversation).await
    }

    pub async fn save_message(&self, message: &ChatMessage) -> Result<()> {
        operations::chat::save_message(&self.pool, message).await
    }

    pub async fn get_conversations(&self) -> Result<Vec<ChatConversationSummary>> {
        operations::chat::get_conversations(&self.pool).await
    }

    pub async fn get_conversation_messages(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<ChatMessage>> {
        operations::chat::get_conversation_messages(&self.pool, conversation_id).await
    }

    pub async fn delete_conversation(&self, conversation_id: Uuid) -> Result<()> {
        operations::chat::delete_conversation(&self.pool, conversation_id).await
    }

    pub async fn update_conversation_title(
        &self,
        conversation_id: Uuid,
        title: String,
    ) -> Result<()> {
        operations::chat::update_conversation_title(&self.pool, conversation_id, title).await
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
