use crate::error::{AppError, Result};
use crate::rag::{SearchResult, VectorStore};
use uuid::Uuid;

pub struct DocumentRetriever {
    vector_store: VectorStore,
}

impl DocumentRetriever {
    pub fn new(vector_store: VectorStore) -> Self {
        Self { vector_store }
    }

    pub async fn search(&self, query_embedding: &[f32], goal_id: Option<Uuid>, limit: usize) -> Result<Vec<SearchResult>> {
        self.vector_store.search_similar(query_embedding, goal_id, limit).await
    }

    pub async fn get_goal_documents(&self, goal_id: Uuid, limit: usize) -> Result<Vec<SearchResult>> {
        self.vector_store.get_goal_documents(goal_id, limit).await
    }

    pub async fn search_with_filters(&self, query_embedding: &[f32], filters: SearchFilters, limit: usize) -> Result<Vec<SearchResult>> {
        let mut results = self.vector_store.search_similar(query_embedding, filters.goal_id, limit * 2).await?;

        // Apply additional filters
        if let Some(content_type) = &filters.content_type {
            results.retain(|result| {
                result.metadata.get("content_type")
                    .map(|ct| ct == content_type)
                    .unwrap_or(false)
            });
        }

        if let Some(min_score) = filters.min_score {
            results.retain(|result| result.score >= min_score);
        }

        if let Some(file_types) = &filters.file_types {
            results.retain(|result| {
                result.metadata.get("file_type")
                    .map(|ft| file_types.contains(ft))
                    .unwrap_or(false)
            });
        }

        results.truncate(limit);
        Ok(results)
    }

    pub async fn get_document_context(&self, document_id: Uuid, chunk_id: Uuid, context_size: usize) -> Result<Vec<SearchResult>> {
        let chunks = self.vector_store.get_document_chunks(document_id).await?;
        
        // Find the target chunk
        let target_chunk_index = chunks.iter()
            .position(|chunk| chunk.id == chunk_id)
            .ok_or_else(|| AppError::NotFound("Chunk not found".to_string()))?;

        // Get surrounding chunks
        let start_index = target_chunk_index.saturating_sub(context_size / 2);
        let end_index = std::cmp::min(target_chunk_index + context_size / 2 + 1, chunks.len());

        let mut results = Vec::new();
        for i in start_index..end_index {
            if let Some(chunk) = chunks.get(i) {
                let result = SearchResult {
                    document_id: chunk.document_id,
                    chunk_id: chunk.id,
                    content: chunk.content.clone(),
                    score: if i == target_chunk_index { 1.0 } else { 0.8 },
                    metadata: chunk.metadata.clone(),
                };
                results.push(result);
            }
        }

        Ok(results)
    }

    pub async fn hybrid_search(&self, query_embedding: &[f32], keywords: &[String], goal_id: Option<Uuid>, limit: usize) -> Result<Vec<SearchResult>> {
        // Vector search
        let vector_results = self.vector_store.search_similar(query_embedding, goal_id, limit).await?;
        
        // Keyword search
        let keyword_results = self.keyword_search(keywords, goal_id, limit).await?;
        
        // Combine and re-rank results
        let combined_results = self.combine_search_results(vector_results, keyword_results, limit);
        
        Ok(combined_results)
    }

    async fn keyword_search(&self, keywords: &[String], goal_id: Option<Uuid>, limit: usize) -> Result<Vec<SearchResult>> {
        let documents = self.vector_store.list_documents(goal_id).await?;
        let mut results = Vec::new();

        for document in documents {
            let chunks = self.vector_store.get_document_chunks(document.id).await?;
            
            for chunk in chunks {
                let mut score = 0.0;
                let content_lower = chunk.content.to_lowercase();
                
                for keyword in keywords {
                    let keyword_lower = keyword.to_lowercase();
                    let matches = content_lower.matches(&keyword_lower).count();
                    score += matches as f32 * 0.1; // Simple scoring
                }

                if score > 0.0 {
                    let result = SearchResult {
                        document_id: chunk.document_id,
                        chunk_id: chunk.id,
                        content: chunk.content.clone(),
                        score,
                        metadata: chunk.metadata.clone(),
                    };
                    results.push(result);
                }
            }
        }

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    fn combine_search_results(&self, vector_results: Vec<SearchResult>, keyword_results: Vec<SearchResult>, limit: usize) -> Vec<SearchResult> {
        let mut combined = std::collections::HashMap::new();

        // Add vector results
        for result in vector_results {
            combined.insert(result.chunk_id, result);
        }

        // Add keyword results, combining scores if chunk already exists
        for result in keyword_results {
            if let Some(existing) = combined.get_mut(&result.chunk_id) {
                existing.score = (existing.score + result.score) / 2.0; // Average the scores
            } else {
                combined.insert(result.chunk_id, result);
            }
        }

        // Convert to vector and sort
        let mut results: Vec<SearchResult> = combined.into_values().collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        results
    }
}

#[derive(Debug, Clone)]
pub struct SearchFilters {
    pub goal_id: Option<Uuid>,
    pub content_type: Option<String>,
    pub file_types: Option<Vec<String>>,
    pub min_score: Option<f32>,
    pub date_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
}