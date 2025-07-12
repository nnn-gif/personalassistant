use crate::error::{AppError, Result};
use crate::models::{
    BrowserAIProgress, ResearchResult, ResearchSubtask, ResearchTask, SearchResult, TaskStatus,
};
use crate::llm::LlmClient;
use super::{ScraperEngine, ChromeController};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct ResearchPlan {
    main_topic: String,
    category: String,
    subtopics: Vec<String>,
    search_queries: Vec<String>,
    requires_browser: bool,
}

pub struct BrowserAIAgent {
    scraper: ScraperEngine,
    chrome: ChromeController,
    active_tasks: HashMap<Uuid, ResearchTask>,
    llm_client: Arc<LlmClient>,
}

impl BrowserAIAgent {
    pub fn new() -> Self {
        Self {
            scraper: ScraperEngine::new(),
            chrome: ChromeController::new(),
            active_tasks: HashMap::new(),
            llm_client: Arc::new(LlmClient::new()),
        }
    }
    
    pub async fn start_research(
        &mut self,
        query: String,
        progress_sender: mpsc::Sender<BrowserAIProgress>,
    ) -> Result<Uuid> {
        println!("Agent: Starting research for: {}", query);
        
        let task_id = Uuid::new_v4();
        let now = Utc::now();
        
        let mut task = ResearchTask {
            id: task_id,
            query: query.clone(),
            status: TaskStatus::Pending,
            subtasks: Vec::new(),
            results: Vec::new(),
            conclusion: None,
            created_at: now,
            updated_at: now,
        };
        
        // Step 1: Create research plan
        task.status = TaskStatus::SplittingTasks;
        if let Err(e) = self.send_progress(&task, &progress_sender).await {
            println!("Agent: Error sending initial progress: {}", e);
        }
        
        println!("Agent: Creating research plan...");
        let plan = match self.create_research_plan(&query).await {
            Ok(p) => {
                println!("Agent: Research plan created successfully");
                println!("Plan: {:?}", p);
                p
            },
            Err(e) => {
                println!("Agent: Error creating research plan: {}", e);
                task.status = TaskStatus::Failed(format!("Failed to create plan: {}", e));
                let _ = self.send_progress(&task, &progress_sender).await;
                return Err(e);
            }
        };
        
        // Convert plan to subtasks
        let subtasks = self.plan_to_subtasks(&plan).await?;
        task.subtasks = subtasks;
        
        // Step 2: Execute searches
        task.status = TaskStatus::Searching;
        self.send_progress(&task, &progress_sender).await?;
        
        for subtask in &mut task.subtasks {
            let search_results = if plan.requires_browser {
                self.search_with_browser(&subtask.query).await?
            } else {
                self.search_web(&subtask.query).await?
            };
            subtask.search_results = search_results;
        }
        
        // Step 3: Intelligent scraping
        task.status = TaskStatus::Scraping;
        self.send_progress(&task, &progress_sender).await?;
        
        let mut results = Vec::new();
        for subtask in &task.subtasks {
            // Prioritize top results
            let top_results = subtask.search_results.iter()
                .take(3)
                .collect::<Vec<_>>();
            
            for search_result in top_results {
                if let Ok(content) = self.scraper.scrape_url(&search_result.url).await {
                    // Extract relevant content using LLM
                    let extracted = self.extract_relevant_content(&content, &subtask.query).await?;
                    
                    let result = ResearchResult {
                        id: Uuid::new_v4(),
                        subtask_id: subtask.id,
                        url: search_result.url.clone(),
                        title: search_result.title.clone(),
                        content: extracted,
                        relevance_score: search_result.relevance_score,
                        scraped_at: Utc::now(),
                    };
                    results.push(result);
                }
            }
        }
        task.results = results;
        
        // Step 4: Synthesize results
        task.status = TaskStatus::Analyzing;
        self.send_progress(&task, &progress_sender).await?;
        
        let conclusion = self.synthesize_results(&task, &plan).await?;
        task.conclusion = Some(conclusion);
        
        // Complete
        task.status = TaskStatus::Completed;
        task.updated_at = Utc::now();
        self.send_progress(&task, &progress_sender).await?;
        
        self.active_tasks.insert(task_id, task);
        Ok(task_id)
    }
    
    pub fn get_task(&self, task_id: &Uuid) -> Option<&ResearchTask> {
        self.active_tasks.get(task_id)
    }
    
