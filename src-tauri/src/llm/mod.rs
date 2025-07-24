use crate::config::{Config, InferenceProvider};
use crate::error::{AppError, Result};
use crate::models::{Activity, ProductivityInsights, ProductivityScore};
use chrono::Utc;
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use std::path::PathBuf;

mod candle_backend;
use candle_backend::CandleBackend;

mod crane_backend;
use crane_backend::CraneBackend;

mod callm_backend;
use callm_backend::CallmBackend;

mod bert_metal_backend;

pub mod llama_cpp_metal_backend;
use llama_cpp_metal_backend::LlamaCppMetalBackend;

pub struct LlmClient {
    model_name: String,
    inference_provider: InferenceProvider,
    candle_backend: Option<CandleBackend>,
    crane_backend: Option<CraneBackend>,
    callm_backend: Option<CallmBackend>,
    llama_cpp_backend: Option<LlamaCppMetalBackend>,
}

impl LlmClient {
    pub fn new() -> Self {
        let config = Config::get();
        
        // For synchronous new(), we'll initialize without Candle/Crane
        // They will be initialized later via init_candle_if_needed()
        Self {
            model_name: config.services.ollama_model.clone(),
            inference_provider: config.services.inference_provider.clone(),
            candle_backend: None,
            crane_backend: None,
            callm_backend: None,
            llama_cpp_backend: None,
        }
    }
    
    /// Update the inference provider and model without restarting
    pub async fn update_inference_provider(
        &mut self,
        provider: InferenceProvider,
        model_id: Option<String>,
    ) -> Result<()> {
        println!("[LlmClient] Updating inference provider to: {:?}", provider);
        
        // Update the provider
        self.inference_provider = provider.clone();
        
        // Update model IDs based on provider
        match provider {
            InferenceProvider::Ollama => {
                if let Some(id) = model_id {
                    self.model_name = id;
                }
                // Clear other backends
                self.candle_backend = None;
                self.crane_backend = None;
                self.callm_backend = None;
            }
            InferenceProvider::Candle => {
                // Initialize Candle backend if needed
                if self.candle_backend.is_none() {
                    let config = Config::get();
                    let model_id = model_id.as_ref().unwrap_or(&config.services.candle_model_id);
                    
                    println!("[LlmClient] Initializing Candle backend with model: {}", model_id);
                    match CandleBackend::new(
                        model_id,
                        &config.services.candle_model_revision,
                        PathBuf::from(&config.services.candle_cache_dir),
                    ).await {
                        Ok(backend) => {
                            println!("[LlmClient] Candle backend initialized successfully");
                            self.candle_backend = Some(backend);
                        }
                        Err(e) => {
                            eprintln!("[LlmClient] Failed to initialize Candle backend: {}", e);
                            return Err(e);
                        }
                    }
                }
                // Clear other backends
                self.crane_backend = None;
                self.callm_backend = None;
            }
            InferenceProvider::Crane => {
                // Initialize Crane backend if needed
                if self.crane_backend.is_none() {
                    let config = Config::get();
                    let model_id = model_id.as_ref().unwrap_or(&config.services.candle_model_id);
                    
                    println!("[LlmClient] Initializing Crane backend with model: {}", model_id);
                    match CraneBackend::new(
                        model_id,
                        PathBuf::from(&config.services.candle_cache_dir),
                    ).await {
                        Ok(backend) => {
                            println!("[LlmClient] Crane backend initialized successfully");
                            self.crane_backend = Some(backend);
                        }
                        Err(e) => {
                            eprintln!("[LlmClient] Failed to initialize Crane backend: {}", e);
                            return Err(e);
                        }
                    }
                }
                // Clear other backends
                self.candle_backend = None;
                self.callm_backend = None;
            }
            InferenceProvider::Callm => {
                // Initialize Callm backend if needed
                if self.callm_backend.is_none() {
                    let config = Config::get();
                    let model_id = model_id.as_ref().unwrap_or(&config.services.candle_model_id);
                    
                    println!("[LlmClient] Initializing Callm backend with model: {}", model_id);
                    match CallmBackend::new(
                        model_id,
                        PathBuf::from(&config.services.candle_cache_dir),
                    ).await {
                        Ok(backend) => {
                            println!("[LlmClient] Callm backend initialized successfully");
                            self.callm_backend = Some(backend);
                        }
                        Err(e) => {
                            eprintln!("[LlmClient] Failed to initialize Callm backend: {}", e);
                            return Err(e);
                        }
                    }
                }
                // Clear other backends
                self.candle_backend = None;
                self.crane_backend = None;
            }
            InferenceProvider::LlamaCpp => {
                // Initialize LlamaCpp backend if needed
                if self.llama_cpp_backend.is_none() {
                    let config = Config::get();
                    let model_id = model_id.as_ref().unwrap_or(&config.services.candle_model_id);
                    
                    println!("[LlmClient] Initializing LlamaCpp backend with model: {}", model_id);
                    match LlamaCppMetalBackend::new(
                        model_id,
                        PathBuf::from(&config.services.candle_cache_dir),
                    ).await {
                        Ok(backend) => {
                            println!("[LlmClient] LlamaCpp backend initialized successfully");
                            self.llama_cpp_backend = Some(backend);
                        }
                        Err(e) => {
                            eprintln!("[LlmClient] Failed to initialize LlamaCpp backend: {}", e);
                            return Err(e);
                        }
                    }
                }
                // Clear other backends
                self.candle_backend = None;
                self.crane_backend = None;
                self.callm_backend = None;
            }
        }
        
        println!("[LlmClient] Inference provider updated successfully");
        Ok(())
    }
    
