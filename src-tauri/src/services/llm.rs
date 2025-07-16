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
    activity_tracker: State<'_, Arc<Mutex<crate::activity_tracking::ActivityTracker>>>,
    hours: usize,
) -> Result<ProductivityInsights> {
    // Get recent activities from the tracker
    let activities = {
        let tracker = activity_tracker.lock().await;
        tracker.get_recent_activities(hours * 20) // Estimate 20 activities per hour
    };

    println!(
        "Generating productivity insights from {} activities",
        activities.len()
    );
    llm.generate_productivity_insights(&activities).await
}

#[tauri::command]
pub async fn get_productivity_score(
    llm: State<'_, Arc<LlmClient>>,
    activity_tracker: State<'_, Arc<Mutex<crate::activity_tracking::ActivityTracker>>>,
    hours: usize,
) -> Result<ProductivityScore> {
    // Get recent activities from the tracker
    let activities = {
        let tracker = activity_tracker.lock().await;
        tracker.get_recent_activities(hours * 20) // Estimate 20 activities per hour
    };

    println!(
        "Generating productivity score from {} activities",
        activities.len()
    );
    llm.generate_productivity_score(&activities).await
}

#[tauri::command]
pub async fn get_recommendations(
    llm: State<'_, Arc<LlmClient>>,
    activity_tracker: State<'_, Arc<Mutex<crate::activity_tracking::ActivityTracker>>>,
    hours: usize,
) -> Result<Vec<String>> {
    // Get recent activities from the tracker
    let activities = {
        let tracker = activity_tracker.lock().await;
        tracker.get_recent_activities(hours * 20) // Estimate 20 activities per hour
    };

    println!(
        "Generating recommendations from {} activities",
        activities.len()
    );
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

    println!("Starting document chat with query: {query}");

    // Get recent activity context
    let activity_context = {
        let tracker = activity_tracker.lock().await;
        tracker.get_recent_activities(10)
    };

    // Search for relevant documents
    let rag = rag_system.lock().await;
    println!("Acquired RAG system lock, searching for documents...");
    let search_results = rag.search(&query, goal_uuid, limit).await.map_err(|e| {
        eprintln!("Failed to search documents: {e}");
        format!("Search failed: {e}")
    })?;

    println!("Found {len} search results", len = search_results.len());

    // Log search results for debugging
    for (i, result) in search_results.iter().enumerate() {
        println!(
            "Search Result {num}: (score: {score:.3})",
            num = i + 1,
            score = result.score
        );
        println!("Document ID: {}", result.document_id);
        println!(
            "Content: {}",
            result.content.chars().take(200).collect::<String>()
        );
        if result.content.len() > 200 {
            println!(
                "... (truncated, full length: {len} chars)",
                len = result.content.len()
            );
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
                "--- Document {num} ---\n{content}\n\n",
                num = i + 1,
                content = result.content
            ));
        }
    }

    // Add activity context
    if !activity_context.is_empty() {
        context.push_str("=== RECENT ACTIVITY CONTEXT ===\n");
        context.push_str("Your recent activities (last 10 activities):\n");
        for (i, activity) in activity_context.iter().enumerate() {
            let duration_str = if activity.duration_seconds >= 60 {
                format!(
                    "{minutes}m{seconds}s",
                    minutes = activity.duration_seconds / 60,
                    seconds = activity.duration_seconds % 60
                )
            } else {
                format!("{}s", activity.duration_seconds)
            };

            context.push_str(&format!(
                "{num}. {time} - {app} ({window}) - Duration: {duration_str}\n",
                num = i + 1,
                time = activity.timestamp.format("%H:%M"),
                app = activity.app_usage.app_name,
                window = activity.app_usage.window_title
            ));
        }
        context.push_str("\n");
    }

    // Generate response using LLM with RAG context
    let prompt = if context.is_empty() {
        format!(
            "I don't have any relevant documents to answer your question: \"{query}\"\n\n\
            Please provide a helpful response explaining that no relevant documents were found \
            and suggest how the user might get better results (such as indexing more documents or refining their query)."
        )
    } else {
        format!(
            "You are a local personal assistant running on the user's own device. \
            The user has indexed their personal documents into your local knowledge base and you have access to their activity data. \
            Answer the user's question using the available context: \"{query}\"\n\n\
            Available Context:\n{context}\n\n\
            This information includes the user's personal documents and recent activity data stored locally on their device. \
            You are running locally and have full access to help the user with their own information. \
            Use both document content and activity context to provide a comprehensive and helpful answer. \
            When referencing activities, be specific about apps, times, and durations when relevant."
        )
    };

    println!(
        "Sending prompt to LLM (length: {len} chars)",
        len = prompt.len()
    );
    let response_text = if let Some(model) = model {
        llm.send_request_with_model(&prompt, &model).await
    } else {
        llm.send_request(&prompt).await
    }
    .map_err(|e| {
        eprintln!("LLM request failed: {e}");
        format!("LLM error: {e}")
    })?;

    println!(
        "Received LLM response (length: {len} chars)",
        len = response_text.len()
    );

    Ok(ChatResponse {
        message: response_text,
        sources: search_results
            .into_iter()
            .map(|r| DocumentSource {
                document_id: r.document_id.to_string(),
                content: r.content,
                score: r.score,
            })
            .collect(),
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
        Err(e) => Err(format!("Failed to get available models: {e}")),
    }
}

#[tauri::command]
pub async fn general_chat(
    llm: State<'_, Arc<LlmClient>>,
    message: String,
    model: Option<String>,
) -> std::result::Result<String, String> {
    println!("Starting general chat with message: {message}");

    let model_name = model.unwrap_or_else(|| "llama3.2:1b".to_string());

    let prompt = format!(
        "You are a helpful AI assistant. Please respond to the following message in a conversational and helpful manner:\n\n{message}"
    );

    match llm.send_request_with_model(&prompt, &model_name).await {
        Ok(response) => {
            println!("Generated response for general chat");
            Ok(response)
        }
        Err(e) => {
            eprintln!("Failed to generate response: {e}");
            Err(format!("Failed to generate response: {e}"))
        }
    }
}
