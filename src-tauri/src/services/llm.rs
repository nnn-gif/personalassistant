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
    activity_tracker: State<'_, Arc<Mutex<crate::activity_tracking::ActivityTracker>>>,
    query: String,
    goal_id: Option<String>,
    limit: Option<usize>,
    model: Option<String>,
) -> std::result::Result<ChatResponse, String> {
    let goal_uuid = if let Some(goal_str) = goal_id {
        Some(Uuid::parse_str(&goal_str).map_err(|e| e.to_string())?)
    } else {
        None
    };

    let limit = limit.unwrap_or(5);

    println!("Starting document chat with query: {}", query);
    
    // Get recent activity context
    let activity_context = {
        let tracker = activity_tracker.lock().await;
        tracker.get_recent_activities(10)
    };
    
    // Search for relevant documents
    let rag = rag_system.lock().await;
    println!("Acquired RAG system lock, searching for documents...");
    let search_results = rag.search(&query, goal_uuid, limit).await
        .map_err(|e| {
            eprintln!("Failed to search documents: {}", e);
            format!("Search failed: {}", e)
        })?;
    
    println!("Found {} search results", search_results.len());
    
    // Log search results for debugging
    for (i, result) in search_results.iter().enumerate() {
        println!("Search Result {}: (score: {:.3})", i + 1, result.score);
        println!("Document ID: {}", result.document_id);
        println!("Content: {}", result.content.chars().take(200).collect::<String>());
        if result.content.len() > 200 {
            println!("... (truncated, full length: {} chars)", result.content.len());
        }
        println!("---");
    }

    // Build context from search results and recent activity
    let mut context = String::new();
    
    // Add document context
    if !search_results.is_empty() {
        context.push_str("=== DOCUMENT CONTEXT ===\n");
        for (i, result) in search_results.iter().enumerate() {
            context.push_str(&format!(
                "--- Document {} ---\n{}\n\n",
                i + 1,
                result.content
            ));
        }
    }
    
    // Add activity context
    if !activity_context.is_empty() {
        context.push_str("=== RECENT ACTIVITY CONTEXT ===\n");
        context.push_str("Your recent activities (last 10 activities):\n");
        for (i, activity) in activity_context.iter().enumerate() {
            let duration_str = if activity.duration_seconds >= 60 {
                format!("{}m{}s", activity.duration_seconds / 60, activity.duration_seconds % 60)
            } else {
                format!("{}s", activity.duration_seconds)
            };
            
            context.push_str(&format!(
                "{}. {} - {} ({}) - Duration: {}\n",
                i + 1,
                activity.timestamp.format("%H:%M"),
                activity.app_usage.app_name,
                activity.app_usage.window_title,
                duration_str
            ));
        }
        context.push_str("\n");
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
            "You are a local personal assistant running on the user's own device. \
            The user has indexed their personal documents into your local knowledge base and you have access to their activity data. \
            Answer the user's question using the available context: \"{}\"\n\n\
            Available Context:\n{}\n\n\
            This information includes the user's personal documents and recent activity data stored locally on their device. \
            You are running locally and have full access to help the user with their own information. \
            Use both document content and activity context to provide a comprehensive and helpful answer. \
            When referencing activities, be specific about apps, times, and durations when relevant.",
            query, context
        )
    };

    println!("Sending prompt to LLM (length: {} chars)", prompt.len());
    let response_text = if let Some(model) = model {
        llm.send_request_with_model(&prompt, &model).await
    } else {
        llm.send_request(&prompt).await
    }.map_err(|e| {
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

#[tauri::command]
pub async fn get_available_models(
    llm: State<'_, Arc<LlmClient>>,
) -> std::result::Result<Vec<String>, String> {
    match llm.get_available_models().await {
        Ok(models) => Ok(models),
        Err(e) => Err(format!("Failed to get available models: {}", e)),
    }
}