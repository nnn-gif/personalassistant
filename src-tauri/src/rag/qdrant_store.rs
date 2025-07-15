use crate::database::SqliteDatabase;
use crate::error::{AppError, Result};
use crate::rag::{Document, DocumentChunk, SearchResult};
use qdrant_client::{
    qdrant::{
        points_selector::PointsSelectorOneOf, vectors_config::Config, Condition, CreateCollection,
        Datatype, DeletePoints, Distance, FieldCondition, Filter, Match, PointStruct,
        PointsSelector, ScrollPoints, SearchParams, SearchPoints, UpsertPoints, Value,
        VectorParams, VectorsConfig,
    },
    Qdrant,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

const COLLECTION_NAME: &str = "document_chunks";
const EMBEDDING_SIZE: u64 = 768; // Adjust based on your embedding model

pub struct QdrantVectorStore {
    client: Qdrant,
    database: Option<Arc<Mutex<SqliteDatabase>>>,
}

impl QdrantVectorStore {
    pub async fn new() -> Result<Self> {
        // Try to connect to local Qdrant instance
        let client = Qdrant::from_url("http://localhost:6333")
            .build()
            .map_err(|e| AppError::VectorStore(format!("Failed to connect to Qdrant: {}", e)))?;

        let store = Self {
            client,
            database: None,
        };

        // Initialize collection
        store.initialize_collection().await?;

        Ok(store)
    }

    pub fn set_database(&mut self, database: Arc<Mutex<SqliteDatabase>>) {
        self.database = Some(database);
    }

    async fn initialize_collection(&self) -> Result<()> {
        // Check if collection exists
        let collections = self
            .client
            .list_collections()
            .await
            .map_err(|e| AppError::VectorStore(format!("Failed to list collections: {}", e)))?;

        let collection_exists = collections
            .collections
            .iter()
            .any(|c| c.name == COLLECTION_NAME);

        if !collection_exists {
            // Create collection
            let config = VectorsConfig {
                config: Some(Config::Params(VectorParams {
                    size: EMBEDDING_SIZE,
                    distance: Distance::Cosine as i32,
                    hnsw_config: None,
                    quantization_config: None,
                    on_disk: None,
                    datatype: Some(Datatype::Float32 as i32),
                    multivector_config: None,
                })),
            };

            self.client
                .create_collection(CreateCollection {
                    collection_name: COLLECTION_NAME.to_string(),
                    vectors_config: Some(config),
                    shard_number: Some(1),
                    replication_factor: Some(1),
                    write_consistency_factor: Some(1),
                    on_disk_payload: Some(true),
                    timeout: None,
                    hnsw_config: None,
                    wal_config: None,
                    optimizers_config: None,
                    init_from_collection: None,
                    quantization_config: None,
                    sharding_method: None,
                    sparse_vectors_config: None,
                    strict_mode_config: None,
                })
                .await
                .map_err(|e| {
                    AppError::VectorStore(format!("Failed to create collection: {}", e))
                })?;

            println!("Created Qdrant collection: {}", COLLECTION_NAME);
        } else {
            println!("Qdrant collection {} already exists", COLLECTION_NAME);
        }

        Ok(())
    }

    pub async fn load_from_database(&self) -> Result<()> {
        if let Some(db) = &self.database {
            let database = db.lock().await;
            let documents = database.load_documents(None).await?;

            let mut points = Vec::new();

            for document in documents {
                for chunk in &document.chunks {
                    let point = PointStruct {
                        id: Some(chunk.id.to_string().into()),
                        vectors: Some(chunk.embedding.clone().into()),
                        payload: create_chunk_payload(&document, chunk),
                    };
                    points.push(point);
                }
            }

            if !points.is_empty() {
                let points_len = points.len();
                self.client
                    .upsert_points(UpsertPoints {
                        collection_name: COLLECTION_NAME.to_string(),
                        points,
                        wait: None,
                        ordering: None,
                        shard_key_selector: None,
                    })
                    .await
                    .map_err(|e| {
                        AppError::VectorStore(format!("Failed to load data to Qdrant: {}", e))
                    })?;

                println!("Loaded {} chunks to Qdrant", points_len);
            }
        }

        Ok(())
    }

    pub async fn store_document(
        &self,
        document: &Document,
        chunks: &[DocumentChunk],
    ) -> Result<()> {
        // Save to database first
        if let Some(db) = &self.database {
            let database = db.lock().await;
            database.save_document(document).await?;
            for chunk in chunks {
                database.save_document_chunk(chunk).await?;
            }
        }

        // Store in Qdrant
        let points: Vec<PointStruct> = chunks
            .iter()
            .map(|chunk| PointStruct {
                id: Some(chunk.id.to_string().into()),
                vectors: Some(chunk.embedding.clone().into()),
                payload: create_chunk_payload(document, chunk),
            })
            .collect();

        if !points.is_empty() {
            self.client
                .upsert_points(UpsertPoints {
                    collection_name: COLLECTION_NAME.to_string(),
                    points,
                    wait: None,
                    ordering: None,
                    shard_key_selector: None,
                })
                .await
                .map_err(|e| AppError::VectorStore(format!("Failed to store in Qdrant: {}", e)))?;
        }

        Ok(())
    }

    pub async fn search_similar(
        &self,
        query_embedding: &[f32],
        goal_id: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut filter = None;

        // Add goal filter if specified
        if let Some(goal_id) = goal_id {
            filter = Some(Filter {
                should: vec![],
                must: vec![Condition {
                    condition_one_of: Some(
                        qdrant_client::qdrant::condition::ConditionOneOf::Field(FieldCondition {
                            key: "goal_id".to_string(),
                            r#match: Some(Match {
                                match_value: Some(
                                    qdrant_client::qdrant::r#match::MatchValue::Keyword(
                                        goal_id.to_string(),
                                    ),
                                ),
                            }),
                            range: None,
                            geo_bounding_box: None,
                            geo_radius: None,
                            geo_polygon: None,
                            values_count: None,
                            is_empty: None,
                            is_null: None,
                            datetime_range: None,
                        }),
                    ),
                }],
                must_not: vec![],
                min_should: None,
            });
        }

        let search_result = self
            .client
            .search_points(SearchPoints {
                collection_name: COLLECTION_NAME.to_string(),
                vector: query_embedding.to_vec(),
                filter,
                limit: limit as u64,
                with_payload: Some(true.into()),
                params: Some(SearchParams {
                    hnsw_ef: None,
                    exact: Some(false),
                    quantization: None,
                    indexed_only: Some(false),
                }),
                score_threshold: None,
                offset: None,
                vector_name: None,
                with_vectors: None,
                read_consistency: None,
                timeout: None,
                shard_key_selector: None,
                sparse_indices: None,
            })
            .await
            .map_err(|e| AppError::VectorStore(format!("Failed to search in Qdrant: {}", e)))?;

        let mut results = Vec::new();
        for scored_point in search_result.result {
            let payload = scored_point.payload;
            let search_result = SearchResult {
                document_id: Uuid::parse_str(
                    payload
                        .get("document_id")
                        .map(|v| v.to_string())
                        .as_deref()
                        .unwrap_or(""),
                )
                .unwrap_or_default(),
                chunk_id: Uuid::parse_str(
                    scored_point
                        .id
                        .and_then(|id| match id.point_id_options {
                            Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(uuid)) => {
                                Some(uuid)
                            }
                            Some(qdrant_client::qdrant::point_id::PointIdOptions::Num(_)) => None,
                            None => None,
                        })
                        .map(|id| id.to_string())
                        .as_deref()
                        .unwrap_or(""),
                )
                .unwrap_or_default(),
                content: payload
                    .get("content")
                    .map(|v| v.to_string())
                    .as_deref()
                    .unwrap_or("")
                    .to_string(),
                score: scored_point.score,
                metadata: extract_metadata_from_payload(&payload),
            };
            results.push(search_result);
        }

        Ok(results)
    }

    pub async fn get_goal_documents(
        &self,
        goal_id: Uuid,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let filter = Filter {
            should: vec![],
            must: vec![Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "goal_id".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Keyword(
                                goal_id.to_string(),
                            )),
                        }),
                        range: None,
                        geo_bounding_box: None,
                        geo_radius: None,
                        geo_polygon: None,
                        values_count: None,
                        is_empty: None,
                        is_null: None,
                        datetime_range: None,
                    },
                )),
            }],
            must_not: vec![],
            min_should: None,
        };

        // Use scroll to get all matching points (not vector search)
        let scroll_result = self
            .client
            .scroll(ScrollPoints {
                collection_name: COLLECTION_NAME.to_string(),
                filter: Some(filter),
                offset: None,
                limit: Some(limit as u32),
                with_payload: Some(true.into()),
                with_vectors: None,
                read_consistency: None,
                order_by: None,
                shard_key_selector: None,
                timeout: None,
            })
            .await
            .map_err(|e| AppError::VectorStore(format!("Failed to scroll in Qdrant: {}", e)))?;

        let mut results = Vec::new();
        for point in scroll_result.result {
            let payload = point.payload;
            let search_result = SearchResult {
                document_id: Uuid::parse_str(
                    payload
                        .get("document_id")
                        .map(|v| v.to_string())
                        .as_deref()
                        .unwrap_or(""),
                )
                .unwrap_or_default(),
                chunk_id: Uuid::parse_str(
                    point
                        .id
                        .and_then(|id| match id.point_id_options {
                            Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(uuid)) => {
                                Some(uuid)
                            }
                            Some(qdrant_client::qdrant::point_id::PointIdOptions::Num(_)) => None,
                            None => None,
                        })
                        .map(|id| id.to_string())
                        .as_deref()
                        .unwrap_or(""),
                )
                .unwrap_or_default(),
                content: payload
                    .get("content")
                    .map(|v| v.to_string())
                    .as_deref()
                    .unwrap_or("")
                    .to_string(),
                score: 1.0, // Default score for goal-based retrieval
                metadata: extract_metadata_from_payload(&payload),
            };
            results.push(search_result);
        }

        // Sort by chunk index for consistent ordering
        results.sort_by(|a, b| {
            let a_index = a
                .metadata
                .get("chunk_index")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let b_index = b
                .metadata
                .get("chunk_index")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            a_index.cmp(&b_index)
        });

        Ok(results)
    }

    pub async fn remove_document(&self, document_id: Uuid) -> Result<()> {
        // Delete from database first
        if let Some(db) = &self.database {
            let database = db.lock().await;
            database.delete_document(document_id).await?;
        }

        // Remove from Qdrant using filter
        let filter = Filter {
            should: vec![],
            must: vec![Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "document_id".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Keyword(
                                document_id.to_string(),
                            )),
                        }),
                        range: None,
                        geo_bounding_box: None,
                        geo_radius: None,
                        geo_polygon: None,
                        values_count: None,
                        is_empty: None,
                        is_null: None,
                        datetime_range: None,
                    },
                )),
            }],
            must_not: vec![],
            min_should: None,
        };

        self.client
            .delete_points(DeletePoints {
                collection_name: COLLECTION_NAME.to_string(),
                points: Some(PointsSelector {
                    points_selector_one_of: Some(PointsSelectorOneOf::Filter(filter)),
                }),
                wait: None,
                ordering: None,
                shard_key_selector: None,
            })
            .await
            .map_err(|e| AppError::VectorStore(format!("Failed to delete from Qdrant: {}", e)))?;

        Ok(())
    }

    pub async fn list_documents(&self, goal_id: Option<Uuid>) -> Result<Vec<Document>> {
        // This operation is more efficient with the SQLite database
        if let Some(db) = &self.database {
            let database = db.lock().await;
            database.load_documents(goal_id).await
        } else {
            Err(AppError::VectorStore(
                "Database not available for listing documents".to_string(),
            ))
        }
    }

    pub async fn get_document(&self, document_id: Uuid) -> Result<Option<Document>> {
        if let Some(db) = &self.database {
            let database = db.lock().await;
            let documents = database.load_documents(None).await?;
            Ok(documents.into_iter().find(|d| d.id == document_id))
        } else {
            Err(AppError::VectorStore(
                "Database not available for getting document".to_string(),
            ))
        }
    }

    pub async fn get_document_chunks(&self, document_id: Uuid) -> Result<Vec<DocumentChunk>> {
        if let Some(db) = &self.database {
            let database = db.lock().await;
            let documents = database.load_documents(None).await?;
            if let Some(document) = documents.into_iter().find(|d| d.id == document_id) {
                Ok(document.chunks)
            } else {
                Ok(Vec::new())
            }
        } else {
            Err(AppError::VectorStore(
                "Database not available for getting chunks".to_string(),
            ))
        }
    }
}

