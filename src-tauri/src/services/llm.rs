use crate::error::Result;
use crate::llm::LlmClient;
use crate::models::{ProductivityInsights, ProductivityScore};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use uuid::Uuid;

#[tauri::command]
pub async fn get_productivity_insights(
    llm: State<'_, Arc<LlmClient>>,
    _hours: usize,
) -> Result<ProductivityInsights> {
    // For now, return mock data until database is ready
    let activities = vec![];
    llm.generate_productivity_insights(&activities).await
}

#[tauri::command]
pub async fn get_productivity_score(
    llm: State<'_, Arc<LlmClient>>,
    _hours: usize,
) -> Result<ProductivityScore> {
    // For now, return mock data until database is ready
    let activities = vec![];
    llm.generate_productivity_score(&activities).await
}

#[tauri::command]
pub async fn get_recommendations(
    llm: State<'_, Arc<LlmClient>>,
    _hours: usize,
) -> Result<Vec<String>> {
    // For now, return mock data until database is ready
    let activities = vec![];
    llm.generate_recommendations(&activities).await
}

#[tauri::command]
pub async fn chat_with_documents(
    llm: State<'_, Arc<LlmClient>>,
    rag_system: State<'_, Arc<Mutex<crate::rag::RAGSystemWrapper>>>,
    query: String,
    goal_id: Option<String>,
    limit: Option<usize>,
) -> std::result::Result<ChatResponse, String> {
    let goal_uuid = if let Some(goal_str) = goal_id {
        Some(Uuid::parse_str(&goal_str).map_err(|e| e.to_string())?)
    } else {
        None
    };

    let limit = limit.unwrap_or(5);

    println!("Starting document chat with query: {}", query);
    
    // Search for relevant documents
    let rag = rag_system.lock().await;
    println!("Acquired RAG system lock, searching for documents...");
    let search_results = rag.search(&query, goal_uuid, limit).await
        .map_err(|e| {
            eprintln!("Failed to search documents: {}", e);
            format!("Search failed: {}", e)
        })?;
    
    println!("Found {} search results", search_results.len());

    // Build context from search results
    let mut context = String::new();
    for (i, result) in search_results.iter().enumerate() {
        context.push_str(&format!(
            "Document {}: {}\n\n",
            i + 1,
            result.content
        ));
    }

    // Generate response using LLM with RAG context
    let prompt = if context.is_empty() {
        format!(
            "I don't have any relevant documents to answer your question: \"{}\"\n\n\
            Please provide a helpful response explaining that no relevant documents were found \
            and suggest how the user might get better results (such as indexing more documents or refining their query).",
            query
        )
    } else {
        format!(
            "Based on the following documents, please answer the user's question: \"{}\"\n\n\
            Documents:\n{}\n\n\
            Please provide a comprehensive answer based on the information in these documents. \
            If the documents don't contain enough information to fully answer the question, \
            please mention what information is available and what might be missing.",
            query, context
        )
    };

    println!("Sending prompt to LLM (length: {} chars)", prompt.len());
    let response_text = llm.send_request(&prompt).await
        .map_err(|e| {
            eprintln!("LLM request failed: {}", e);
            format!("LLM error: {}", e)
        })?;
    
    println!("Received LLM response (length: {} chars)", response_text.len());

    Ok(ChatResponse {
        message: response_text,
        sources: search_results.into_iter().map(|r| DocumentSource {
            document_id: r.document_id.to_string(),
            content: r.content,
            score: r.score,
        }).collect(),
        context_used: !context.is_empty(),
    })
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChatResponse {
    pub message: String,
    pub sources: Vec<DocumentSource>,
    pub context_used: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DocumentSource {
    pub document_id: String,
    pub content: String,
    pub score: f32,
}