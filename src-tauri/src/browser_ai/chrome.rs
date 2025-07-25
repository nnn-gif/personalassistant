use crate::browser_ai::cdp_client::{CdpClient, PageState};
use crate::error::{AppError, Result};
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client as GenAiClient;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserAction {
    Click { element_index: usize },
    Type { element_index: usize, text: String },
    Select { element_index: usize, value: String },
    Navigate { url: String },
    Wait { selector: String, timeout_secs: u64 },
    Screenshot,
    Complete { message: String },
}

pub struct ChromeController {
    cdp_client: CdpClient,
    llm_client: Option<GenAiClient>,
    use_vision: bool,
}

impl ChromeController {
    pub fn new() -> Self {
        println!("[ChromeController] Creating new ChromeController without LLM");
        Self {
            cdp_client: CdpClient::new(),
            llm_client: None,
            use_vision: false,
        }
    }

    pub async fn with_llm(model_name: &str) -> Result<Self> {
        println!("[ChromeController] Creating ChromeController with LLM model: {}", model_name);
        let llm_client = GenAiClient::default();

        Ok(Self {
            cdp_client: CdpClient::new(),
            llm_client: Some(llm_client),
            use_vision: false,
        })
    }
    
    pub async fn with_temporary_profile(model_name: &str) -> Result<Self> {
        println!("[ChromeController] Creating ChromeController with temporary profile");
        let llm_client = GenAiClient::default();

        Ok(Self {
            cdp_client: CdpClient::with_temporary_profile(),
            llm_client: Some(llm_client),
            use_vision: false,
        })
    }

    pub fn set_vision_mode(&mut self, enabled: bool) {
        self.use_vision = enabled;
    }

    pub async fn launch_browser(&mut self, headless: bool) -> Result<()> {
        println!("[ChromeController] Launching browser (headless: {})", headless);
        self.cdp_client.launch(headless).await?;
        println!("[ChromeController] Browser launched successfully");
        Ok(())
    }

    pub async fn connect_to_browser(&mut self, ws_url: &str) -> Result<()> {
        self.cdp_client.connect_to_existing(ws_url).await
    }

    pub async fn open_url(&mut self, url: &str) -> Result<()> {
        if self.llm_client.is_some() {
            println!("[ChromeController] Navigating to URL using CDP: {}", url);
            self.cdp_client.navigate(url).await?;
            println!("[ChromeController] Navigation complete");
            Ok(())
        } else {
            println!("[ChromeController] Opening URL using native method: {}", url);
            self.open_url_native(url).await
        }
    }

