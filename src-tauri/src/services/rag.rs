use crate::rag::{EnhancedDocumentProcessor, RAGSystemWrapper};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use uuid::Uuid;

type RAGState = Arc<Mutex<RAGSystemWrapper>>;

#[tauri::command]
pub async fn initialize_rag(
    rag_system: State<'_, RAGState>,
) -> std::result::Result<String, String> {
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

    println!("Indexing document synchronously: {file_path}");
    let document = rag
        .index_document(&file_path, goal_uuid)
        .await
        .map_err(|e| {
            eprintln!(
                "Failed to index document synchronously {file_path}: {e}"
            );
            e.to_string()
        })?;

    println!(
        "Successfully indexed document synchronously: {} with ID {}",
        document.title, document.id
    );
    Ok(format!("Document indexed successfully: {}", document.id))
}

#[tauri::command]
pub async fn index_document_async(
    app: AppHandle,
    rag_system: State<'_, RAGState>,
    file_path: String,
    goal_id: Option<String>,
    task_id: String,
) -> std::result::Result<String, String> {
    let rag_system = rag_system.inner().clone();
    let app_handle = app.clone();
    let task_id_clone = task_id.clone();

    // Spawn async task to prevent UI blocking
    tokio::spawn(async move {
        // Emit start event
        let _ = app_handle.emit(
            "indexing-progress",
            IndexingProgress {
                task_id: task_id_clone.clone(),
                status: "starting".to_string(),
                current_file: file_path.clone(),
                progress: 0,
                total: 1,
                phase: "Preparing".to_string(),
                error: None,
            },
        );

        let goal_uuid = if let Some(goal_str) = goal_id {
            match Uuid::parse_str(&goal_str) {
                Ok(uuid) => Some(uuid),
                Err(e) => {
                    let _ = app_handle.emit(
                        "indexing-progress",
                        IndexingProgress {
                            task_id: task_id_clone.clone(),
                            status: "error".to_string(),
                            current_file: file_path.clone(),
                            progress: 0,
                            total: 1,
                            phase: "Error".to_string(),
                            error: Some(format!("Invalid goal ID: {e}")),
                        },
                    );
                    return;
                }
            }
        } else {
            None
        };

        // Emit processing event
        let _ = app_handle.emit(
            "indexing-progress",
            IndexingProgress {
                task_id: task_id_clone.clone(),
                status: "processing".to_string(),
                current_file: file_path.clone(),
                progress: 0,
                total: 1,
                phase: "Processing document".to_string(),
                error: None,
            },
        );

        // Perform actual indexing
        let mut rag = rag_system.lock().await;

        println!("Starting to index document: {file_path}");
        match rag.index_document(&file_path, goal_uuid).await {
            Ok(document) => {
                println!(
                    "Successfully indexed document: {} with {} chunks",
                    document.title,
                    document.chunks.len()
                );
                let _ = app_handle.emit(
                    "indexing-progress",
                    IndexingProgress {
                        task_id: task_id_clone.clone(),
                        status: "completed".to_string(),
                        current_file: file_path.clone(),
                        progress: 1,
                        total: 1,
                        phase: "Completed".to_string(),
                        error: None,
                    },
                );

                let _ = app_handle.emit(
                    "document-indexed",
                    DocumentIndexedEvent {
                        task_id: task_id_clone,
                        document_id: document.id.to_string(),
                        title: document.title,
                        chunks_count: document.chunks.len(),
                    },
                );
            }
            Err(e) => {
                eprintln!("Failed to index document {file_path}: {e}");
                let _ = app_handle.emit(
                    "indexing-progress",
                    IndexingProgress {
                        task_id: task_id_clone,
                        status: "error".to_string(),
                        current_file: file_path.clone(),
                        progress: 0,
                        total: 1,
                        phase: "Error".to_string(),
                        error: Some(e.to_string()),
                    },
                );
            }
        }
    });

    Ok(format!("Indexing started with task ID: {task_id}"))
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

    let results = rag
        .search(&query, goal_uuid, limit.unwrap_or(10))
        .await
        .map_err(|e| e.to_string())?;

    let response: Vec<SearchResultResponse> = results
        .into_iter()
        .map(|r| SearchResultResponse {
            document_id: r.document_id.to_string(),
            chunk_id: r.chunk_id.to_string(),
            content: r.content,
            score: r.score,
            metadata: r.metadata,
        })
        .collect();

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
    let results = rag
        .get_goal_context(goal_uuid, limit.unwrap_or(10))
        .await
        .map_err(|e| e.to_string())?;

    let response: Vec<SearchResultResponse> = results
        .into_iter()
        .map(|r| SearchResultResponse {
            document_id: r.document_id.to_string(),
            chunk_id: r.chunk_id.to_string(),
            content: r.content,
            score: r.score,
            metadata: r.metadata,
        })
        .collect();

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

    let documents = rag
        .list_documents(goal_uuid)
        .await
        .map_err(|e| e.to_string())?;

    let response: Vec<DocumentResponse> = documents
        .into_iter()
        .map(|d| DocumentResponse {
            id: d.id.to_string(),
            title: d.title,
            file_path: d.file_path,
            goal_id: d.goal_id.map(|id| id.to_string()),
            chunks_count: d.chunks.len(),
            created_at: d.created_at.to_rfc3339(),
        })
        .collect();

    Ok(response)
}

