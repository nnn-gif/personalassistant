use crate::error::{AppError, Result};
use crate::rag::{Document, DocumentChunk};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use uuid::Uuid;

pub async fn save_document(pool: &SqlitePool, document: &Document) -> Result<()> {
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
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to save document: {}", e)))?;

    Ok(())
}

pub async fn save_document_chunk(pool: &SqlitePool, chunk: &DocumentChunk) -> Result<()> {
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
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to save document chunk: {}", e)))?;

    Ok(())
}

pub async fn load_documents(pool: &SqlitePool, goal_id: Option<Uuid>) -> Result<Vec<Document>> {
    let rows = if let Some(goal_id) = goal_id {
        sqlx::query("SELECT id, title, content, file_path, goal_id, created_at FROM documents WHERE goal_id = ?")
            .bind(goal_id.to_string())
            .fetch_all(pool)
            .await
    } else {
        sqlx::query("SELECT id, title, content, file_path, goal_id, created_at FROM documents")
            .fetch_all(pool)
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
        let chunks = load_document_chunks(pool, document_id).await?;

        let document = Document {
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
    pool: &SqlitePool,
    document_id: Uuid,
) -> Result<Vec<DocumentChunk>> {
    let rows = sqlx::query("SELECT id, content, embedding, chunk_index, metadata FROM document_chunks WHERE document_id = ? ORDER BY chunk_index")
        .bind(document_id.to_string())
        .fetch_all(pool)
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
        let metadata: HashMap<String, String> =
            serde_json::from_str(&metadata_json).map_err(|e| {
                AppError::Database(format!("Failed to deserialize metadata: {}", e))
            })?;

        let chunk = DocumentChunk {
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

pub async fn delete_document(pool: &SqlitePool, document_id: Uuid) -> Result<()> {
    // Delete chunks first (due to foreign key constraint)
    sqlx::query("DELETE FROM document_chunks WHERE document_id = ?")
        .bind(document_id.to_string())
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to delete document chunks: {}", e)))?;

    // Delete document
    sqlx::query("DELETE FROM documents WHERE id = ?")
        .bind(document_id.to_string())
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to delete document: {}", e)))?;

    Ok(())
}