    async fn open_url_native(&self, url: &str) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg("-a")
                .arg("Google Chrome")
                .arg(url)
                .output()
                .map_err(|e| AppError::BrowserAI(format!("Failed to open Chrome: {}", e)))?;
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(&["/C", "start", "chrome", url])
                .output()
                .map_err(|e| AppError::BrowserAI(format!("Failed to open Chrome: {}", e)))?;
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("google-chrome")
                .arg(url)
                .output()
                .map_err(|e| AppError::BrowserAI(format!("Failed to open Chrome: {}", e)))?;
        }

        Ok(())
    }

    pub async fn search_google(&mut self, query: &str) -> Result<()> {
        println!("[ChromeController] Searching Google for: {}", query);
        let search_url = format!(
            "https://www.google.com/search?q={}",
            urlencoding::encode(query)
        );
        println!("[ChromeController] Opening URL: {}", search_url);
        self.open_url(&search_url).await?;
        
        // Wait for search results to load
        if self.llm_client.is_some() {
            println!("[ChromeController] Waiting for search results to load...");
            
            // Try to wait for search results container
            let wait_script = r#"
                (() => {
                    // Check if search results are present
                    const results = document.querySelectorAll('div.g');
                    const hasResults = results.length > 0;
                    const hasH3 = document.querySelectorAll('h3').length > 0;
                    return hasResults || hasH3;
                })()
            "#;
            
            // Poll for results up to 5 seconds
            let start = std::time::Instant::now();
            loop {
                if let Ok(has_results) = self.cdp_client.execute_javascript(wait_script).await {
                    if has_results.as_bool().unwrap_or(false) {
                        println!("[ChromeController] Search results detected");
                        break;
                    }
                }
                
                if start.elapsed().as_secs() > 5 {
                    println!("[ChromeController] Timeout waiting for search results");
                    break;
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            
            // Additional wait to ensure everything is rendered
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        Ok(())
    }

    pub async fn execute_script(&self, script: &str) -> Result<String> {
        if self.llm_client.is_some() {
            let result = self.cdp_client.execute_javascript(script).await?;
            Ok(result.to_string())
        } else {
            self.execute_script_native(script).await
        }
    }

    async fn execute_script_native(&self, script: &str) -> Result<String> {
        #[cfg(target_os = "macos")]
        {
            let applescript = format!(
                r#"tell application "Google Chrome"
                    execute front window's active tab javascript "{}"
                end tell"#,
                script.replace('"', r#"\""#)
            );

            let output = Command::new("osascript")
                .arg("-e")
                .arg(&applescript)
                .output()
                .map_err(|e| AppError::BrowserAI(format!("Failed to execute script: {}", e)))?;

            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(result)
        }

        #[cfg(not(target_os = "macos"))]
        {
            Err(AppError::BrowserAI(
                "Chrome scripting not implemented for this platform".into(),
            ))
        }
    }

    pub async fn execute_task(&mut self, task_description: &str) -> Result<()> {
        println!("[ChromeController] Starting LLM-driven task: {}", task_description);
        
        let llm_client = self.llm_client.as_ref()
            .ok_or_else(|| AppError::BrowserAI("LLM client not initialized".into()))?;

        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 50;

        while iteration < MAX_ITERATIONS {
            let page_state = self.cdp_client.get_page_state(self.use_vision).await?;
            let action = self.get_next_action(llm_client, task_description, &page_state).await?;

            match action {
                BrowserAction::Click { element_index } => {
                    self.cdp_client.click_element(element_index).await?;
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                BrowserAction::Type { element_index, text } => {
                    self.cdp_client.type_text(element_index, &text, 50).await?;
                }
                BrowserAction::Select { element_index, value } => {
                    self.cdp_client.select_option(element_index, &value).await?;
                }
                BrowserAction::Navigate { url } => {
                    self.cdp_client.navigate(&url).await?;
                }
                BrowserAction::Wait { selector, timeout_secs } => {
                    self.cdp_client.wait_for_selector(&selector, timeout_secs).await?;
                }
                BrowserAction::Screenshot => {
                    let screenshot = self.cdp_client.take_screenshot().await?;
                    // Save or process screenshot as needed
                    println!("Screenshot taken: {} bytes", screenshot.len());
                }
                BrowserAction::Complete { message } => {
                    println!("Task completed: {}", message);
                    return Ok(());
                }
            }

            iteration += 1;
        }

        Err(AppError::BrowserAI("Task execution reached maximum iterations".into()))
    }

    async fn get_next_action(
        &self,
        llm_client: &GenAiClient,
        task_description: &str,
        page_state: &PageState,
    ) -> Result<BrowserAction> {
        let mut prompt = format!(
            "You are a browser automation assistant. Your task is: {}\n\n",
            task_description
        );
        prompt.push_str(&page_state.to_llm_prompt());
        prompt.push_str("\n\nBased on the current page state and the task, what is the next action to take?\n");
        prompt.push_str("Respond with a JSON object representing one of these actions:\n");
        prompt.push_str(r#"- {"action": "click", "element_index": <number>}"#);
        prompt.push_str("\n");
        prompt.push_str(r#"- {"action": "type", "element_index": <number>, "text": "<text>"}"#);
        prompt.push_str("\n");
        prompt.push_str(r#"- {"action": "select", "element_index": <number>, "value": "<value>"}"#);
        prompt.push_str("\n");
        prompt.push_str(r#"- {"action": "navigate", "url": "<url>"}"#);
        prompt.push_str("\n");
        prompt.push_str(r#"- {"action": "wait", "selector": "<selector>", "timeout_secs": <number>}"#);
        prompt.push_str("\n");
        prompt.push_str(r#"- {"action": "screenshot"}"#);
        prompt.push_str("\n");
        prompt.push_str(r#"- {"action": "complete", "message": "<completion message>"}"#);
        prompt.push_str("\n\nOnly respond with the JSON object, no explanation.");

        let chat_req = ChatRequest::new(vec![ChatMessage::user(prompt)]);
        let response = llm_client
            .exec_chat("claude-3-5-sonnet-20241022", chat_req, None)
            .await
            .map_err(|e| AppError::BrowserAI(format!("LLM request failed: {}", e)))?;

        let full_response = response
            .content_text_as_str()
            .ok_or_else(|| AppError::BrowserAI("Empty response from LLM".into()))?
            .to_string();

        let action_json: serde_json::Value = serde_json::from_str(&full_response)
            .map_err(|e| AppError::BrowserAI(format!("Failed to parse LLM response: {}. Response: {}", e, full_response)))?;

        let action = match action_json["action"].as_str() {
            Some("click") => BrowserAction::Click {
                element_index: action_json["element_index"].as_u64().unwrap_or(0) as usize,
            },
            Some("type") => BrowserAction::Type {
                element_index: action_json["element_index"].as_u64().unwrap_or(0) as usize,
                text: action_json["text"].as_str().unwrap_or("").to_string(),
            },
            Some("select") => BrowserAction::Select {
                element_index: action_json["element_index"].as_u64().unwrap_or(0) as usize,
                value: action_json["value"].as_str().unwrap_or("").to_string(),
            },
            Some("navigate") => BrowserAction::Navigate {
                url: action_json["url"].as_str().unwrap_or("").to_string(),
            },
            Some("wait") => BrowserAction::Wait {
                selector: action_json["selector"].as_str().unwrap_or("").to_string(),
                timeout_secs: action_json["timeout_secs"].as_u64().unwrap_or(10),
            },
            Some("screenshot") => BrowserAction::Screenshot,
            Some("complete") => BrowserAction::Complete {
                message: action_json["message"].as_str().unwrap_or("Task completed").to_string(),
            },
            _ => return Err(AppError::BrowserAI("Invalid action from LLM".into())),
        };

        Ok(action)
    }

    pub async fn fill_form_smart(&mut self, form_data: &str) -> Result<()> {
        let task = format!(
            "Fill out the form on this page with the following information:\n{}\n\n\
            Make sure to fill all the required fields accurately.",
            form_data
        );
        self.execute_task(&task).await
    }

    pub async fn extract_structured_data(&mut self, extraction_prompt: &str) -> Result<serde_json::Value> {
        let llm_client = self.llm_client.as_ref()
            .ok_or_else(|| AppError::BrowserAI("LLM client not initialized".into()))?;

        let page_state = self.cdp_client.get_page_state(false).await?;
        
        let prompt = format!(
            "Extract the following information from this web page:\n{}\n\n\
            Page content:\n{}\n\n\
            Return the extracted data as a valid JSON object. Only respond with the JSON, no explanation.",
            extraction_prompt,
            page_state.to_llm_prompt()
        );

        let chat_req = ChatRequest::new(vec![ChatMessage::user(prompt)]);
        let response = llm_client
            .exec_chat("claude-3-5-sonnet-20241022", chat_req, None)
            .await
            .map_err(|e| AppError::BrowserAI(format!("LLM request failed: {}", e)))?;

        let full_response = response
            .content_text_as_str()
            .ok_or_else(|| AppError::BrowserAI("Empty response from LLM".into()))?
            .to_string();

        serde_json::from_str(&full_response)
            .map_err(|e| AppError::BrowserAI(format!("Failed to parse extracted data: {}. Response: {}", e, full_response)))
    }

    pub async fn get_page_content(&self) -> Result<String> {
        let script = "document.body.innerText";
        self.execute_script(script).await
    }

    pub async fn get_search_results(&mut self) -> Result<Vec<(String, String)>> {
        println!("[ChromeController] Extracting search results from page...");
        
        if self.llm_client.is_some() {
            // Use CDP to extract search results directly from DOM
            println!("[ChromeController] Using CDP to extract search results");
            
            // First, let's get the page state to see what we're working with
            let page_state = self.cdp_client.get_page_state(false).await?;
            println!("[ChromeController] Page title: {}", page_state.title);
            println!("[ChromeController] Found {} interactive elements", page_state.interactive_elements.len());
            
            // Extract search results using JavaScript
            let script = r#"
                (() => {
                    const results = [];
                    
                    // Method 1: Try the standard Google search result structure
                    const searchResults = document.querySelectorAll('div.g');
                    console.log('Found div.g elements:', searchResults.length);
                    
                    searchResults.forEach(el => {
                        // Look for links within the result
                        const linkEl = el.querySelector('a[href]');
                        const titleEl = el.querySelector('h3');
                        
                        if (linkEl && titleEl && linkEl.href && !linkEl.href.includes('google.com')) {
                            results.push({
                                url: linkEl.href,
                                title: titleEl.innerText || titleEl.textContent
                            });
                        }
                    });
                    
                    // Method 2: If no results, try alternative selectors
                    if (results.length === 0) {
                        console.log('Trying alternative selectors...');
                        
                        // Try to find all h3 elements and their parent links
                        document.querySelectorAll('h3').forEach(h3 => {
                            const parentLink = h3.closest('a[href]');
                            if (parentLink && parentLink.href && !parentLink.href.includes('google.com')) {
                                results.push({
                                    url: parentLink.href,
                                    title: h3.innerText || h3.textContent
                                });
                            }
                        });
                    }
                    
                    // Method 3: Look for cite elements which often contain URLs
                    if (results.length === 0) {
                        console.log('Trying cite elements...');
                        document.querySelectorAll('cite').forEach(cite => {
                            const parent = cite.closest('div.g') || cite.closest('[data-sokoban-container]');
                            if (parent) {
                                const link = parent.querySelector('a[href]');
                                const title = parent.querySelector('h3');
                                if (link && title && link.href && !link.href.includes('google.com')) {
                                    results.push({
                                        url: link.href,
                                        title: title.innerText || title.textContent
                                    });
                                }
                            }
                        });
                    }
                    
                    // Remove duplicates
                    const uniqueResults = [];
                    const seenUrls = new Set();
                    
                    results.forEach(result => {
                        if (!seenUrls.has(result.url)) {
                            seenUrls.add(result.url);
                            uniqueResults.push(result);
                        }
                    });
                    
                    console.log('Total unique results found:', uniqueResults.length);
                    return uniqueResults.slice(0, 10);
                })()
            "#;

            let result = self.cdp_client.execute_javascript(script).await?;
            println!("[ChromeController] JavaScript execution result: {:?}", result);
            
            if let Some(results_array) = result.as_array() {
                let results: Vec<(String, String)> = results_array.iter()
                    .filter_map(|item| {
                        let url = item["url"].as_str()?.to_string();
                        let title = item["title"].as_str()?.to_string();
                        println!("[ChromeController] Found result: {} - {}", title, url);
                        Some((url, title))
                    })
                    .collect();
                
                println!("[ChromeController] Extracted {} search results", results.len());
                
                if results.is_empty() {
                    println!("[ChromeController] No results found, dumping page info for debugging");
                    // Get page content for debugging
                    let debug_script = r#"
                        JSON.stringify({
                            url: window.location.href,
                            title: document.title,
                            bodyText: document.body.innerText.substring(0, 500),
                            gElements: document.querySelectorAll('.g').length,
                            h3Elements: document.querySelectorAll('h3').length,
                            linkElements: document.querySelectorAll('a').length
                        })
                    "#;
                    let debug_info = self.cdp_client.execute_javascript(debug_script).await?;
                    println!("[ChromeController] Page debug info: {}", debug_info);
                }
                
                Ok(results)
            } else {
                println!("[ChromeController] Failed to parse results as array");
                Ok(vec![])
            }
        } else {
            // Fallback to native script execution
            println!("[ChromeController] Using native script execution (AppleScript)");
            let script = r#"
                Array.from(document.querySelectorAll('.g')).slice(0, 10).map(el => {
                    const link = el.querySelector('a');
                    const title = el.querySelector('h3');
                    return {
                        url: link ? link.href : '',
                        title: title ? title.innerText : ''
                    };
                })
            "#;

            let result = self.execute_script(script).await?;
            println!("[ChromeController] Native script result: {}", result);
            
            // Try to parse the result
            match serde_json::from_str::<Vec<serde_json::Value>>(&result) {
                Ok(results_array) => {
                    let results: Vec<(String, String)> = results_array.iter()
                        .filter_map(|item| {
                            let url = item["url"].as_str()?.to_string();
                            let title = item["title"].as_str()?.to_string();
                            if !url.is_empty() && !title.is_empty() {
                                Some((url, title))
                            } else {
                                None
                            }
                        })
                        .collect();
                    Ok(results)
                }
                Err(e) => {
                    println!("[ChromeController] Failed to parse native script result: {}", e);
                    Ok(vec![])
                }
            }
        }
    }

    pub async fn close(&mut self) -> Result<()> {
        self.cdp_client.close().await
    }
}