#[tauri::command]
pub async fn remove_document(
    rag_system: State<'_, RAGState>,
    document_id: String,
) -> std::result::Result<String, String> {
    let mut rag = rag_system.lock().await;

    let doc_uuid = Uuid::parse_str(&document_id).map_err(|e| e.to_string())?;
    rag.remove_document(doc_uuid)
        .await
        .map_err(|e| e.to_string())?;

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
    let document = rag
        .update_document(doc_uuid, &file_path)
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!("Document updated successfully: {}", document.id))
}

#[tauri::command]
pub async fn get_supported_file_types() -> std::result::Result<Vec<String>, String> {
    let processor = EnhancedDocumentProcessor::new();
    let extensions = processor.get_supported_extensions();
    Ok(extensions.into_iter().map(|s| s.to_string()).collect())
}

#[tauri::command]
pub async fn check_file_supported(file_path: String) -> std::result::Result<bool, String> {
    let processor = EnhancedDocumentProcessor::new();
    Ok(processor.is_supported_file(&file_path))
}

#[tauri::command]
pub async fn get_enhanced_file_info(
    file_path: String,
) -> std::result::Result<EnhancedFileInfo, String> {
    let processor = EnhancedDocumentProcessor::new();

    let is_supported = processor.is_supported_file(&file_path);
    let stats = processor
        .get_file_stats(&file_path)
        .map_err(|e| e.to_string())?;

    Ok(EnhancedFileInfo {
        file_path,
        is_supported,
        stats,
        supported_extensions: processor
            .get_supported_extensions()
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
    })
}

#[tauri::command]
pub async fn inspect_rag_database(
    rag_system: State<'_, RAGState>,
) -> std::result::Result<DatabaseInspection, String> {
    let rag = rag_system.lock().await;

    // Get all documents
    let documents = rag.list_documents(None).await.map_err(|e| e.to_string())?;

    let mut document_summaries = Vec::new();
    let mut total_chunks = 0;
    let mut corrupted_count = 0;

    for document in documents {
        let content_preview = if document.content.len() > 200 {
            format!("{}...", &document.content[..200])
        } else {
            document.content.clone()
        };

        let is_corrupted = {
            let content_lower = document.content.to_lowercase();
            content_lower.contains("identity-h")
                || content_lower.contains("unimplemented")
                || content_lower.contains("unimpl")
                || document.content.trim().is_empty()
        };

        if is_corrupted {
            corrupted_count += 1;
        }

        total_chunks += document.chunks.len();

        document_summaries.push(DocumentSummary {
            id: document.id.to_string(),
            title: document.title,
            file_path: document.file_path,
            content_preview,
            chunks_count: document.chunks.len(),
            goal_id: document.goal_id.map(|g| g.to_string()),
            created_at: document.created_at.to_rfc3339(),
            is_corrupted,
            content_length: document.content.len(),
        });
    }

    Ok(DatabaseInspection {
        total_documents: document_summaries.len(),
        total_chunks,
        corrupted_documents: corrupted_count,
        documents: document_summaries,
    })
}

