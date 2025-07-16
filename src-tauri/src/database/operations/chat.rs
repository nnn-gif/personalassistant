use crate::error::{AppError, Result};
use crate::models::{ChatConversation, ChatConversationSummary, ChatMessage, ChatMode};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub async fn create_conversation(pool: &SqlitePool, conversation: &ChatConversation) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO chat_conversations (
            id, title, mode, goal_id, created_at, updated_at, message_count, last_message_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(conversation.id.to_string())
    .bind(&conversation.title)
    .bind(conversation.mode.to_string())
    .bind(conversation.goal_id.to_string())
    .bind(conversation.created_at.to_rfc3339())
    .bind(conversation.updated_at.to_rfc3339())
    .bind(conversation.message_count as i64)
    .bind(conversation.last_message_at.map(|dt| dt.to_rfc3339()))
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to create conversation: {}", e)))?;

    Ok(())
}

pub async fn save_message(pool: &SqlitePool, message: &ChatMessage) -> Result<()> {
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
    .execute(pool)
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
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to update conversation: {}", e)))?;

    Ok(())
}

pub async fn get_conversations(pool: &SqlitePool) -> Result<Vec<ChatConversationSummary>> {
    let rows = sqlx::query(
        r#"
        SELECT id, title, mode, goal_id, message_count, last_message_at, created_at
        FROM chat_conversations 
        ORDER BY COALESCE(last_message_at, created_at) DESC
        "#,
    )
    .fetch_all(pool)
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
            goal_id: Uuid::parse_str(&row.get::<String, _>("goal_id")).map_err(|e| {
                AppError::Database(format!("Invalid goal UUID in conversation: {}", e))
            })?,
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

pub async fn get_conversation_messages(
    pool: &SqlitePool,
    conversation_id: Uuid,
) -> Result<Vec<ChatMessage>> {
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
    .fetch_all(pool)
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
            id: Uuid::parse_str(&row.get::<String, _>("id"))
                .map_err(|e| AppError::Database(format!("Invalid UUID in message: {}", e)))?,
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

pub async fn delete_conversation(pool: &SqlitePool, conversation_id: Uuid) -> Result<()> {
    // Messages will be deleted by CASCADE
    sqlx::query("DELETE FROM chat_conversations WHERE id = ?")
        .bind(conversation_id.to_string())
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to delete conversation: {}", e)))?;

    Ok(())
}

pub async fn update_conversation_title(
    pool: &SqlitePool,
    conversation_id: Uuid,
    title: String,
) -> Result<()> {
    sqlx::query("UPDATE chat_conversations SET title = ?, updated_at = ? WHERE id = ?")
        .bind(title)
        .bind(Utc::now().to_rfc3339())
        .bind(conversation_id.to_string())
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::Database(format!("Failed to update conversation title: {}", e))
        })?;

    Ok(())
}