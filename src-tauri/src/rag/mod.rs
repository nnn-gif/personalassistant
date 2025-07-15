mod chunker;
mod document_processor;
mod embeddings;
mod enhanced_document_processor;
mod qdrant_store;
mod retriever;
mod vector_store;

pub use chunker::TextChunker;
pub use document_processor::DocumentProcessor;
pub use embeddings::EmbeddingModel;
pub use enhanced_document_processor::EnhancedDocumentProcessor;
pub use qdrant_store::QdrantVectorStore;
pub use retriever::DocumentRetriever;
pub use vector_store::VectorStore;

use crate::database::SqliteDatabase;
use crate::error::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

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
    document_processor: EnhancedDocumentProcessor,
    vector_store: QdrantVectorStore,
    text_chunker: TextChunker,
}

// Keep the old VectorStore as a fallback
pub struct LegacyRAGSystem {
    embedding_model: EmbeddingModel,
    document_processor: EnhancedDocumentProcessor,
    vector_store: VectorStore,
    text_chunker: TextChunker,
    retriever: DocumentRetriever,
}

// Wrapper enum to handle both Qdrant and legacy systems
pub enum RAGSystemWrapper {
    Qdrant(RAGSystem),
    Legacy(LegacyRAGSystem),
}

impl RAGSystem {
    pub async fn new() -> Result<Self> {
        let embedding_model = EmbeddingModel::new().await?;
        let document_processor = EnhancedDocumentProcessor::new();

        // Try to create Qdrant store, fallback to VectorStore if failed
        let vector_store = match QdrantVectorStore::new().await {
            Ok(store) => {
                println!("Successfully connected to Qdrant vector database");
                store
            }
            Err(e) => {
                eprintln!("Failed to connect to Qdrant: {}. Please ensure Qdrant is running on localhost:6333", e);
                return Err(e);
            }
        };

        let text_chunker = TextChunker::new();

        Ok(Self {
            embedding_model,
            document_processor,
            vector_store,
            text_chunker,
        })
    }

    pub async fn new_with_automatic_fallback() -> Result<RAGSystemWrapper> {
        let embedding_model = EmbeddingModel::new().await?;
        let document_processor = EnhancedDocumentProcessor::new();

        // Try Qdrant first, fallback to VectorStore if failed
        match QdrantVectorStore::new().await {
            Ok(qdrant_store) => {
                println!("Successfully connected to Qdrant vector database");
                let text_chunker = TextChunker::new();
                Ok(RAGSystemWrapper::Qdrant(Self {
                    embedding_model,
                    document_processor,
                    vector_store: qdrant_store,
                    text_chunker,
                }))
            }
            Err(e) => {
                eprintln!(
                    "Failed to connect to Qdrant: {}. Falling back to in-memory vector store",
                    e
                );
                let vector_store = VectorStore::new().await?;
                let text_chunker = TextChunker::new();
                let retriever = DocumentRetriever::new(vector_store.clone());
                Ok(RAGSystemWrapper::Legacy(LegacyRAGSystem {
                    embedding_model,
                    document_processor,
                    vector_store,
                    text_chunker,
                    retriever,
                }))
            }
        }
    }

    pub async fn new_with_fallback() -> Result<LegacyRAGSystem> {
        let embedding_model = EmbeddingModel::new().await?;
        let document_processor = EnhancedDocumentProcessor::new();
        let vector_store = VectorStore::new().await?;
        let text_chunker = TextChunker::new();
        let retriever = DocumentRetriever::new(vector_store.clone());

        Ok(LegacyRAGSystem {
            embedding_model,
            document_processor,
            vector_store,
            text_chunker,
            retriever,
        })
    }