    pub async fn new_async() -> Self {
        let config = Config::get();
        
        // Initialize appropriate backend based on provider
        let (candle_backend, crane_backend, callm_backend, llama_cpp_backend) = match config.services.inference_provider {
            InferenceProvider::Candle => {
                println!("[LlmClient] Initializing Candle backend...");
                match CandleBackend::new(
                    &config.services.candle_model_id,
                    &config.services.candle_model_revision,
                    PathBuf::from(&config.services.candle_cache_dir),
                ).await {
                    Ok(backend) => {
                        println!("[LlmClient] Candle backend initialized successfully");
                        (Some(backend), None, None, None)
                    }
                    Err(e) => {
                        eprintln!("[LlmClient] Failed to initialize Candle backend: {}", e);
                        eprintln!("[LlmClient] Falling back to Ollama");
                        (None, None, None, None)
                    }
                }
            }
            InferenceProvider::Crane => {
                println!("[LlmClient] Initializing Crane backend...");
                match CraneBackend::new(
                    &config.services.candle_model_id, // Reuse same model config
                    PathBuf::from(&config.services.candle_cache_dir),
                ).await {
                    Ok(backend) => {
                        println!("[LlmClient] Crane backend initialized successfully");
                        (None, Some(backend), None, None)
                    }
                    Err(e) => {
                        eprintln!("[LlmClient] Failed to initialize Crane backend: {}", e);
                        eprintln!("[LlmClient] Falling back to Ollama");
                        (None, None, None, None)
                    }
                }
            }
            InferenceProvider::Callm => {
                println!("[LlmClient] Initializing Callm backend...");
                match CallmBackend::new(
                    &config.services.candle_model_id,
                    PathBuf::from(&config.services.candle_cache_dir),
                ).await {
                    Ok(backend) => {
                        println!("[LlmClient] Callm backend initialized successfully");
                        (None, None, Some(backend), None)
                    }
                    Err(e) => {
                        eprintln!("[LlmClient] Failed to initialize Callm backend: {}", e);
                        eprintln!("[LlmClient] Falling back to Ollama");
                        (None, None, None, None)
                    }
                }
            }
            InferenceProvider::LlamaCpp => {
                let device_type = if cfg!(target_os = "macos") { "Metal" } 
                                else if cfg!(target_os = "windows") { "GPU" }
                                else { "CPU" };
                println!("[LlmClient] Initializing LlamaCpp backend with {} support...", device_type);
                match LlamaCppMetalBackend::new(
                    &config.services.candle_model_id,
                    PathBuf::from(&config.services.candle_cache_dir),
                ).await {
                    Ok(backend) => {
                        println!("[LlmClient] LlamaCpp backend initialized successfully with {} support", device_type);
                        (None, None, None, Some(backend))
                    }
                    Err(e) => {
                        eprintln!("[LlmClient] Failed to initialize LlamaCpp backend: {}", e);
                        eprintln!("[LlmClient] Falling back to Ollama");
                        (None, None, None, None)
                    }
                }
            }
            _ => (None, None, None, None),
        };
        
        Self {
            model_name: config.services.ollama_model.clone(),
            inference_provider: config.services.inference_provider.clone(),
            candle_backend,
            crane_backend,
            callm_backend,
            llama_cpp_backend,
        }
    }
    
