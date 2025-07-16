use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConversation {
    pub id: Uuid,
    pub title: String,
    pub mode: ChatMode,
    pub goal_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: u32,
    pub last_message_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub content: String,
    pub is_user: bool,
    pub mode: ChatMode,
    pub created_at: DateTime<Utc>,
    pub sources: Option<String>, // JSON serialized DocumentSource array
    pub context_used: Option<bool>,
    pub research_task_id: Option<String>,
    pub metadata: Option<String>, // JSON for additional mode-specific data
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatMode {
    General,
    Knowledge,
    Research,
}

impl std::fmt::Display for ChatMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatMode::General => write!(f, "general"),
            ChatMode::Knowledge => write!(f, "knowledge"),
            ChatMode::Research => write!(f, "research"),
        }
    }
}

impl ChatConversation {
    pub fn new(title: String, mode: ChatMode, goal_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            mode,
            goal_id,
            created_at: now,
            updated_at: now,
            message_count: 0,
            last_message_at: None,
        }
    }

    pub fn add_message(&mut self) {
        self.message_count += 1;
        self.last_message_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn update_title(&mut self, title: String) {
        self.title = title;
        self.updated_at = Utc::now();
    }
}

impl ChatMessage {
    pub fn new(conversation_id: Uuid, content: String, is_user: bool, mode: ChatMode) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            content,
            is_user,
            mode,
            created_at: Utc::now(),
            sources: None,
            context_used: None,
            research_task_id: None,
            metadata: None,
        }
    }

    pub fn with_sources(mut self, sources: Vec<crate::services::llm::DocumentSource>) -> Self {
        if !sources.is_empty() {
            self.sources = serde_json::to_string(&sources).ok();
        }
        self
    }

    pub fn with_context(mut self, context_used: bool) -> Self {
        self.context_used = Some(context_used);
        self
    }

    pub fn with_research_task(mut self, task_id: String) -> Self {
        self.research_task_id = Some(task_id);
        self
    }

    pub fn with_metadata<T: Serialize>(mut self, metadata: T) -> Self {
        self.metadata = serde_json::to_string(&metadata).ok();
        self
    }

    pub fn get_sources(&self) -> Option<Vec<crate::services::llm::DocumentSource>> {
        self.sources
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }

    pub fn get_metadata<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        self.metadata
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConversationSummary {
    pub id: Uuid,
    pub title: String,
    pub mode: ChatMode,
    pub goal_id: Uuid,
    pub message_count: u32,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<ChatConversation> for ChatConversationSummary {
    fn from(conversation: ChatConversation) -> Self {
        Self {
            id: conversation.id,
            title: conversation.title,
            mode: conversation.mode,
            goal_id: conversation.goal_id,
            message_count: conversation.message_count,
            last_message_at: conversation.last_message_at,
            created_at: conversation.created_at,
        }
    }
}