    pub fn get_all_tasks(&self) -> Vec<&ResearchTask> {
        self.active_tasks.values().collect()
    }
    
    async fn create_research_plan(&self, query: &str) -> Result<ResearchPlan> {
        // Try to use LLM for intelligent planning
        let prompt = format!(
            "Create a research plan for the following query: '{}'\n\n\
            Analyze the query and return a JSON object with:\n\
            1. main_topic: The main topic being researched\n\
            2. category: Category (e.g., 'technical', 'academic', 'news', 'product', 'general')\n\
            3. subtopics: Array of 3-5 specific subtopics to research\n\
            4. search_queries: Array of optimized search queries\n\
            5. requires_browser: Boolean indicating if interactive browser is needed\n\n\
            Format your response as JSON:\n\
            {{\n\
              \"main_topic\": \"...\",\n\
              \"category\": \"...\",\n\
              \"subtopics\": [\"...\"],\n\
              \"search_queries\": [\"...\"],\n\
              \"requires_browser\": false\n\
            }}",
            query
        );
        
        match self.llm_client.send_request(&prompt).await {
            Ok(response) => {
                match serde_json::from_str::<ResearchPlan>(&self.extract_json(&response)?) {
                    Ok(plan) => Ok(plan),
                    Err(e) => {
                        println!("Failed to parse LLM response, using fallback: {}", e);
                        Ok(self.create_fallback_plan(query))
                    }
                }
            }
            Err(e) => {
                println!("LLM request failed, using fallback plan: {}", e);
                Ok(self.create_fallback_plan(query))
            }
        }
    }
    
    fn create_fallback_plan(&self, query: &str) -> ResearchPlan {
        ResearchPlan {
            main_topic: query.to_string(),
            category: "general".to_string(),
            subtopics: vec![
                format!("{} overview", query),
                format!("{} details", query),
                format!("{} examples", query),
            ],
            search_queries: vec![
                query.to_string(),
                format!("{} tutorial", query),
                format!("{} guide", query),
            ],
            requires_browser: false,
        }
    }
    
    async fn plan_to_subtasks(&self, plan: &ResearchPlan) -> Result<Vec<ResearchSubtask>> {
        let mut subtasks = Vec::new();
        
        for query in plan.search_queries.iter() {
            let subtask = ResearchSubtask {
                id: Uuid::new_v4(),
                query: query.clone(),
                status: TaskStatus::Pending,
                search_results: Vec::new(),
            };
            subtasks.push(subtask);
        }
        
        Ok(subtasks)
    }
    
    async fn search_with_browser(&self, query: &str) -> Result<Vec<SearchResult>> {
        // Open Chrome with search query
        self.chrome.search_google(query).await?;
        
        // Wait a bit for page to load
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        
        // Try to get search results from Chrome
        match self.chrome.get_search_results().await {
            Ok(results) if !results.is_empty() => {
                Ok(results.into_iter()
                    .enumerate()
                    .map(|(i, (url, title))| SearchResult {
                        url,
                        title,
                        snippet: String::new(),
                        relevance_score: 1.0 - (i as f32 * 0.05),
                    })
                    .collect())
            }
            _ => {
                // Fallback to regular web search
                self.search_web(query).await
            }
        }
    }
    
    async fn extract_relevant_content(&self, content: &str, query: &str) -> Result<String> {
        let prompt = format!(
            "Extract the most relevant information from the following content for the query: '{}'\n\n\
            Content:\n{}\n\n\
            Extract and summarize only the parts directly relevant to the query. \
            Keep important details, facts, and examples. Limit to 500 words.",
            query,
            content.chars().take(5000).collect::<String>()
        );
        
        match self.llm_client.send_request(&prompt).await {
            Ok(extracted) => Ok(extracted),
            Err(_) => {
                // Fallback: Return first 500 chars of content
                Ok(content.chars().take(500).collect())
            }
        }
    }
    
