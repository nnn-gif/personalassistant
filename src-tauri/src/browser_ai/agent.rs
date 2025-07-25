use super::{ChromeController, ScraperEngine};
use crate::error::{AppError, Result};
use crate::llm::LlmClient;
use crate::models::{
    BrowserAIProgress, BrowserAIProgressLight, BrowserAINewResult, PhaseDetails, ResearchResult, 
    ResearchSubtask, ResearchTask, SearchResult, SubtaskProgress, TaskStatus,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct ResearchPlan {
    main_topic: String,
    category: String,
    subtopics: Vec<String>,
    search_queries: Vec<String>,
    requires_browser: bool,
    #[serde(default)]
    is_time_sensitive: bool,
}

pub struct BrowserAIAgent {
    scraper: ScraperEngine,
    chrome: ChromeController,
    active_tasks: HashMap<Uuid, ResearchTask>,
    llm_client: Arc<LlmClient>,
    browser_initialized: bool,
    last_progress_time: std::time::Instant,
}

impl BrowserAIAgent {
    pub fn new() -> Self {
        Self::with_llm_client(Arc::new(LlmClient::new()))
    }
    
    pub fn with_llm_client(llm_client: Arc<LlmClient>) -> Self {
        println!("[BrowserAIAgent] Creating new BrowserAIAgent");
        Self {
            scraper: ScraperEngine::new(),
            chrome: ChromeController::new(),
            active_tasks: HashMap::new(),
            llm_client,
            browser_initialized: false,
            last_progress_time: std::time::Instant::now(),
        }
    }

    pub async fn start_research(
        &mut self,
        query: String,
        progress_sender: mpsc::Sender<BrowserAIProgressLight>,
        result_sender: Option<mpsc::Sender<BrowserAINewResult>>,
    ) -> Result<Uuid> {
        println!("Agent: Starting research for: {query}");

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
        let _ = self
            .send_light_progress(
                &task,
                &progress_sender,
                Some("Analyzing query and creating research plan...".to_string()),
                Some(PhaseDetails {
                    phase: "Planning".to_string(),
                    details: "Breaking down your query into specific research topics".to_string(),
                    estimated_completion: None,
                }),
            )
            .await;

        println!("Agent: Creating research plan...");
        let plan = match self.create_research_plan(&query).await {
            Ok(p) => {
                println!("Agent: Research plan created successfully");
                println!("Plan: {p:?}");
                p
            }
            Err(e) => {
                println!("Agent: Error creating research plan: {e}");
                task.status = TaskStatus::Failed(format!("Failed to create plan: {e}"));
                let _ = self.send_progress(&task, &progress_sender).await;
                return Err(e);
            }
        };

        // Convert plan to subtasks
        let subtasks = self.plan_to_subtasks(&plan).await?;
        task.subtasks = subtasks;

        // Send progress with created subtasks
        let _ = self
            .send_light_progress(
                &task,
                &progress_sender,
                Some(format!("Created {} research tasks", task.subtasks.len())),
                Some(PhaseDetails {
                    phase: "Planning Complete".to_string(),
                    details: format!(
                        "Split research into {} focused queries",
                        task.subtasks.len()
                    ),
                    estimated_completion: None,
                }),
            )
            .await;

        // Step 2: Execute searches
        task.status = TaskStatus::Searching;
        let _ = self
            .send_light_progress(
                &task,
                &progress_sender,
                Some("Starting web searches...".to_string()),
                Some(PhaseDetails {
                    phase: "Searching".to_string(),
                    details: "Searching the web for relevant information".to_string(),
                    estimated_completion: None,
                }),
            )
            .await;

        let total_subtasks = task.subtasks.len();
        for i in 0..total_subtasks {
            let subtask_query = task.subtasks[i].query.clone();

            // Update progress for current search
            let _ = self
                .send_light_progress(
                    &task,
                    &progress_sender,
                    Some(format!("Searching: {subtask_query}")),
                    Some(PhaseDetails {
                        phase: format!("Search {}/{}", i + 1, total_subtasks),
                        details: format!("Finding sources for: {subtask_query}"),
                        estimated_completion: None,
                    }),
                )
                .await;

            println!("[BrowserAIAgent] Processing subtask {}/{}: {}", i + 1, total_subtasks, subtask_query);
            println!("[BrowserAIAgent] Requires browser: {}, Time sensitive: {}", plan.requires_browser, plan.is_time_sensitive);
            
            let search_results = if plan.requires_browser {
                println!("[BrowserAIAgent] Using browser for search");
                self.search_with_browser(&subtask_query).await?
            } else {
                // For time-sensitive queries, add date filtering hint to search
                let enhanced_query = if plan.is_time_sensitive {
                    format!("{} -site:wikipedia.org", subtask_query)
                } else if subtask_query.to_lowercase().contains("news") || subtask_query.to_lowercase().contains("latest") {
                    format!("{} site:news.google.com OR site:reuters.com OR site:bloomberg.com OR site:techcrunch.com", subtask_query)
                } else {
                    subtask_query.clone()
                };
                println!("[BrowserAIAgent] Using web search with query: {}", enhanced_query);
                self.search_web(&enhanced_query).await?
            };
            
            println!("[BrowserAIAgent] Found {} search results", search_results.len());

            // Update the subtask with results
            task.subtasks[i].search_results = search_results.clone();
            task.subtasks[i].status = TaskStatus::Searching;

            // Send progress after each search
            let _ = self
                .send_light_progress(
                    &task,
                    &progress_sender,
                    Some(format!(
                        "Found {} results for: {}",
                        search_results.len(),
                        subtask_query
                    )),
                    None,
                )
                .await;
        }

        // Step 3: Intelligent scraping
        task.status = TaskStatus::Scraping;
        let _ = self
            .send_light_progress(
                &task,
                &progress_sender,
                Some("Starting content extraction...".to_string()),
                Some(PhaseDetails {
                    phase: "Scraping".to_string(),
                    details: "Extracting and analyzing content from web pages".to_string(),
                    estimated_completion: None,
                }),
            )
            .await;

        let mut results = Vec::new();
        let subtasks_count = task.subtasks.len();

        for i in 0..subtasks_count {
            let subtask_id = task.subtasks[i].id;
            let subtask_query = task.subtasks[i].query.clone();
            let search_results = task.subtasks[i].search_results.clone();

            // Update subtask status
            task.subtasks[i].status = TaskStatus::Scraping;

            let _ = self
                .send_light_progress(
                    &task,
                    &progress_sender,
                    Some(format!("Extracting content for: {subtask_query}")),
                    None,
                )
                .await;

            // Prioritize top results
            let top_results: Vec<_> = search_results.iter().take(3).collect();

            for (j, search_result) in top_results.iter().enumerate() {
                let _ = self
                    .send_light_progress(
                        &task,
                        &progress_sender,
                        Some(format!(
                            "Scraping: {} ({}/{})",
                            search_result.title,
                            j + 1,
                            top_results.len()
                        )),
                        None,
                    )
                    .await;

                println!("[BrowserAIAgent] Attempting to scrape: {}", search_result.url);
                if let Ok(content) = self.scraper.scrape_url(&search_result.url).await {
                    println!("[BrowserAIAgent] Successfully scraped {} characters", content.len());
                    
                    // Extract relevant content using LLM
                    println!("[BrowserAIAgent] Extracting relevant content using LLM...");
                    let extracted = self
                        .extract_relevant_content(&content, &subtask_query)
                        .await?;
                    println!("[BrowserAIAgent] Extracted {} characters of relevant content", extracted.len());

                    let result = ResearchResult {
                        id: Uuid::new_v4(),
                        subtask_id,
                        url: search_result.url.clone(),
                        title: search_result.title.clone(),
                        content: extracted,
                        relevance_score: search_result.relevance_score,
                        scraped_at: Utc::now(),
                    };
                    results.push(result.clone());

                    // Update task with new result
                    task.results = results.clone();
                    
                    // Send new result event if sender is available
                    if let Some(ref result_sender) = result_sender {
                        let _ = result_sender.send(BrowserAINewResult {
                            task_id,
                            result: result.clone(),
                            subtask_query: subtask_query.clone(),
                        }).await;
                    }
                    
                    // Send lightweight progress
                    let _ = self
                        .send_light_progress(
                            &task,
                            &progress_sender,
                            Some(format!("Found content: {}", result.title)),
                            None,
                        )
                        .await;
                }
            }

            // Mark subtask as completed
            task.subtasks[i].status = TaskStatus::Completed;
        }

        // Step 4: Synthesize results
        task.status = TaskStatus::Analyzing;
        let _ = self
            .send_light_progress(
                &task,
                &progress_sender,
                Some("Analyzing and synthesizing findings...".to_string()),
                Some(PhaseDetails {
                    phase: "Analysis".to_string(),
                    details: format!(
                        "Analyzing {} research findings to create comprehensive conclusion",
                        task.results.len()
                    ),
                    estimated_completion: None,
                }),
            )
            .await;

        let conclusion = self.synthesize_results(&task, &plan).await?;
        task.conclusion = Some(conclusion);

        // Complete
        task.status = TaskStatus::Completed;
        task.updated_at = Utc::now();
        let _ = self
            .send_light_progress(
                &task,
                &progress_sender,
                Some("Research completed!".to_string()),
                Some(PhaseDetails {
                    phase: "Complete".to_string(),
                    details: format!(
                        "Found {} results across {} research topics",
                        task.results.len(),
                        task.subtasks.len()
                    ),
                    estimated_completion: None,
                }),
            )
            .await;

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
        // Get current date for time-sensitive queries
        let current_date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let current_year = chrono::Local::now().format("%Y").to_string();
        
        // Try to use LLM for intelligent planning
        let prompt = format!(
            "Create a research plan for the following query: '{query}'\n\
            Current date: {current_date}\n\n\
            Analyze the query and break it down into multiple focused search tasks.\n\
            IMPORTANT: If the query is about current events, latest information, or time-sensitive topics \
            (like 'latest movies', 'current news', 'new releases', 'recent events'), make sure to:\n\
            - Include the current year ({current_year}) or recent time frames in search queries\n\
            - Focus on recent and up-to-date sources\n\
            - Set category as 'news' or 'current' when appropriate\n\n\
            Return a JSON object with:\n\
            1. main_topic: The main topic being researched\n\
            2. category: Category (e.g., 'technical', 'academic', 'news', 'current', 'product', 'general')\n\
            3. subtopics: Array of 3-5 specific subtopics to research\n\
            4. search_queries: Array of 3-5 DIFFERENT optimized search queries (MUST have at least 3)\n\
               - For current/latest topics, include time qualifiers like '{current_year}', 'latest', 'new', 'recent'\n\
            5. requires_browser: Boolean indicating if interactive browser is needed\n\
            6. is_time_sensitive: Boolean indicating if this query needs current/latest information\n\n\
            IMPORTANT: You MUST provide at least 3 different search queries to cover different aspects.\n\n\
            Example for current topics:\n\
            {{\n\
              \"main_topic\": \"Latest Bollywood Movies\",\n\
              \"category\": \"current\",\n\
              \"subtopics\": [\"new releases {current_year}\", \"upcoming movies\", \"box office hits\", \"streaming releases\"],\n\
              \"search_queries\": [\n\
                \"latest bollywood movies {current_year}\",\n\
                \"new bollywood releases this month\",\n\
                \"bollywood box office collection {current_year}\",\n\
                \"upcoming bollywood movies {current_year}\",\n\
                \"best bollywood movies {current_year} so far\"\n\
              ],\n\
              \"requires_browser\": false,\n\
              \"is_time_sensitive\": true\n\
            }}\n\n\
            Example for technical topics:\n\
            {{\n\
              \"main_topic\": \"Machine Learning Algorithms\",\n\
              \"category\": \"technical\",\n\
              \"subtopics\": [\"supervised learning\", \"unsupervised learning\", \"neural networks\"],\n\
              \"search_queries\": [\n\
                \"machine learning algorithms comparison {current_year}\",\n\
                \"supervised vs unsupervised learning examples\",\n\
                \"neural network architectures guide\",\n\
                \"best ML algorithms for beginners\"\n\
              ],\n\
              \"requires_browser\": false,\n\
              \"is_time_sensitive\": false\n\
            }}\n\n\
            Now create a plan for: '{query}'"
        );

        match self.llm_client.send_request(&prompt).await {
            Ok(response) => {
                println!("[Research Plan] LLM Response: {}", response);
                match serde_json::from_str::<ResearchPlan>(&self.extract_json(&response)?) {
                    Ok(plan) => {
                        println!("[Research Plan] Successfully parsed plan with {} search queries", plan.search_queries.len());
                        Ok(plan)
                    },
                    Err(e) => {
                        println!("Failed to parse LLM response, using fallback: {e}");
                        println!("Response was: {}", response);
                        Ok(self.create_fallback_plan(query))
                    }
                }
            }
            Err(e) => {
                println!("LLM request failed, using fallback plan: {e}");
                Ok(self.create_fallback_plan(query))
            }
        }
    }

    fn create_fallback_plan(&self, query: &str) -> ResearchPlan {
        let query_lower = query.to_lowercase();
        let current_year = chrono::Local::now().format("%Y").to_string();
        
        // Check if query contains time-sensitive keywords
        let is_time_sensitive = query_lower.contains("latest") 
            || query_lower.contains("new")
            || query_lower.contains("recent")
            || query_lower.contains("current")
            || query_lower.contains("today")
            || query_lower.contains("this week")
            || query_lower.contains("this month")
            || query_lower.contains("this year")
            || query_lower.contains(&current_year);
        
        let category = if is_time_sensitive { "current" } else { "general" };
        
        // Determine if browser is needed for better results
        let requires_browser = query_lower.contains("google") 
            || query_lower.contains("search")
            || is_time_sensitive;
        
        // Create search queries with time qualifiers for time-sensitive topics
        let search_queries = if is_time_sensitive {
            vec![
                format!("{query} {current_year} -wikipedia"),
                format!("{query} latest news"),
                format!("{query} recent updates {current_year}"),
                format!("{query} trending"),
            ]
        } else {
            vec![
                format!("{query} -wikipedia"),
                format!("{query} tutorial"),
                format!("{query} guide"),
                format!("{query} examples"),
            ]
        };
        
        ResearchPlan {
            main_topic: query.to_string(),
            category: category.to_string(),
            subtopics: vec![
                format!("{query} overview"),
                format!("{query} details"),
                if is_time_sensitive { format!("{query} recent developments") } else { format!("{query} examples") },
            ],
            search_queries,
            requires_browser,
            is_time_sensitive,
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

    async fn search_with_browser(&mut self, query: &str) -> Result<Vec<SearchResult>> {
        println!("[BrowserAIAgent] Starting browser search for: {}", query);
        
        // Initialize browser if not already done
        if !self.browser_initialized {
            println!("[BrowserAIAgent] Browser not initialized, creating Chrome with LLM support...");
            self.chrome = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
            println!("[BrowserAIAgent] Launching browser...");
            self.chrome.launch_browser(false).await?;
            self.browser_initialized = true;
            println!("[BrowserAIAgent] Browser initialized successfully");
        }
        
        // Open Chrome with search query
        println!("[BrowserAIAgent] Opening Chrome with Google search...");
        self.chrome.search_google(query).await?;

        // Wait a bit for page to load
        println!("[BrowserAIAgent] Waiting 3 seconds for page to load...");
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Try to get search results from Chrome
        println!("[BrowserAIAgent] Extracting search results from Chrome...");
        match self.chrome.get_search_results().await {
            Ok(results) if !results.is_empty() => {
                println!("[BrowserAIAgent] Successfully extracted {} results from Chrome", results.len());
                let search_results: Vec<SearchResult> = results
                    .into_iter()
                    .enumerate()
                    .map(|(i, (url, title))| {
                        println!("[BrowserAIAgent] Result {}: {} - {}", i + 1, title, url);
                        SearchResult {
                            url,
                            title,
                            snippet: String::new(),
                            relevance_score: 1.0 - (i as f32 * 0.05),
                        }
                    })
                    .collect();
                Ok(search_results)
            }
            Ok(_) => {
                println!("[BrowserAIAgent] No results from Chrome, falling back to web search");
                self.search_web(query).await
            }
            Err(e) => {
                println!("[BrowserAIAgent] Error getting Chrome results: {}, falling back to web search", e);
                self.search_web(query).await
            }
        }
    }

    async fn extract_relevant_content(&self, content: &str, query: &str) -> Result<String> {
        let prompt = format!(
            "Extract the most relevant information from the following content for the query: '{query}'\n\n\
            Content:\n{}\n\n\
            Extract and summarize only the parts directly relevant to the query. \
            Keep important details, facts, and examples. Limit to 500 words.",
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
            "Synthesize the following research results into a comprehensive conclusion:\n\n{context}\n\n\
            Provide a well-structured summary that:\n\
            1. Answers the original query: '{}'\n\
            2. Highlights key findings and insights\n\
            3. Provides actionable recommendations if applicable\n\
            4. Notes any gaps or areas needing further research",
            task.query
        );

        match self.llm_client.send_request(&prompt).await {
            Ok(synthesis) => Ok(synthesis),
            Err(_) => {
                // Fallback synthesis
                let fallback = format!(
                    "Research Summary for: {}\n\n\
                    Based on {} sources found:\n\n",
                    task.query,
                    task.results.len()
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
        println!("[BrowserAIAgent] Performing web search via DuckDuckGo for: {}", query);
        
        // Use DuckDuckGo HTML API with site exclusions to get diverse results
        let enhanced_query = format!("{} -site:wikipedia.org", query);
        let encoded_query = urlencoding::encode(&enhanced_query);
        let url = format!("https://html.duckduckgo.com/html/?q={encoded_query}");
        println!("[BrowserAIAgent] DuckDuckGo URL: {}", url);

        let response = reqwest::get(&url)
            .await
            .map_err(|e| AppError::BrowserAI(format!("Search request failed: {e}")))?;

        let html = response
            .text()
            .await
            .map_err(|e| AppError::BrowserAI(format!("Failed to read search response: {e}")))?;

        // Parse search results from HTML - do all parsing before any await
        let mut results = {
            let document = scraper::Html::parse_document(&html);
            let result_selector = scraper::Selector::parse(".result").unwrap();
            let title_selector = scraper::Selector::parse(".result__a").unwrap();
            let snippet_selector = scraper::Selector::parse(".result__snippet").unwrap();

            let mut temp_results = Vec::new();

            for (i, result) in document.select(&result_selector).enumerate() {
                if i >= 10 {
                    break;
                } // Limit to 10 results

                let title = result
                    .select(&title_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default();

                let url = result
                    .select(&title_selector)
                    .next()
                    .and_then(|el| el.value().attr("href"))
                    .unwrap_or_default()
                    .to_string();

                let snippet = result
                    .select(&snippet_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default();

                if !url.is_empty() && !title.is_empty() {
                    temp_results.push(SearchResult {
                        url: url.replace("//duckduckgo.com/l/?uddg=", ""),
                        title: title.trim().to_string(),
                        snippet: snippet.trim().to_string(),
                        relevance_score: 1.0 - (i as f32 * 0.05), // Score based on ranking
                    });
                }
            }
            temp_results
        };

        println!("[BrowserAIAgent] Parsed {} results from DuckDuckGo", results.len());
        
        // Filter out duplicate domains to ensure diversity
        let mut seen_domains = std::collections::HashSet::new();
        let mut diverse_results = Vec::new();
        
        for result in results {
            if let Ok(parsed_url) = url::Url::parse(&result.url) {
                if let Some(domain) = parsed_url.domain() {
                    if seen_domains.insert(domain.to_string()) {
                        diverse_results.push(result);
                    }
                }
            }
        }
        
        results = diverse_results;
        
        if results.is_empty() {
            println!("[BrowserAIAgent] No diverse results found, trying alternative search");
            // Try a Google search via URL (won't get as good results but better than nothing)
            results = self.search_google_via_url(query).await.unwrap_or_else(|_| {
                // Last resort fallback
                vec![SearchResult {
                    url: format!("https://www.google.com/search?q={}", urlencoding::encode(query)),
                    title: format!("Search for {query}"),
                    snippet: format!("Search results for {query}"),
                    relevance_score: 0.5,
                }]
            });
        }
        
        // If we still have Wikipedia as the only result, add more diverse sources
        if results.len() == 1 && results[0].url.contains("wikipedia") {
            println!("[BrowserAIAgent] Only Wikipedia found, adding diverse sources");
            // Add some general tech sites for technical queries
            if query.to_lowercase().contains("programming") || query.to_lowercase().contains("code") || query.to_lowercase().contains("rust") {
                results.push(SearchResult {
                    url: format!("https://stackoverflow.com/search?q={}", urlencoding::encode(query)),
                    title: format!("{} - Stack Overflow", query),
                    snippet: "Community discussions and solutions".to_string(),
                    relevance_score: 0.8,
                });
                results.push(SearchResult {
                    url: format!("https://github.com/search?q={}", urlencoding::encode(query)),
                    title: format!("{} - GitHub", query),
                    snippet: "Open source projects and code examples".to_string(),
                    relevance_score: 0.7,
                });
            }
        }
        
        for (i, result) in results.iter().enumerate() {
            println!("[BrowserAIAgent] Final result {}: {} - {}", i + 1, result.title, result.url);
        }

        Ok(results)
    }

    async fn search_google_via_url(&self, query: &str) -> Result<Vec<SearchResult>> {
        println!("[BrowserAIAgent] Trying Google search via URL scraping");
        
        // Create a Google search URL
        let encoded_query = urlencoding::encode(query);
        let search_url = format!("https://www.google.com/search?q={}", encoded_query);
        
        // Try to scrape Google search results page
        match self.scraper.scrape_url(&search_url).await {
            Ok(content) => {
                // Extract URLs from the content (basic extraction)
                let mut results = Vec::new();
                
                // Look for common patterns in Google results
                if content.contains("stackoverflow.com") {
                    results.push(SearchResult {
                        url: format!("https://stackoverflow.com/search?q={}", encoded_query),
                        title: format!("{} - Stack Overflow", query),
                        snippet: "Programming Q&A and solutions".to_string(),
                        relevance_score: 0.9,
                    });
                }
                
                if content.contains("github.com") {
                    results.push(SearchResult {
                        url: format!("https://github.com/search?q={}", encoded_query),
                        title: format!("{} - GitHub", query),
                        snippet: "Open source code and projects".to_string(),
                        relevance_score: 0.85,
                    });
                }
                
                if content.contains("docs.") || content.contains("documentation") {
                    results.push(SearchResult {
                        url: format!("https://docs.rs/releases/search?query={}", encoded_query),
                        title: format!("{} - Documentation", query),
                        snippet: "Official documentation and guides".to_string(),
                        relevance_score: 0.8,
                    });
                }
                
                Ok(results)
            }
            Err(e) => {
                println!("[BrowserAIAgent] Failed to scrape Google: {}", e);
                Ok(vec![])
            }
        }
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
        &mut self,
        task: &ResearchTask,
        sender: &mpsc::Sender<BrowserAIProgressLight>,
    ) -> Result<()> {
        self.send_light_progress(task, sender, None, None).await
    }

    async fn send_light_progress(
        &mut self,
        task: &ResearchTask,
        sender: &mpsc::Sender<BrowserAIProgressLight>,
        current_operation: Option<String>,
        phase_details: Option<PhaseDetails>,
    ) -> Result<()> {
        // Throttle progress updates to max 1 per 500ms
        let now = std::time::Instant::now();
        if now.duration_since(self.last_progress_time).as_millis() < 500 && task.status != TaskStatus::Completed {
            return Ok(());
        }
        self.last_progress_time = now;

        let completed_subtasks = task
            .subtasks
            .iter()
            .filter(|s| matches!(s.status, TaskStatus::Completed))
            .count();

        let total_subtasks = task.subtasks.len();
        let percentage = if total_subtasks > 0 {
            (completed_subtasks as f32 / total_subtasks as f32) * 100.0
        } else {
            match task.status {
                TaskStatus::SplittingTasks => 10.0,
                TaskStatus::Searching => 30.0,
                TaskStatus::Scraping => 60.0,
                TaskStatus::Analyzing => 85.0,
                TaskStatus::Completed => 100.0,
                _ => 0.0,
            }
        };

        let progress = BrowserAIProgressLight {
            task_id: task.id,
            status: task.status.clone(),
            current_operation,
            percentage,
            phase: phase_details.map(|pd| pd.phase),
            completed_subtasks,
            total_subtasks,
        };

        sender
            .send(progress)
            .await
            .map_err(|_| AppError::BrowserAI("Failed to send progress".into()))?;

        Ok(())
    }

    // Keep old method for final complete status with all data
    async fn send_detailed_progress(
        &self,
        task: &ResearchTask,
        sender: &mpsc::Sender<BrowserAIProgress>,
        current_operation: Option<String>,
        phase_details: Option<PhaseDetails>,
    ) -> Result<()> {
        let completed_subtasks = task
            .subtasks
            .iter()
            .filter(|s| matches!(s.status, TaskStatus::Completed))
            .count();

        let total_subtasks = task.subtasks.len();
        let percentage = if total_subtasks > 0 {
            (completed_subtasks as f32 / total_subtasks as f32) * 100.0
        } else {
            match task.status {
                TaskStatus::SplittingTasks => 10.0,
                TaskStatus::Searching => 30.0,
                TaskStatus::Scraping => 60.0,
                TaskStatus::Analyzing => 85.0,
                TaskStatus::Completed => 100.0,
                _ => 0.0,
            }
        };

        // Create detailed subtask progress
        let subtasks_progress: Vec<SubtaskProgress> = task
            .subtasks
            .iter()
            .map(|subtask| {
                let results: Vec<ResearchResult> = task
                    .results
                    .iter()
                    .filter(|r| r.subtask_id == subtask.id)
                    .cloned()
                    .collect();

                SubtaskProgress {
                    id: subtask.id,
                    query: subtask.query.clone(),
                    status: subtask.status.clone(),
                    current_operation: None,
                    search_results_count: subtask.search_results.len(),
                    scraped_pages_count: results.len(),
                    results,
                }
            })
            .collect();

        let progress = BrowserAIProgress {
            task_id: task.id,
            status: task.status.clone(),
            current_subtask: task.subtasks.first().map(|s| s.query.clone()),
            completed_subtasks,
            total_subtasks,
            percentage,
            current_operation,
            subtasks_progress,
            intermediate_results: task.results.clone(),
            phase_details,
        };

        sender
            .send(progress)
            .await
            .map_err(|_| AppError::BrowserAI("Failed to send progress".into()))?;

        Ok(())
    }
}
