use crate::rag::RAGSystem;
use tauri::State;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

type RAGState = Arc<Mutex<RAGSystem>>;

#[tauri::command]
pub async fn initialize_rag(rag_system: State<'_, RAGState>) -> std::result::Result<String, String> {
    let _rag = rag_system.lock().await;
    // RAG system is already initialized when created
    Ok("RAG system initialized successfully".to_string())
}

#[tauri::command]
pub async fn index_document(
    rag_system: State<'_, RAGState>,
    file_path: String,
    goal_id: Option<String>,
) -> std::result::Result<String, String> {
    let mut rag = rag_system.lock().await;
    
    let goal_uuid = if let Some(goal_str) = goal_id {
        Some(Uuid::parse_str(&goal_str).map_err(|e| e.to_string())?)
    } else {
        None
    };

    let document = rag.index_document(&file_path, goal_uuid).await.map_err(|e| e.to_string())?;
    
    Ok(format!("Document indexed successfully: {}", document.id))
}

#[tauri::command]
pub async fn search_documents(
    rag_system: State<'_, RAGState>,
    query: String,
    goal_id: Option<String>,
    limit: Option<usize>,
) -> std::result::Result<Vec<SearchResultResponse>, String> {
    let rag = rag_system.lock().await;
    
    let goal_uuid = if let Some(goal_str) = goal_id {
        Some(Uuid::parse_str(&goal_str).map_err(|e| e.to_string())?)
    } else {
        None
    };

    let results = rag.search(&query, goal_uuid, limit.unwrap_or(10)).await.map_err(|e| e.to_string())?;
    
    let response: Vec<SearchResultResponse> = results.into_iter().map(|r| SearchResultResponse {
        document_id: r.document_id.to_string(),
        chunk_id: r.chunk_id.to_string(),
        content: r.content,
        score: r.score,
        metadata: r.metadata,
    }).collect();

    Ok(response)
}

#[tauri::command]
pub async fn get_goal_context(
    rag_system: State<'_, RAGState>,
    goal_id: String,
    limit: Option<usize>,
) -> std::result::Result<Vec<SearchResultResponse>, String> {
    let rag = rag_system.lock().await;
    
    let goal_uuid = Uuid::parse_str(&goal_id).map_err(|e| e.to_string())?;
    let results = rag.get_goal_context(goal_uuid, limit.unwrap_or(10)).await.map_err(|e| e.to_string())?;
    
    let response: Vec<SearchResultResponse> = results.into_iter().map(|r| SearchResultResponse {
        document_id: r.document_id.to_string(),
        chunk_id: r.chunk_id.to_string(),
        content: r.content,
        score: r.score,
        metadata: r.metadata,
    }).collect();

    Ok(response)
}

#[tauri::command]
pub async fn list_indexed_documents(
    rag_system: State<'_, RAGState>,
    goal_id: Option<String>,
) -> std::result::Result<Vec<DocumentResponse>, String> {
    let rag = rag_system.lock().await;
    
    let goal_uuid = if let Some(goal_str) = goal_id {
        Some(Uuid::parse_str(&goal_str).map_err(|e| e.to_string())?)
    } else {
        None
    };

    let documents = rag.list_documents(goal_uuid).await.map_err(|e| e.to_string())?;
    
    let response: Vec<DocumentResponse> = documents.into_iter().map(|d| DocumentResponse {
        id: d.id.to_string(),
        title: d.title,
        file_path: d.file_path,
        goal_id: d.goal_id.map(|id| id.to_string()),
        chunks_count: d.chunks.len(),
        created_at: d.created_at.to_rfc3339(),
    }).collect();

    Ok(response)
}

#[tauri::command]
pub async fn remove_document(
    rag_system: State<'_, RAGState>,
    document_id: String,
) -> std::result::Result<String, String> {
    let mut rag = rag_system.lock().await;
    
    let doc_uuid = Uuid::parse_str(&document_id).map_err(|e| e.to_string())?;
    rag.remove_document(doc_uuid).await.map_err(|e| e.to_string())?;
    
    Ok("Document removed successfully".to_string())
}

#[tauri::command]
pub async fn update_document_index(
    rag_system: State<'_, RAGState>,
    document_id: String,
    file_path: String,
) -> std::result::Result<String, String> {
    let mut rag = rag_system.lock().await;
    
    let doc_uuid = Uuid::parse_str(&document_id).map_err(|e| e.to_string())?;
    let document = rag.update_document(doc_uuid, &file_path).await.map_err(|e| e.to_string())?;
    
    Ok(format!("Document updated successfully: {}", document.id))
}

#[tauri::command]
pub async fn get_supported_file_types() -> std::result::Result<Vec<String>, String> {
    let processor = crate::rag::DocumentProcessor::new();
    let extensions = processor.get_supported_extensions();
    Ok(extensions.into_iter().map(|s| s.to_string()).collect())
}

#[tauri::command]
pub async fn check_file_supported(file_path: String) -> std::result::Result<bool, String> {
    let processor = crate::rag::DocumentProcessor::new();
    Ok(processor.is_supported_file(&file_path))
}

// Response types for frontend
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SearchResultResponse {
    pub document_id: String,
    pub chunk_id: String,
    pub content: String,
    pub score: f32,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DocumentResponse {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub goal_id: Option<String>,
    pub chunks_count: usize,
    pub created_at: String,
}