    /// Initialize backend if needed (can be called after construction)
    pub async fn init_backend_if_needed(&mut self) -> Result<()> {
        let config = Config::get();
        
        match config.services.inference_provider {
            InferenceProvider::Candle if self.candle_backend.is_none() => {
                println!("[LlmClient] Late initialization of Candle backend...");
                match CandleBackend::new(
                    &config.services.candle_model_id,
                    &config.services.candle_model_revision,
                    PathBuf::from(&config.services.candle_cache_dir),
                ).await {
                    Ok(backend) => {
                        println!("[LlmClient] Candle backend initialized successfully");
                        self.candle_backend = Some(backend);
                    }
                    Err(e) => {
                        eprintln!("[LlmClient] Failed to initialize Candle backend: {}", e);
                        return Err(e);
                    }
                }
            }
            InferenceProvider::Crane if self.crane_backend.is_none() => {
                println!("[LlmClient] Late initialization of Crane backend...");
                match CraneBackend::new(
                    &config.services.candle_model_id,
                    PathBuf::from(&config.services.candle_cache_dir),
                ).await {
                    Ok(backend) => {
                        println!("[LlmClient] Crane backend initialized successfully");
                        self.crane_backend = Some(backend);
                    }
                    Err(e) => {
                        eprintln!("[LlmClient] Failed to initialize Crane backend: {}", e);
                        return Err(e);
                    }
                }
            }
            InferenceProvider::Callm if self.callm_backend.is_none() => {
                println!("[LlmClient] Late initialization of Callm backend...");
                match CallmBackend::new(
                    &config.services.candle_model_id,
                    PathBuf::from(&config.services.candle_cache_dir),
                ).await {
                    Ok(backend) => {
                        println!("[LlmClient] Callm backend initialized successfully");
                        self.callm_backend = Some(backend);
                    }
                    Err(e) => {
                        eprintln!("[LlmClient] Failed to initialize Callm backend: {}", e);
                        return Err(e);
                    }
                }
            }
            InferenceProvider::LlamaCpp if self.llama_cpp_backend.is_none() => {
                println!("[LlmClient] Late initialization of LlamaCpp backend...");
                match LlamaCppMetalBackend::new(
                    &config.services.candle_model_id,
                    PathBuf::from(&config.services.candle_cache_dir),
                ).await {
                    Ok(backend) => {
                        println!("[LlmClient] LlamaCpp backend initialized successfully");
                        self.llama_cpp_backend = Some(backend);
                    }
                    Err(e) => {
                        eprintln!("[LlmClient] Failed to initialize LlamaCpp backend: {}", e);
                        return Err(e);
                    }
                }
            }
            _ => {}
        }
        
        Ok(())
    }

