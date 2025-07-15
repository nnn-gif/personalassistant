use crate::error::Result;
use mime_guess::MimeGuess;
use std::path::Path;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use uuid::Uuid;
use walkdir::WalkDir;

#[tauri::command]
pub async fn scan_folder_for_documents(
    folder_path: String,
    include_subdirs: Option<bool>,
    max_depth: Option<usize>,
) -> std::result::Result<Vec<String>, String> {
    let path = Path::new(&folder_path);
    if !path.exists() || !path.is_dir() {
        return Err("Invalid folder path".to_string());
    }

    let include_subdirs = include_subdirs.unwrap_or(true);
    let max_depth = max_depth.unwrap_or(10);

    let processor = crate::rag::DocumentProcessor::new();
    let mut document_files = Vec::new();

    let walker = if include_subdirs {
        WalkDir::new(path).max_depth(max_depth)
    } else {
        WalkDir::new(path).max_depth(1)
    };

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let file_path = entry.path();

        if file_path.is_file() {
            if let Some(path_str) = file_path.to_str() {
                if processor.is_supported_file(path_str) {
                    document_files.push(path_str.to_string());
                }
            }
        }
    }

    Ok(document_files)
}

#[tauri::command]
pub async fn get_file_info(file_path: String) -> std::result::Result<FileInfo, String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err("File does not exist".to_string());
    }

    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mime_type = MimeGuess::from_path(path).first_or_octet_stream();
    let file_extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_string();

    let is_supported = crate::rag::DocumentProcessor::new().is_supported_file(&file_path);

    Ok(FileInfo {
        name: file_name,
        path: file_path,
        size: metadata.len(),
        mime_type: mime_type.to_string(),
        extension: file_extension,
        modified: metadata
            .modified()
            .map(|time| {
                time.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            })
            .unwrap_or(0),
        is_supported,
    })
}

#[tauri::command]
pub async fn index_multiple_documents(
    rag_system: State<'_, Arc<Mutex<crate::rag::RAGSystem>>>,
    file_paths: Vec<String>,
    goal_id: Option<String>,
) -> std::result::Result<IndexingResult, String> {
    let mut rag = rag_system.lock().await;
    let goal_uuid = if let Some(goal_str) = goal_id {
        Some(Uuid::parse_str(&goal_str).map_err(|e| e.to_string())?)
    } else {
        None
    };

    let mut successful = Vec::new();
    let mut failed = Vec::new();

    for file_path in file_paths {
        match rag.index_document(&file_path, goal_uuid).await {
            Ok(document) => {
                successful.push(IndexedDocumentInfo {
                    id: document.id.to_string(),
                    path: file_path,
                    title: document.title,
                    chunks_count: document.chunks.len(),
                });
            }
            Err(e) => {
                failed.push(FailedIndexInfo {
                    path: file_path,
                    error: e.to_string(),
                });
            }
        }
    }

    let total_processed = successful.len() + failed.len();

    Ok(IndexingResult {
        successful,
        failed,
        total_processed,
    })
}

#[tauri::command]
pub async fn get_folder_stats(
    folder_path: String,
    include_subdirs: Option<bool>,
) -> std::result::Result<FolderStats, String> {
    let files = scan_folder_for_documents(folder_path.clone(), include_subdirs, Some(10)).await?;

    let mut total_size = 0u64;
    let mut file_types = std::collections::HashMap::new();

    for file_path in &files {
        if let Ok(metadata) = std::fs::metadata(file_path) {
            total_size += metadata.len();
        }

        if let Some(extension) = Path::new(file_path).extension().and_then(|e| e.to_str()) {
            *file_types.entry(extension.to_lowercase()).or_insert(0) += 1;
        }
    }

    Ok(FolderStats {
        folder_path,
        total_files: files.len(),
        total_size,
        file_types,
        sample_files: files.into_iter().take(10).collect(),
    })
}

// Response types
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub mime_type: String,
    pub extension: String,
    pub modified: u64,
    pub is_supported: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IndexedDocumentInfo {
    pub id: String,
    pub path: String,
    pub title: String,
    pub chunks_count: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FailedIndexInfo {
    pub path: String,
    pub error: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IndexingResult {
    pub successful: Vec<IndexedDocumentInfo>,
    pub failed: Vec<FailedIndexInfo>,
    pub total_processed: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FolderStats {
    pub folder_path: String,
    pub total_files: usize,
    pub total_size: u64,
    pub file_types: std::collections::HashMap<String, usize>,
    pub sample_files: Vec<String>,
}