    /// Index a document from file path
    pub async fn index_document(
        &mut self,
        file_path: &str,
        goal_id: Option<Uuid>,
    ) -> Result<Document> {
        println!("🚀 Starting document indexing for: {}", file_path);

        // Process document
        println!("📄 Processing document content...");
        let processed_doc = self.document_processor.process_file(file_path).await?;

        // Create document
        println!("📋 Creating document structure...");
        let document = Document {
            id: Uuid::new_v4(),
            title: processed_doc.title.clone(),
            content: processed_doc.content.clone(),
            file_path: file_path.to_string(),
            goal_id,
            chunks: Vec::new(),
            created_at: chrono::Utc::now(),
        };
        println!("✅ Document created with ID: {}", document.id);

        // Chunk the document
        println!("✂️  Chunking document content...");
        let chunks = self.text_chunker.chunk_text(&processed_doc.content)?;
        println!("✅ Document chunked into {} pieces", chunks.len());

        // Generate embeddings and create document chunks
        println!("🧠 Generating embeddings for {} chunks...", chunks.len());
        let mut document_chunks = Vec::new();
        for (index, chunk_text) in chunks.iter().enumerate() {
            println!(
                "   Processing chunk {}/{} ({} characters)",
                index + 1,
                chunks.len(),
                chunk_text.len()
            );
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
        println!("✅ All embeddings generated successfully");

        // Store in vector database
        println!("💾 Storing document in vector database...");
        self.vector_store
            .store_document(&document, &document_chunks)
            .await?;
        println!("✅ Document stored in vector database");

        let mut final_document = document;
        final_document.chunks = document_chunks;

        println!("🎉 Document indexing completed successfully!");
        println!("📊 Final statistics:");
        println!("   - Document ID: {}", final_document.id);
        println!("   - Title: {}", final_document.title);
        println!(
            "   - Content length: {} characters",
            final_document.content.len()
        );
        println!("   - Number of chunks: {}", final_document.chunks.len());
        println!("   - Goal ID: {:?}", final_document.goal_id);

        Ok(final_document)
    }

    /// Search for relevant documents
    pub async fn search(
        &self,
        query: &str,
        goal_id: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        // Generate query embedding
        let query_embedding = self.embedding_model.embed_text(query).await?;

        // Search in Qdrant vector store
        self.vector_store
            .search_similar(&query_embedding, goal_id, limit)
            .await
    }

    /// Get document context for a goal
    pub async fn get_goal_context(&self, goal_id: Uuid, limit: usize) -> Result<Vec<SearchResult>> {
        self.vector_store.get_goal_documents(goal_id, limit).await
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
    pub async fn update_document(
        &mut self,
        document_id: Uuid,
        file_path: &str,
    ) -> Result<Document> {
        // Remove old document
        self.remove_document(document_id).await?;

        // Get goal_id from old document
        let old_docs = self.vector_store.list_documents(None).await?;
        let goal_id = old_docs
            .iter()
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

impl RAGSystemWrapper {
    /// Index a document from file path
    pub async fn index_document(
        &mut self,
        file_path: &str,
        goal_id: Option<Uuid>,
    ) -> Result<Document> {
        match self {
            RAGSystemWrapper::Qdrant(rag) => rag.index_document(file_path, goal_id).await,
            RAGSystemWrapper::Legacy(rag) => rag.index_document(file_path, goal_id).await,
        }
    }

    /// Search for relevant documents
    pub async fn search(
        &self,
        query: &str,
        goal_id: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        match self {
            RAGSystemWrapper::Qdrant(rag) => rag.search(query, goal_id, limit).await,
            RAGSystemWrapper::Legacy(rag) => rag.search(query, goal_id, limit).await,
        }
    }

    /// Get document context for a goal
    pub async fn get_goal_context(&self, goal_id: Uuid, limit: usize) -> Result<Vec<SearchResult>> {
        match self {
            RAGSystemWrapper::Qdrant(rag) => rag.get_goal_context(goal_id, limit).await,
            RAGSystemWrapper::Legacy(rag) => rag.get_goal_context(goal_id, limit).await,
        }
    }

    /// Remove document from index
    pub async fn remove_document(&mut self, document_id: Uuid) -> Result<()> {
        match self {
            RAGSystemWrapper::Qdrant(rag) => rag.remove_document(document_id).await,
            RAGSystemWrapper::Legacy(rag) => rag.remove_document(document_id).await,
        }
    }

    /// List all indexed documents
    pub async fn list_documents(&self, goal_id: Option<Uuid>) -> Result<Vec<Document>> {
        match self {
            RAGSystemWrapper::Qdrant(rag) => rag.list_documents(goal_id).await,
            RAGSystemWrapper::Legacy(rag) => rag.list_documents(goal_id).await,
        }
    }

    /// Update document index
    pub async fn update_document(
        &mut self,
        document_id: Uuid,
        file_path: &str,
    ) -> Result<Document> {
        match self {
            RAGSystemWrapper::Qdrant(rag) => rag.update_document(document_id, file_path).await,
            RAGSystemWrapper::Legacy(rag) => rag.update_document(document_id, file_path).await,
        }
    }

    /// Set database for persistence
    pub async fn set_database(&mut self, database: Arc<Mutex<SqliteDatabase>>) {
        match self {
            RAGSystemWrapper::Qdrant(rag) => rag.set_database(database).await,
            RAGSystemWrapper::Legacy(rag) => rag.set_database(database).await,
        }
    }

    /// Load documents from database
    pub async fn load_from_database(&mut self) -> Result<()> {
        match self {
            RAGSystemWrapper::Qdrant(rag) => rag.load_from_database().await,
            RAGSystemWrapper::Legacy(rag) => rag.load_from_database().await,
        }
    }
}

impl LegacyRAGSystem {
    /// Index a document from file path
    pub async fn index_document(
        &mut self,
        file_path: &str,
        goal_id: Option<Uuid>,
    ) -> Result<Document> {
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
        self.vector_store
            .store_document(&document, &document_chunks)
            .await?;

        let mut final_document = document;
        final_document.chunks = document_chunks;

        Ok(final_document)
    }

    /// Search for relevant documents
    pub async fn search(
        &self,
        query: &str,
        goal_id: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        // Generate query embedding
        let query_embedding = self.embedding_model.embed_text(query).await?;

        // Search in vector store
        let results = self
            .retriever
            .search(&query_embedding, goal_id, limit)
            .await?;

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
    pub async fn update_document(
        &mut self,
        document_id: Uuid,
        file_path: &str,
    ) -> Result<Document> {
        // Remove old document
        self.remove_document(document_id).await?;

        // Get goal_id from old document
        let old_docs = self.vector_store.list_documents(None).await?;
        let goal_id = old_docs
            .iter()
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