    async fn synthesize_results(&self, task: &ResearchTask, plan: &ResearchPlan) -> Result<String> {
        let mut context = String::new();
        
        context.push_str(&format!("Research Topic: {}\n", plan.main_topic));
        context.push_str(&format!("Category: {}\n\n", plan.category));
        
        for result in &task.results {
            context.push_str(&format!("Source: {}\n", result.title));
            context.push_str(&format!("URL: {}\n", result.url));
            context.push_str(&format!("Content: {}\n\n", result.content));
        }
        
        let prompt = format!(
            "Synthesize the following research results into a comprehensive conclusion:\n\n{}\n\n\
            Provide a well-structured summary that:\n\
            1. Answers the original query: '{}'\n\
            2. Highlights key findings and insights\n\
            3. Provides actionable recommendations if applicable\n\
            4. Notes any gaps or areas needing further research",
            context, task.query
        );
        
        match self.llm_client.send_request(&prompt).await {
            Ok(synthesis) => Ok(synthesis),
            Err(_) => {
                // Fallback synthesis
                let fallback = format!(
                    "Research Summary for: {}\n\n\
                    Based on {} sources found:\n\n",
                    task.query, task.results.len()
                );
                
                let mut summary = fallback;
                for (i, result) in task.results.iter().enumerate() {
                    summary.push_str(&format!(
                        "{}. {} - {}\n{}\n\n",
                        i + 1,
                        result.title,
                        result.url,
                        result.content.chars().take(200).collect::<String>()
                    ));
                }
                
                Ok(summary)
            }
        }
    }
    
    async fn search_web(&self, query: &str) -> Result<Vec<SearchResult>> {
        // Use DuckDuckGo HTML API
        let encoded_query = urlencoding::encode(query);
        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);
        
        let response = reqwest::get(&url).await
            .map_err(|e| AppError::BrowserAI(format!("Search request failed: {}", e)))?;
        
        let html = response.text().await
            .map_err(|e| AppError::BrowserAI(format!("Failed to read search response: {}", e)))?;
        
        // Parse search results from HTML
        let document = scraper::Html::parse_document(&html);
        let result_selector = scraper::Selector::parse(".result").unwrap();
        let title_selector = scraper::Selector::parse(".result__a").unwrap();
        let snippet_selector = scraper::Selector::parse(".result__snippet").unwrap();
        
        let mut results = Vec::new();
        
        for (i, result) in document.select(&result_selector).enumerate() {
            if i >= 10 { break; } // Limit to 10 results
            
            let title = result.select(&title_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();
            
            let url = result.select(&title_selector)
                .next()
                .and_then(|el| el.value().attr("href"))
                .unwrap_or_default()
                .to_string();
            
            let snippet = result.select(&snippet_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();
            
            if !url.is_empty() && !title.is_empty() {
                results.push(SearchResult {
                    url: url.replace("//duckduckgo.com/l/?uddg=", ""),
                    title: title.trim().to_string(),
                    snippet: snippet.trim().to_string(),
                    relevance_score: 1.0 - (i as f32 * 0.05), // Score based on ranking
                });
            }
        }
        
        if results.is_empty() {
            // Fallback to mock results if parsing fails
            results = vec![
                SearchResult {
                    url: format!("https://en.wikipedia.org/wiki/{}", query.replace(' ', "_")),
                    title: format!("{} - Wikipedia", query),
                    snippet: format!("Information about {}", query),
                    relevance_score: 0.9,
                },
            ];
        }
        
        Ok(results)
    }
    
    fn extract_json(&self, text: &str) -> Result<String> {
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
        
        // Find JSON object in the text
        if let Some(start) = cleaned.find('{') {
            if let Some(end) = cleaned.rfind('}') {
                return Ok(cleaned[start..=end].to_string());
            }
        }
        
        Ok(cleaned.trim().to_string())
    }
    
    async fn send_progress(
        &self,
        task: &ResearchTask,
        sender: &mpsc::Sender<BrowserAIProgress>,
    ) -> Result<()> {
        let completed_subtasks = task.subtasks.iter()
            .filter(|s| matches!(s.status, TaskStatus::Completed))
            .count();
        
        let total_subtasks = task.subtasks.len();
        let percentage = if total_subtasks > 0 {
            (completed_subtasks as f32 / total_subtasks as f32) * 100.0
        } else {
            0.0
        };
        
        let progress = BrowserAIProgress {
            task_id: task.id,
            status: task.status.clone(),
            current_subtask: task.subtasks.first().map(|s| s.query.clone()),
            completed_subtasks,
            total_subtasks,
            percentage,
        };
        
        sender.send(progress).await
            .map_err(|_| AppError::BrowserAI("Failed to send progress".into()))?;
        
        Ok(())
    }
}