impl Clone for QdrantVectorStore {
    fn clone(&self) -> Self {
        // Qdrant client doesn't implement Clone, so we create a new connection
        let client = Qdrant::from_url("http://localhost:6333")
            .build()
            .expect("Failed to clone Qdrant client");

        Self {
            client,
            database: self.database.clone(),
        }
    }
}

fn create_chunk_payload(document: &Document, chunk: &DocumentChunk) -> HashMap<String, Value> {
    let mut payload = HashMap::new();

    payload.insert(
        "document_id".to_string(),
        Value::from(document.id.to_string()),
    );
    payload.insert("content".to_string(), Value::from(chunk.content.clone()));
    payload.insert(
        "chunk_index".to_string(),
        Value::from(chunk.chunk_index as i64),
    );
    payload.insert(
        "document_title".to_string(),
        Value::from(document.title.clone()),
    );
    payload.insert(
        "file_path".to_string(),
        Value::from(document.file_path.clone()),
    );

    if let Some(goal_id) = document.goal_id {
        payload.insert("goal_id".to_string(), Value::from(goal_id.to_string()));
    }

    // Add chunk metadata
    for (key, value) in &chunk.metadata {
        payload.insert(format!("meta_{}", key), Value::from(value.clone()));
    }

    payload
}

fn extract_metadata_from_payload(payload: &HashMap<String, Value>) -> HashMap<String, String> {
    let mut metadata = HashMap::new();

    for (key, value) in payload {
        if key.starts_with("meta_") {
            let meta_key = key.strip_prefix("meta_").unwrap_or(key);
            metadata.insert(meta_key.to_string(), value.to_string());
        } else if key == "chunk_index" {
            metadata.insert(key.clone(), value.to_string());
        }
    }

    metadata
}
