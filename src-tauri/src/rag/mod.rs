mod embeddings;
mod document_processor;
mod vector_store;
mod chunker;
mod retriever;

pub use embeddings::EmbeddingModel;
pub use document_processor::DocumentProcessor;
pub use vector_store::VectorStore;
pub use chunker::TextChunker;
pub use retriever::DocumentRetriever;

use crate::error::{AppError, Result};
use crate::models::Goal;
use uuid::Uuid;
use std::path::Path;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::database::SqliteDatabase;

#[derive(Debug, Clone)]
pub struct Document {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub file_path: String,
    pub goal_id: Option<Uuid>,
    pub chunks: Vec<DocumentChunk>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct DocumentChunk {
    pub id: Uuid,
    pub document_id: Uuid,
    pub content: String,
    pub embedding: Vec<f32>,
    pub chunk_index: usize,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub document_id: Uuid,
    pub chunk_id: Uuid,
    pub content: String,
    pub score: f32,
    pub metadata: HashMap<String, String>,
}

pub struct RAGSystem {
    embedding_model: EmbeddingModel,
    document_processor: DocumentProcessor,
    vector_store: VectorStore,
    text_chunker: TextChunker,
    retriever: DocumentRetriever,
}

impl RAGSystem {
    pub async fn new() -> Result<Self> {
        let embedding_model = EmbeddingModel::new().await?;
        let document_processor = DocumentProcessor::new();
        let vector_store = VectorStore::new().await?;
        let text_chunker = TextChunker::new();
        let retriever = DocumentRetriever::new(vector_store.clone());

        Ok(Self {
            embedding_model,
            document_processor,
            vector_store,
            text_chunker,
            retriever,
        })
    }

    /// Index a document from file path
    pub async fn index_document(&mut self, file_path: &str, goal_id: Option<Uuid>) -> Result<Document> {
        // Process document
        let processed_doc = self.document_processor.process_file(file_path).await?;
        
        // Create document
        let document = Document {
            id: Uuid::new_v4(),
            title: processed_doc.title,
            content: processed_doc.content.clone(),
            file_path: file_path.to_string(),
            goal_id,
            chunks: Vec::new(),
            created_at: chrono::Utc::now(),
        };

        // Chunk the document
        let chunks = self.text_chunker.chunk_text(&processed_doc.content)?;
        
        // Generate embeddings and create document chunks
        let mut document_chunks = Vec::new();
        for (index, chunk_text) in chunks.iter().enumerate() {
            let embedding = self.embedding_model.embed_text(chunk_text).await?;
            
            let chunk = DocumentChunk {
                id: Uuid::new_v4(),
                document_id: document.id,
                content: chunk_text.clone(),
                embedding,
                chunk_index: index,
                metadata: HashMap::new(),
            };
            
            document_chunks.push(chunk);
        }

        // Store in vector database
        self.vector_store.store_document(&document, &document_chunks).await?;

        let mut final_document = document;
        final_document.chunks = document_chunks;
        
        Ok(final_document)
    }

    /// Search for relevant documents
    pub async fn search(&self, query: &str, goal_id: Option<Uuid>, limit: usize) -> Result<Vec<SearchResult>> {
        // Generate query embedding
        let query_embedding = self.embedding_model.embed_text(query).await?;
        
        // Search in vector store
        let results = self.retriever.search(&query_embedding, goal_id, limit).await?;
        
        Ok(results)
    }

    /// Get document context for a goal
    pub async fn get_goal_context(&self, goal_id: Uuid, limit: usize) -> Result<Vec<SearchResult>> {
        self.retriever.get_goal_documents(goal_id, limit).await
    }

    /// Remove document from index
    pub async fn remove_document(&mut self, document_id: Uuid) -> Result<()> {
        self.vector_store.remove_document(document_id).await
    }

    /// List all indexed documents
    pub async fn list_documents(&self, goal_id: Option<Uuid>) -> Result<Vec<Document>> {
        self.vector_store.list_documents(goal_id).await
    }

    /// Update document index
    pub async fn update_document(&mut self, document_id: Uuid, file_path: &str) -> Result<Document> {
        // Remove old document
        self.remove_document(document_id).await?;
        
        // Get goal_id from old document
        let old_docs = self.vector_store.list_documents(None).await?;
        let goal_id = old_docs.iter()
            .find(|doc| doc.id == document_id)
            .and_then(|doc| doc.goal_id);

        // Re-index updated document
        self.index_document(file_path, goal_id).await
    }
    
    /// Set database for persistence
    pub async fn set_database(&mut self, database: Arc<Mutex<SqliteDatabase>>) {
        self.vector_store.set_database(database);
    }
    
    /// Load documents from database
    pub async fn load_from_database(&mut self) -> Result<()> {
        self.vector_store.load_from_database().await
    }
}