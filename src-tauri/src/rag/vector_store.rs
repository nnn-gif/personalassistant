use crate::error::{AppError, Result};
use crate::rag::{Document, DocumentChunk, SearchResult};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// In-memory vector store for now
// In production, you'd use a proper vector database like Qdrant or LanceDB
pub struct VectorStore {
    documents: Arc<RwLock<HashMap<Uuid, Document>>>,
    chunks: Arc<RwLock<HashMap<Uuid, DocumentChunk>>>,
    goal_index: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>, // goal_id -> document_ids
}

impl VectorStore {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            chunks: Arc::new(RwLock::new(HashMap::new())),
            goal_index: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn store_document(&self, document: &Document, chunks: &[DocumentChunk]) -> Result<()> {
        let mut documents = self.documents.write().await;
        let mut chunks_store = self.chunks.write().await;
        let mut goal_index = self.goal_index.write().await;

        // Store document
        documents.insert(document.id, document.clone());

        // Store chunks
        for chunk in chunks {
            chunks_store.insert(chunk.id, chunk.clone());
        }

        // Update goal index
        if let Some(goal_id) = document.goal_id {
            goal_index.entry(goal_id)
                .or_insert_with(Vec::new)
                .push(document.id);
        }

        Ok(())
    }

    pub async fn search_similar(&self, query_embedding: &[f32], goal_id: Option<Uuid>, limit: usize) -> Result<Vec<SearchResult>> {
        let chunks = self.chunks.read().await;
        let documents = self.documents.read().await;
        
        let mut results = Vec::new();

        for chunk in chunks.values() {
            // Filter by goal if specified
            if let Some(goal_id) = goal_id {
                if let Some(doc) = documents.get(&chunk.document_id) {
                    if doc.goal_id != Some(goal_id) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Calculate similarity (cosine similarity)
            let similarity = self.cosine_similarity(query_embedding, &chunk.embedding);
            
            let result = SearchResult {
                document_id: chunk.document_id,
                chunk_id: chunk.id,
                content: chunk.content.clone(),
                score: similarity,
                metadata: chunk.metadata.clone(),
            };

            results.push(result);
        }

        // Sort by similarity score (descending)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Return top results
        results.truncate(limit);
        Ok(results)
    }

    pub async fn get_goal_documents(&self, goal_id: Uuid, limit: usize) -> Result<Vec<SearchResult>> {
        let documents = self.documents.read().await;
        let chunks = self.chunks.read().await;
        
        let mut results = Vec::new();

        for document in documents.values() {
            if document.goal_id == Some(goal_id) {
                // Get chunks for this document
                for chunk in chunks.values() {
                    if chunk.document_id == document.id {
                        let result = SearchResult {
                            document_id: chunk.document_id,
                            chunk_id: chunk.id,
                            content: chunk.content.clone(),
                            score: 1.0, // Default score for goal-based retrieval
                            metadata: chunk.metadata.clone(),
                        };
                        results.push(result);
                    }
                }
            }
        }

        // Sort by chunk index for consistent ordering
        results.sort_by(|a, b| {
            let a_index = a.metadata.get("chunk_index")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let b_index = b.metadata.get("chunk_index")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            a_index.cmp(&b_index)
        });

        results.truncate(limit);
        Ok(results)
    }

    pub async fn remove_document(&self, document_id: Uuid) -> Result<()> {
        let mut documents = self.documents.write().await;
        let mut chunks = self.chunks.write().await;
        let mut goal_index = self.goal_index.write().await;

        // Remove document
        if let Some(document) = documents.remove(&document_id) {
            // Remove from goal index
            if let Some(goal_id) = document.goal_id {
                if let Some(doc_ids) = goal_index.get_mut(&goal_id) {
                    doc_ids.retain(|&id| id != document_id);
                    if doc_ids.is_empty() {
                        goal_index.remove(&goal_id);
                    }
                }
            }
        }

        // Remove all chunks for this document
        chunks.retain(|_, chunk| chunk.document_id != document_id);

        Ok(())
    }

    pub async fn list_documents(&self, goal_id: Option<Uuid>) -> Result<Vec<Document>> {
        let documents = self.documents.read().await;
        
        let mut result = Vec::new();
        
        for document in documents.values() {
            if let Some(goal_id) = goal_id {
                if document.goal_id == Some(goal_id) {
                    result.push(document.clone());
                }
            } else {
                result.push(document.clone());
            }
        }

        // Sort by creation time (newest first)
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(result)
    }

    pub async fn get_document(&self, document_id: Uuid) -> Result<Option<Document>> {
        let documents = self.documents.read().await;
        Ok(documents.get(&document_id).cloned())
    }

    pub async fn get_document_chunks(&self, document_id: Uuid) -> Result<Vec<DocumentChunk>> {
        let chunks = self.chunks.read().await;
        
        let mut result = Vec::new();
        for chunk in chunks.values() {
            if chunk.document_id == document_id {
                result.push(chunk.clone());
            }
        }

        // Sort by chunk index
        result.sort_by_key(|chunk| chunk.chunk_index);

        Ok(result)
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }
}

impl Clone for VectorStore {
    fn clone(&self) -> Self {
        Self {
            documents: Arc::clone(&self.documents),
            chunks: Arc::clone(&self.chunks),
            goal_index: Arc::clone(&self.goal_index),
        }
    }
}