    pub async fn generate_productivity_insights(
        &self,
        activities: &[Activity],
    ) -> Result<ProductivityInsights> {
        println!("[LLM] Generating productivity insights for {} activities", activities.len());
        
        // If no activities, return helpful getting started message
        if activities.is_empty() {
            return Ok(ProductivityInsights {
                summary: "No activity data available yet. Start tracking to see insights."
                    .to_string(),
                key_insights: vec![
                    "Enable activity tracking to monitor productivity".to_string(),
                    "Set up goals to track focused work time".to_string(),
                    "Track for a few hours to get meaningful insights".to_string(),
                ],
                suggested_improvements: vec![
                    "Start tracking your daily activities".to_string(),
                    "Create goals for better focus tracking".to_string(),
                    "Use the app for work tasks to build data".to_string(),
                ],
                timestamp: Utc::now(),
            });
        }

        let activities_json = serde_json::to_string_pretty(activities)?;

        let prompt = format!(
            "Analyze the following activity data and provide productivity insights:\n\n{}\n\n\
            Please provide:\n\
            1. A brief summary of the user's productivity patterns\n\
            2. 3-5 key insights about their work habits\n\
            3. 3-5 specific suggestions for improvement\n\n\
            Format your response as JSON with the following structure:\n\
            {{\n\
              \"summary\": \"Brief summary here\",\n\
              \"key_insights\": [\"insight1\", \"insight2\", ...],\n\
              \"suggested_improvements\": [\"improvement1\", \"improvement2\", ...]\n\
            }}",
            activities_json
        );

        let response = self.send_request(&prompt).await?;
        let json_response = self.extract_json(&response)?;

        Ok(ProductivityInsights {
            summary: json_response["summary"].as_str().unwrap_or("").to_string(),
            key_insights: json_response["key_insights"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default(),
            suggested_improvements: json_response["suggested_improvements"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default(),
            timestamp: Utc::now(),
        })
    }

    pub async fn generate_productivity_score(
        &self,
        activities: &[Activity],
    ) -> Result<ProductivityScore> {
        // If no activities, return baseline scores
        if activities.is_empty() {
            return Ok(ProductivityScore {
                overall: 0.0,
                focus: 0.0,
                efficiency: 0.0,
                breaks: 0.0,
                timestamp: Utc::now(),
            });
        }

        let activities_json = serde_json::to_string_pretty(activities)?;

        let prompt = format!(
            "Analyze the following activity data and calculate productivity scores:\n\n{}\n\n\
            Calculate scores (0-100) for:\n\
            1. Overall productivity\n\
            2. Focus (time spent on productive tasks)\n\
            3. Efficiency (active work vs idle time)\n\
            4. Break quality (appropriate breaks taken)\n\n\
            Format your response as JSON:\n\
            {{\n\
              \"overall\": 85,\n\
              \"focus\": 90,\n\
              \"efficiency\": 80,\n\
              \"breaks\": 75\n\
            }}",
            activities_json
        );

        let response = self.send_request(&prompt).await?;
        let json_response = self.extract_json(&response)?;

        Ok(ProductivityScore {
            overall: json_response["overall"].as_f64().unwrap_or(0.0) as f32,
            focus: json_response["focus"].as_f64().unwrap_or(0.0) as f32,
            efficiency: json_response["efficiency"].as_f64().unwrap_or(0.0) as f32,
            breaks: json_response["breaks"].as_f64().unwrap_or(0.0) as f32,
            timestamp: Utc::now(),
        })
    }

    pub async fn generate_recommendations(&self, activities: &[Activity]) -> Result<Vec<String>> {
        let activities_json = serde_json::to_string_pretty(activities)?;

        let prompt = format!(
            "Based on the following activity data, provide 5 specific, actionable recommendations:\n\n{}\n\n\
            Format your response as a JSON array of strings:\n\
            [\"recommendation1\", \"recommendation2\", ...]",
            activities_json
        );

        let response = self.send_request(&prompt).await?;
        let json_response = self.extract_json(&response)?;

        Ok(json_response
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn split_research_query(&self, query: &str) -> Result<Vec<String>> {
        let prompt = format!(
            "Split the following research query into 3-5 specific subtasks that can be searched independently:\n\n\
            Query: {}\n\n\
            Format your response as a JSON array of search queries:\n\
            [\"subtask1\", \"subtask2\", ...]",
            query
        );

        let response = self.send_request(&prompt).await?;
        let json_response = self.extract_json(&response)?;

        Ok(json_response
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_else(|| vec![query.to_string()]))
    }

    pub async fn synthesize_research(&self, query: &str, results: &str) -> Result<String> {
        let prompt = format!(
            "Synthesize the following research results for the query '{}' into a comprehensive conclusion:\n\n\
            Results:\n{}\n\n\
            Provide a well-structured summary with key findings and insights.",
            query, results
        );

        self.send_request(&prompt).await
    }

    pub async fn send_request(&self, prompt: &str) -> Result<String> {
        self.send_request_with_model(prompt, &self.model_name).await
    }

    pub async fn send_request_with_model(&self, prompt: &str, model: &str) -> Result<String> {
        // Get current configuration to support hot-swapping
        let config = Config::get();
        let current_provider = &config.services.inference_provider;
        
        // Log the inference provider and model being used
        println!("[LLM] Current inference provider: {:?}", current_provider);
        println!("[LLM] Configured provider: {:?}", self.inference_provider);
        
        // Check which backend to use based on current configuration
        match current_provider {
            InferenceProvider::Candle => {
                if let Some(candle) = &self.candle_backend {
                    println!("[LLM] Using Candle backend for inference");
                    println!("[LLM] Candle model: {}", config.services.candle_model_id);
                    return candle.generate(prompt, 500).await; // Max 500 tokens
                } else {
                    println!("[LLM] Candle backend not available, initializing...");
                    // Try to initialize Candle backend on demand
                    if let Ok(backend) = CandleBackend::new(
                        &config.services.candle_model_id,
                        &config.services.candle_model_revision,
                        PathBuf::from(&config.services.candle_cache_dir),
                    ).await {
                        println!("[LLM] Candle backend initialized successfully");
                        return backend.generate(prompt, 500).await;
                    } else {
                        println!("[LLM] Failed to initialize Candle backend, falling back to Ollama");
                    }
                }
            }
            InferenceProvider::Crane => {
                if let Some(crane) = &self.crane_backend {
                    println!("[LLM] Using Crane backend for inference");
                    println!("[LLM] Crane model: {}", config.services.candle_model_id);
                    return crane.generate(prompt, 500).await; // Max 500 tokens
                } else {
                    println!("[LLM] Crane backend not available, initializing...");
                    // Try to initialize Crane backend on demand
                    if let Ok(backend) = CraneBackend::new(
                        &config.services.candle_model_id,
                        PathBuf::from(&config.services.candle_cache_dir),
                    ).await {
                        println!("[LLM] Crane backend initialized successfully");
                        return backend.generate(prompt, 500).await;
                    } else {
                        println!("[LLM] Failed to initialize Crane backend, falling back to Ollama");
                    }
                }
            }
            InferenceProvider::Callm => {
                // Get the current model from environment variable (which gets updated dynamically)
                let current_model = std::env::var("CANDLE_MODEL_ID")
                    .unwrap_or_else(|_| config.services.candle_model_id.clone());
                
                println!("[LLM] Using Callm backend for inference");
                println!("[LLM] Requested model: {}", current_model);
                
                // Check if we need to reinitialize with a different model
                let needs_reinit = if let Some(callm) = &self.callm_backend {
                    // Check if the model has changed
                    let backend_model = &callm.model_id;
                    backend_model != &current_model
                } else {
                    true
                };
                
                if needs_reinit {
                    println!("[LLM] Initializing Callm backend with model: {}", current_model);
                    // Initialize new backend with current model
                    match CallmBackend::new(
                        &current_model,
                        PathBuf::from(&config.services.candle_cache_dir),
                    ).await {
                        Ok(backend) => {
                            println!("[LLM] Callm backend initialized successfully");
                            match backend.generate(prompt, 500).await {
                                Ok(response) => {
                                    println!("[LLM] Callm generated response successfully");
                                    return Ok(response);
                                }
                                Err(e) => {
                                    eprintln!("[LLM] Callm generation failed: {}", e);
                                    eprintln!("[LLM] Falling back to Ollama");
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[LLM] Failed to initialize Callm backend: {}", e);
                            eprintln!("[LLM] Falling back to Ollama");
                        }
                    }
                } else if let Some(callm) = &self.callm_backend {
                    // Use existing backend
                    match callm.generate(prompt, 500).await {
                        Ok(response) => {
                            println!("[LLM] Callm generated response successfully");
                            return Ok(response);
                        }
                        Err(e) => {
                            eprintln!("[LLM] Callm generation failed: {}", e);
                            eprintln!("[LLM] Falling back to Ollama");
                        }
                    }
                }
            }
            InferenceProvider::LlamaCpp => {
                if let Some(llama_cpp) = &self.llama_cpp_backend {
                    let device_type = if cfg!(target_os = "macos") { "Metal" } 
                                    else if cfg!(target_os = "windows") { "GPU" }
                                    else { "CPU" };
                    println!("[LLM] Using LlamaCpp backend for inference with {} support", device_type);
                    println!("[LLM] LlamaCpp model: {}", config.services.candle_model_id);
                    
                    // Try to generate with LlamaCpp, fall back to Ollama on error
                    match llama_cpp.generate(prompt, 500).await {
                        Ok(response) => return Ok(response),
                        Err(e) => {
                            eprintln!("[LLM] LlamaCpp generation failed: {}, falling back to Ollama", e);
                        }
                    }
                } else {
                    println!("[LLM] LlamaCpp backend not available, initializing...");
                    // Try to initialize LlamaCpp backend on demand
                    match LlamaCppMetalBackend::new(
                        &config.services.candle_model_id,
                        PathBuf::from(&config.services.candle_cache_dir),
                    ).await {
                        Ok(backend) => {
                            println!("[LLM] LlamaCpp backend initialized successfully");
                            match backend.generate(prompt, 500).await {
                                Ok(response) => return Ok(response),
                                Err(e) => {
                                    eprintln!("[LLM] LlamaCpp generation failed: {}, falling back to Ollama", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[LLM] Failed to initialize LlamaCpp backend: {}, falling back to Ollama", e);
                        }
                    }
                }
            }
            _ => {}
        }
        
        // Use Ollama as fallback
        println!("[LLM] Using Ollama for inference");
        println!("[LLM] Ollama model: {}", model);
        let client = Client::default();

        let chat_req = ChatRequest::new(vec![ChatMessage::user(prompt)]);

        println!("LLM: Sending request to model...");
        let chat_response = client.exec_chat(model, chat_req, None).await.map_err(|e| {
            eprintln!("LLM: Request failed: {}", e);
            AppError::Llm(format!("LLM request failed: {}", e))
        })?;

        println!("LLM: Received response, extracting content...");
        let result = chat_response
            .content_text_as_str()
            .ok_or_else(|| AppError::Llm("Empty response from LLM".into()))
            .map(|s| s.to_string());

        match &result {
            Ok(content) => println!(
                "LLM: Successfully extracted content (length: {})",
                content.len()
            ),
            Err(e) => eprintln!("LLM: Failed to extract content: {}", e),
        }

        result
    }

    pub async fn get_available_models(&self) -> Result<Vec<String>> {
        println!("LLM: Getting available models from Ollama");
        let client = reqwest::Client::new();
        let config = Config::get();
        let url = format!("{}/api/tags", config.services.ollama_url);

        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(data) => {
                            if let Some(models) = data.get("models").and_then(|m| m.as_array()) {
                                let model_names: Vec<String> = models
                                    .iter()
                                    .filter_map(|model| model.get("name").and_then(|n| n.as_str()))
                                    .map(|s| s.to_string())
                                    .collect();
                                println!("LLM: Found {} available models", model_names.len());
                                Ok(model_names)
                            } else {
                                println!("LLM: No models found in response");
                                Ok(vec![])
                            }
                        }
                        Err(e) => {
                            eprintln!("LLM: Failed to parse models response: {}", e);
                            Err(AppError::Llm(format!(
                                "Failed to parse models response: {}",
                                e
                            )))
                        }
                    }
                } else {
                    eprintln!("LLM: Failed to list models, status: {}", response.status());
                    Err(AppError::Llm(format!(
                        "Failed to list models, status: {}",
                        response.status()
                    )))
                }
            }
            Err(e) => {
                eprintln!("LLM: Failed to connect to Ollama: {}", e);
                Err(AppError::Llm(format!("Failed to connect to Ollama: {}", e)))
            }
        }
    }

    fn extract_json(&self, text: &str) -> Result<serde_json::Value> {
        // Try to extract JSON from the response
        // LLMs sometimes wrap JSON in markdown code blocks
        let cleaned = if text.contains("```json") {
            text.split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(text)
        } else if text.contains("```") {
            text.split("```")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(text)
        } else {
            text
        };

        serde_json::from_str(cleaned.trim())
            .map_err(|e| AppError::Llm(format!("Failed to parse JSON response: {}", e)))
    }
    
    pub async fn get_inference_info(&self) -> InferenceInfo {
        let config = Config::get();
        
        let (provider_str, model_display) = match &self.inference_provider {
            InferenceProvider::Candle => {
                let model = match config.services.candle_model_id.as_str() {
                    "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => "TinyLlama 1.1B Chat",
                    "microsoft/phi-2" => "Phi-2 2.7B",
                    "mistralai/Mistral-7B-v0.1" => "Mistral 7B",
                    _ => &config.services.candle_model_id,
                };
                ("Candle", model.to_string())
            }
            InferenceProvider::Crane => {
                let model = match config.services.candle_model_id.as_str() {
                    "Qwen/Qwen2.5-0.5B-Instruct" => "Qwen2.5 0.5B (Crane)",
                    "Qwen/Qwen2.5-1.5B-Instruct" => "Qwen2.5 1.5B (Crane)",
                    "Qwen/Qwen2.5-3B-Instruct" => "Qwen2.5 3B (Crane)",
                    "Qwen/Qwen2.5-7B-Instruct" => "Qwen2.5 7B (Crane)",
                    _ => &config.services.candle_model_id,
                };
                ("Crane", model.to_string())
            }
            InferenceProvider::Callm => {
                let model = match config.services.candle_model_id.as_str() {
                    "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => "TinyLlama 1.1B (Callm)",
                    "microsoft/phi-2" => "Phi-2 2.7B (Callm)",
                    model if model.starts_with("Qwen/Qwen2") => "Qwen2 (Callm)",
                    _ => &config.services.candle_model_id,
                };
                ("Callm", model.to_string())
            }
            InferenceProvider::Ollama => {
                ("Ollama", self.model_name.clone())
            }
            InferenceProvider::LlamaCpp => {
                let device_suffix = if cfg!(target_os = "macos") { "Metal" } 
                                  else if cfg!(target_os = "windows") { "GPU" }
                                  else { "CPU" };
                let model = match config.services.candle_model_id.as_str() {
                    "TinyLlama/TinyLlama-1.1B-Chat-v1.0" => format!("TinyLlama 1.1B (LlamaCpp/{})", device_suffix),
                    "Qwen/Qwen2.5-0.5B-Instruct" => format!("Qwen2.5 0.5B (LlamaCpp/{})", device_suffix),
                    _ => format!("{} (LlamaCpp/{})", config.services.candle_model_id, device_suffix),
                };
                ("LlamaCpp", model)
            }
        };
        
        println!("[LLM] Inference info - Provider: {}, Model: {}", provider_str, model_display);
        
        // For now, skip backend info to avoid hanging
        InferenceInfo {
            provider: provider_str.to_string(),
            model_name: model_display,
            candle_info: None, // Skip backend info for now
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct InferenceInfo {
    pub provider: String,
    pub model_name: String,
    pub candle_info: Option<candle_backend::ModelInfo>,
}