#[tauri::command]
pub async fn cleanup_corrupted_documents(
    rag_system: State<'_, RAGState>,
) -> std::result::Result<CleanupResult, String> {
    let mut rag = rag_system.lock().await;

    // Get all documents
    let documents = rag.list_documents(None).await.map_err(|e| e.to_string())?;

    let mut removed_count = 0;
    let mut removed_ids = Vec::new();

    for document in documents {
        // Check if document content contains corrupted text
        let content_lower = document.content.to_lowercase();
        if content_lower.contains("identity-h")
            || content_lower.contains("unimplemented")
            || content_lower.contains("unimpl")
            || document.content.trim().is_empty()
            || document.chunks.is_empty()
        {
            println!(
                "Removing corrupted document: {} ({})",
                document.title, document.id
            );

            match rag.remove_document(document.id).await {
                Ok(_) => {
                    removed_count += 1;
                    removed_ids.push(document.id.to_string());
                }
                Err(e) => {
                    eprintln!("Failed to remove document {}: {e}", document.id);
                }
            }
        }
    }

    Ok(CleanupResult {
        removed_count,
        removed_ids,
    })
}

#[tauri::command]
pub async fn clear_vector_database(
    rag_system: State<'_, RAGState>,
) -> std::result::Result<ClearDatabaseResult, String> {
    let mut rag = rag_system.lock().await;

    println!("Clearing entire vector database...");

    // Get all documents first
    let documents = rag.list_documents(None).await.map_err(|e| e.to_string())?;
    let total_documents = documents.len();

    let mut removed_count = 0;
    let mut removed_ids = Vec::new();
    let mut failed_removals = Vec::new();

    // Remove all documents
    for document in documents {
        match rag.remove_document(document.id).await {
            Ok(_) => {
                removed_count += 1;
                removed_ids.push(document.id.to_string());
                println!("Removed document: {} ({})", document.title, document.id);
            }
            Err(e) => {
                eprintln!("Failed to remove document {}: {e}", document.id);
                failed_removals.push(format!("{}: {e}", document.title));
            }
        }
    }

    println!(
        "Vector database cleared: {removed_count} documents removed, {} failed",
        failed_removals.len()
    );

    Ok(ClearDatabaseResult {
        total_documents,
        removed_count,
        removed_ids,
        failed_removals,
    })
}

#[derive(serde::Serialize)]
pub struct DatabaseInspection {
    pub total_documents: usize,
    pub total_chunks: usize,
    pub corrupted_documents: usize,
    pub documents: Vec<DocumentSummary>,
}

#[derive(serde::Serialize)]
pub struct DocumentSummary {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub content_preview: String,
    pub chunks_count: usize,
    pub goal_id: Option<String>,
    pub created_at: String,
    pub is_corrupted: bool,
    pub content_length: usize,
}

#[derive(serde::Serialize)]
pub struct CleanupResult {
    pub removed_count: usize,
    pub removed_ids: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct ClearDatabaseResult {
    pub total_documents: usize,
    pub removed_count: usize,
    pub removed_ids: Vec<String>,
    pub failed_removals: Vec<String>,
}

// Event types for async indexing
#[derive(serde::Serialize, Clone)]
pub struct IndexingProgress {
    pub task_id: String,
    pub status: String, // "starting", "processing", "completed", "error"
    pub current_file: String,
    pub progress: usize,
    pub total: usize,
    pub phase: String,
    pub error: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct DocumentIndexedEvent {
    pub task_id: String,
    pub document_id: String,
    pub title: String,
    pub chunks_count: usize,
}

#[derive(serde::Serialize, Clone)]
pub struct BatchIndexingResult {
    pub task_id: String,
    pub total_files: usize,
    pub successful: usize,
    pub failed: usize,
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
pub struct EnhancedFileInfo {
    pub file_path: String,
    pub is_supported: bool,
    pub stats: std::collections::HashMap<String, String>,
    pub supported_extensions: Vec<String>,
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
