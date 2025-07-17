use crate::error::Result;
use crate::llm::LlmClient;
use crate::rag::RAGSystemWrapper;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, State, Emitter};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamUpdate {
    pub conversation_id: String,
    pub message_id: String,
    pub update_type: StreamUpdateType,
    pub content: Option<String>,
    pub delta: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StreamUpdateType {
    Thinking,
    StreamStart,
    StreamDelta,
    StreamEnd,
    Error,
    SourcesFound,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingMetadata {
    pub step: String,
    pub progress: Option<f32>,
}

pub async fn stream_chat_response(
    app: AppHandle,
    llm: Arc<LlmClient>,
    conversation_id: String,
    message_id: String,
    prompt: String,
    model: String,
) -> Result<()> {
    // Emit thinking indicator
    emit_stream_update(
        &app,
        StreamUpdate {
            conversation_id: conversation_id.clone(),
            message_id: message_id.clone(),
            update_type: StreamUpdateType::Thinking,
            content: None,
            delta: None,
            metadata: Some(serde_json::json!({
                "step": "Preparing response",
                "progress": 0.1
            })),
        },
    );

    // Update thinking progress
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    emit_stream_update(
        &app,
        StreamUpdate {
            conversation_id: conversation_id.clone(),
            message_id: message_id.clone(),
            update_type: StreamUpdateType::Thinking,
            content: None,
            delta: None,
            metadata: Some(serde_json::json!({
                "step": "Generating response",
                "progress": 0.5
            })),
        },
    );

    // Use the LLM client to generate response
    match llm.send_request_with_model(&prompt, &model).await {
        Ok(response_text) => {
            // Parse the response to extract thinking content
            let (cleaned_response, thinking_content) = parse_llm_response(response_text);
            
            // If there's thinking content, emit it
            if let Some(thinking) = thinking_content {
                emit_stream_update(
                    &app,
                    StreamUpdate {
                        conversation_id: conversation_id.clone(),
                        message_id: message_id.clone(),
                        update_type: StreamUpdateType::Thinking,
                        content: None,
                        delta: None,
                        metadata: Some(serde_json::json!({
                            "step": "AI is thinking",
                            "progress": 0.8,
                            "thinkingContent": thinking
                        })),
                    },
                );
                
                // Give time to show thinking
                tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
            }
            // Emit stream start
            emit_stream_update(
                &app,
                StreamUpdate {
                    conversation_id: conversation_id.clone(),
                    message_id: message_id.clone(),
                    update_type: StreamUpdateType::StreamStart,
                    content: None,
                    delta: None,
                    metadata: None,
                },
            );

            // Simulate streaming by chunking the response
            let chunk_size = 20; // Characters per chunk
            let chars: Vec<char> = cleaned_response.chars().collect();
            let mut sent_content = String::new();

            for chunk in chars.chunks(chunk_size) {
                let chunk_text: String = chunk.iter().collect();
                sent_content.push_str(&chunk_text);

                // Emit delta update
                emit_stream_update(
                    &app,
                    StreamUpdate {
                        conversation_id: conversation_id.clone(),
                        message_id: message_id.clone(),
                        update_type: StreamUpdateType::StreamDelta,
                        content: Some(sent_content.clone()),
                        delta: Some(chunk_text),
                        metadata: None,
                    },
                );

                // Small delay to simulate streaming
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
            }

            // Emit stream end
            emit_stream_update(
                &app,
                StreamUpdate {
                    conversation_id: conversation_id.clone(),
                    message_id: message_id.clone(),
                    update_type: StreamUpdateType::StreamEnd,
                    content: Some(sent_content.clone()),
                    delta: None,
                    metadata: None,
                },
            );

            // Emit complete
            emit_stream_update(
                &app,
                StreamUpdate {
                    conversation_id,
                    message_id,
                    update_type: StreamUpdateType::Complete,
                    content: Some(sent_content),
                    delta: None,
                    metadata: None,
                },
            );

            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to generate response: {}", e);
            emit_stream_update(
                &app,
                StreamUpdate {
                    conversation_id,
                    message_id,
                    update_type: StreamUpdateType::Error,
                    content: None,
                    delta: None,
                    metadata: Some(serde_json::json!({
                        "error": e.to_string()
                    })),
                },
            );
            Err(e)
        }
    }
}

pub async fn stream_chat_with_documents(
    app: AppHandle,
    llm: Arc<LlmClient>,
    rag_system: Arc<Mutex<RAGSystemWrapper>>,
    conversation_id: String,
    message_id: String,
    query: String,
    goal_id: Option<String>,
    limit: Option<usize>,
    model: Option<String>,
) -> Result<()> {
    let goal_uuid = if let Some(goal_str) = goal_id {
        Some(Uuid::parse_str(&goal_str).map_err(|e| {
            crate::error::AppError::InvalidInput(format!("Invalid goal ID: {}", e))
        })?)
    } else {
        None
    };

    let limit = limit.unwrap_or(5);
    let model = model.unwrap_or_else(|| "llama3.2:1b".to_string());

    // Emit thinking - searching documents
    emit_stream_update(
        &app,
        StreamUpdate {
            conversation_id: conversation_id.clone(),
            message_id: message_id.clone(),
            update_type: StreamUpdateType::Thinking,
            content: None,
            delta: None,
            metadata: Some(serde_json::json!({
                "step": "Searching documents",
                "progress": 0.2
            })),
        },
    );

    // Search for relevant documents
    let rag = rag_system.lock().await;
    let search_results = rag
        .search(&query, goal_uuid, limit)
        .await
        .map_err(|e| crate::error::AppError::Llm(format!("Search failed: {}", e)))?;

    // Emit sources found
    if !search_results.is_empty() {
        emit_stream_update(
            &app,
            StreamUpdate {
                conversation_id: conversation_id.clone(),
                message_id: message_id.clone(),
                update_type: StreamUpdateType::SourcesFound,
                content: None,
                delta: None,
                metadata: Some(serde_json::json!({
                    "sources": search_results.iter().map(|r| {
                        serde_json::json!({
                            "document_id": r.document_id.to_string(),
                            "score": r.score,
                            "preview": r.content.chars().take(100).collect::<String>()
                        })
                    }).collect::<Vec<_>>()
                })),
            },
        );
    }

    // Build context
    let mut context = String::new();
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

    // Emit thinking - generating response
    emit_stream_update(
        &app,
        StreamUpdate {
            conversation_id: conversation_id.clone(),
            message_id: message_id.clone(),
            update_type: StreamUpdateType::Thinking,
            content: None,
            delta: None,
            metadata: Some(serde_json::json!({
                "step": "Generating response",
                "progress": 0.5
            })),
        },
    );

    // Generate prompt
    let prompt = if context.is_empty() {
        format!(
            "I don't have any relevant documents to answer your question: \"{}\"\n\n\
            Please provide a helpful response explaining that no relevant documents were found \
            and suggest how the user might get better results.",
            query
        )
    } else {
        format!(
            "You are a local personal assistant. Answer the user's question using the available context: \"{}\"\n\n\
            Available Context:\n{}\n\n\
            Use the document content to provide a comprehensive and helpful answer.",
            query, context
        )
    };

    // Stream the response
    stream_chat_response(app, llm, conversation_id, message_id, prompt, model).await
}

fn emit_stream_update(app: &AppHandle, update: StreamUpdate) {
    if let Err(e) = app.emit("chat-stream", &update) {
        eprintln!("Failed to emit stream update: {}", e);
    }
}

fn parse_llm_response(response: String) -> (String, Option<String>) {
    let mut thinking_content = None;
    let mut clean_response = response.clone();
    
    // Extract thinking content
    if let Some(start) = response.find("<think>") {
        if let Some(end) = response.find("</think>") {
            let thinking = response[start + 7..end].trim().to_string();
            thinking_content = Some(thinking);
            clean_response.replace_range(start..=end + 8, "");
        }
    } else if let Some(start) = response.find("<thinking>") {
        if let Some(end) = response.find("</thinking>") {
            let thinking = response[start + 10..end].trim().to_string();
            thinking_content = Some(thinking);
            clean_response.replace_range(start..=end + 11, "");
        }
    }
    
    (clean_response.trim().to_string(), thinking_content)
}

// Tauri commands for streaming chat
#[tauri::command]
pub async fn stream_general_chat(
    app: AppHandle,
    llm: State<'_, Arc<LlmClient>>,
    conversation_id: String,
    message_id: String,
    message: String,
    model: Option<String>,
) -> std::result::Result<(), String> {
    let model_name = model.unwrap_or_else(|| "llama3.2:1b".to_string());
    let prompt = format!(
        "You are a helpful AI assistant. Please respond to the following message in a conversational and helpful manner:\n\n{}",
        message
    );
    
    let llm_clone = llm.inner().clone();

    tokio::spawn(async move {
        if let Err(e) = stream_chat_response(
            app,
            llm_clone,
            conversation_id,
            message_id,
            prompt,
            model_name,
        )
        .await
        {
            eprintln!("Stream chat error: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stream_document_chat(
    app: AppHandle,
    llm: State<'_, Arc<LlmClient>>,
    rag_system: State<'_, Arc<Mutex<RAGSystemWrapper>>>,
    conversation_id: String,
    message_id: String,
    query: String,
    goal_id: Option<String>,
    limit: Option<usize>,
    model: Option<String>,
) -> std::result::Result<(), String> {
    let llm_clone = llm.inner().clone();
    let rag_system_clone = rag_system.inner().clone();

    tokio::spawn(async move {
        if let Err(e) = stream_chat_with_documents(
            app,
            llm_clone,
            rag_system_clone,
            conversation_id,
            message_id,
            query,
            goal_id,
            limit,
            model,
        )
        .await
        {
            eprintln!("Stream document chat error: {}", e);
        }
    });

    Ok(())
}