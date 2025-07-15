use crate::error::Result;
use crate::rag::EmbeddingModel;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tauri::command]
pub async fn test_embeddings(text: String) -> std::result::Result<TestEmbeddingResponse, String> {
    let embedding_model = EmbeddingModel::new().await.map_err(|e| e.to_string())?;

    let embedding = embedding_model
        .embed_text(&text)
        .await
        .map_err(|e| e.to_string())?;

    Ok(TestEmbeddingResponse {
        text,
        embedding_length: embedding.len(),
        first_few_values: embedding.iter().take(5).cloned().collect(),
        is_zero_vector: embedding.iter().all(|&x| x == 0.0),
    })
}

#[derive(serde::Serialize)]
pub struct TestEmbeddingResponse {
    pub text: String,
    pub embedding_length: usize,
    pub first_few_values: Vec<f32>,
    pub is_zero_vector: bool,
}
