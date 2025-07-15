use crate::error::{AppError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OllamaEmbedRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
struct OllamaEmbedResponse {
    embedding: Vec<f32>,
}

pub struct EmbeddingModel {
    client: Client,
    model_name: String,
    ollama_url: String,
}

impl EmbeddingModel {
    pub async fn new() -> Result<Self> {
        let client = Client::new();
        let model_name = "nomic-embed-text:latest".to_string();
        let ollama_url = "http://localhost:11434".to_string();

        // Test connection to Ollama
        let test_url = format!("{}/api/tags", ollama_url);
        match client.get(&test_url).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    eprintln!("Warning: Ollama may not be running at {}", ollama_url);
                }
            }
            Err(_) => {
                eprintln!(
                    "Warning: Cannot connect to Ollama at {}. Make sure Ollama is running.",
                    ollama_url
                );
            }
        }

        Ok(Self {
            client,
            model_name,
            ollama_url,
        })
    }

    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        // Clean and prepare text
        let cleaned_text = text.trim();
        if cleaned_text.is_empty() {
            return Ok(vec![0.0; 768]); // nomic-embed-text has 768 dimensions
        }

        // Truncate very long text to avoid API limits
        let truncated_text = if cleaned_text.len() > 8000 {
            &cleaned_text[..8000]
        } else {
            cleaned_text
        };

        let request = OllamaEmbedRequest {
            model: self.model_name.clone(),
            prompt: truncated_text.to_string(),
        };

        let url = format!("{}/api/embeddings", self.ollama_url);

        match self.client.post(&url).json(&request).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<OllamaEmbedResponse>().await {
                        Ok(embed_response) => {
                            if embed_response.embedding.is_empty() {
                                eprintln!(
                                    "Warning: Ollama returned empty embedding for text: {}",
                                    &truncated_text[..50.min(truncated_text.len())]
                                );
                                Ok(self.create_fallback_embedding(truncated_text))
                            } else {
                                Ok(embed_response.embedding)
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to parse Ollama embedding response: {}", e);
                            Ok(self.create_fallback_embedding(truncated_text))
                        }
                    }
                } else {
                    eprintln!(
                        "Ollama embedding request failed with status: {}",
                        response.status()
                    );
                    Ok(self.create_fallback_embedding(truncated_text))
                }
            }
            Err(e) => {
                eprintln!("Failed to connect to Ollama for embeddings: {}", e);
                Ok(self.create_fallback_embedding(truncated_text))
            }
        }
    }

    fn create_fallback_embedding(&self, text: &str) -> Vec<f32> {
        // Fallback embedding when Ollama is not available
        // This is better than the old hash-based approach but still not ideal
        let mut embedding = vec![0.0; 768]; // Match nomic-embed-text dimensions

        // Basic text statistics
        let char_count = text.chars().count() as f32;
        let word_count = text.split_whitespace().count() as f32;
        let sentence_count = text.split(&['.', '!', '?']).count() as f32;

        // Normalize and set basic features
        embedding[0] = (char_count / 1000.0).min(1.0);
        embedding[1] = (word_count / 100.0).min(1.0);
        embedding[2] = (sentence_count / 10.0).min(1.0);

        // Word frequency features
        let words: Vec<&str> = text.split_whitespace().collect();
        let unique_words: std::collections::HashSet<&str> = words.iter().cloned().collect();
        embedding[3] = (unique_words.len() as f32 / word_count.max(1.0)).min(1.0);

        // Simple keyword presence (better than the old approach)
        let keywords = [
            "document",
            "file",
            "report",
            "data",
            "information",
            "content",
            "business",
            "license",
            "permit",
            "form",
            "application",
            "request",
            "date",
            "time",
            "location",
            "address",
            "name",
            "number",
            "id",
            "meeting",
            "schedule",
            "appointment",
            "reservation",
            "booking",
        ];

        let text_lower = text.to_lowercase();
        for (i, keyword) in keywords.iter().enumerate() {
            if text_lower.contains(keyword) {
                embedding[10 + i] = 1.0;
            }
        }

        // Fill remaining dimensions with normalized text hash
        let text_bytes = text.as_bytes();
        for i in 50..embedding.len() {
            let idx = i % text_bytes.len();
            embedding[i] = (text_bytes[idx] as f32) / 255.0;
        }

        embedding
    }
}

impl Clone for EmbeddingModel {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            model_name: self.model_name.clone(),
            ollama_url: self.ollama_url.clone(),
        }
    }
}
