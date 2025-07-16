use crate::database::SqliteDatabase;
use crate::goals::GoalService;
use crate::models::{ChatConversation, ChatConversationSummary, ChatMessage, ChatMode};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use uuid::Uuid;

#[tauri::command]
pub async fn create_chat_conversation(
    db: State<'_, Arc<Mutex<SqliteDatabase>>>,
    goal_service: State<'_, Arc<Mutex<GoalService>>>,
    title: String,
    mode: String,
) -> std::result::Result<String, String> {
    let chat_mode = match mode.as_str() {
        "general" => ChatMode::General,
        "knowledge" => ChatMode::Knowledge,
        "research" => ChatMode::Research,
        _ => ChatMode::General,
    };

    // Get the current active goal or default goal
    let goal_id = {
        let goal_service = goal_service.lock().await;
        goal_service.get_current_or_default_goal_id()
    };

    let conversation = ChatConversation::new(title, chat_mode, goal_id);
    let conversation_id = conversation.id;

    let db = db.lock().await;
    match db.create_conversation(&conversation).await {
        Ok(_) => {
            println!(
                "Created chat conversation: {} for goal: {}",
                conversation_id, goal_id
            );
            Ok(conversation_id.to_string())
        }
        Err(e) => {
            eprintln!("Failed to create conversation: {}", e);
            Err(format!("Failed to create conversation: {}", e))
        }
    }
}

#[tauri::command]
pub async fn save_chat_message(
    db: State<'_, Arc<Mutex<SqliteDatabase>>>,
    conversation_id: String,
    content: String,
    is_user: bool,
    mode: String,
    sources: Option<String>,
    context_used: Option<bool>,
    research_task_id: Option<String>,
    metadata: Option<String>,
) -> std::result::Result<String, String> {
    let conversation_uuid =
        Uuid::parse_str(&conversation_id).map_err(|e| format!("Invalid conversation ID: {}", e))?;

    let chat_mode = match mode.as_str() {
        "general" => ChatMode::General,
        "knowledge" => ChatMode::Knowledge,
        "research" => ChatMode::Research,
        _ => ChatMode::General,
    };

    let mut message = ChatMessage::new(conversation_uuid, content, is_user, chat_mode);
    message.sources = sources;
    message.context_used = context_used;
    message.research_task_id = research_task_id;
    message.metadata = metadata;

    let message_id = message.id;

    let db = db.lock().await;
    match db.save_message(&message).await {
        Ok(_) => {
            println!("Saved chat message: {}", message_id);
            Ok(message_id.to_string())
        }
        Err(e) => {
            eprintln!("Failed to save message: {}", e);
            Err(format!("Failed to save message: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_chat_conversations(
    db: State<'_, Arc<Mutex<SqliteDatabase>>>,
) -> std::result::Result<Vec<ChatConversationSummary>, String> {
    let db = db.lock().await;
    match db.get_conversations().await {
        Ok(conversations) => {
            println!("Retrieved {} conversations", conversations.len());
            Ok(conversations)
        }
        Err(e) => {
            eprintln!("Failed to get conversations: {}", e);
            Err(format!("Failed to get conversations: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_chat_messages(
    db: State<'_, Arc<Mutex<SqliteDatabase>>>,
    conversation_id: String,
) -> std::result::Result<Vec<ChatMessage>, String> {
    let conversation_uuid =
        Uuid::parse_str(&conversation_id).map_err(|e| format!("Invalid conversation ID: {}", e))?;

    let db = db.lock().await;
    match db.get_conversation_messages(conversation_uuid).await {
        Ok(messages) => {
            println!(
                "Retrieved {} messages for conversation {}",
                messages.len(),
                conversation_id
            );
            Ok(messages)
        }
        Err(e) => {
            eprintln!("Failed to get messages: {}", e);
            Err(format!("Failed to get messages: {}", e))
        }
    }
}

#[tauri::command]
pub async fn delete_chat_conversation(
    db: State<'_, Arc<Mutex<SqliteDatabase>>>,
    conversation_id: String,
) -> std::result::Result<(), String> {
    let conversation_uuid =
        Uuid::parse_str(&conversation_id).map_err(|e| format!("Invalid conversation ID: {}", e))?;

    let db = db.lock().await;
    match db.delete_conversation(conversation_uuid).await {
        Ok(_) => {
            println!("Deleted conversation: {}", conversation_id);
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to delete conversation: {}", e);
            Err(format!("Failed to delete conversation: {}", e))
        }
    }
}

#[tauri::command]
pub async fn update_chat_conversation_title(
    db: State<'_, Arc<Mutex<SqliteDatabase>>>,
    conversation_id: String,
    title: String,
) -> std::result::Result<(), String> {
    let conversation_uuid =
        Uuid::parse_str(&conversation_id).map_err(|e| format!("Invalid conversation ID: {}", e))?;

    let db = db.lock().await;
    match db.update_conversation_title(conversation_uuid, title).await {
        Ok(_) => {
            println!("Updated conversation title: {}", conversation_id);
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to update conversation title: {}", e);
            Err(format!("Failed to update conversation title: {}", e))
        }
    }
}
