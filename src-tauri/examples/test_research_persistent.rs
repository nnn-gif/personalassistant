use personalassistant_lib::browser_ai::BrowserAIAgent;
use personalassistant_lib::error::Result;
use personalassistant_lib::models::{BrowserAIProgressLight, BrowserAINewResult};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up env vars for the test
    std::env::set_var("GENAI_API_KEY", "test");
    std::env::set_var("OPENAI_API_KEY", "test");
    println!("=== Testing Research with Persistent Profile ===\n");
    println!("This uses a persistent Chrome profile that builds trust over time,");
    println!("reducing CAPTCHA occurrences and improving scraping reliability.\n");

    // Create agent with LLM client directly (avoids config dependency)
    let llm_client = std::sync::Arc::new(personalassistant_lib::llm::LlmClient::new());
    let mut agent = BrowserAIAgent::with_llm_client(llm_client);
    
    // Create progress channels
    let (progress_tx, mut progress_rx) = mpsc::channel::<BrowserAIProgressLight>(100);
    let (result_tx, mut result_rx) = mpsc::channel::<BrowserAINewResult>(100);
    
    // Spawn task to print progress
    let progress_handle = tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            println!("[Progress {:.0}%] {}", 
                progress.percentage,
                progress.current_operation.as_ref().unwrap_or(&"Working".to_string())
            );
        }
    });
    
    // Spawn task to collect results
    let result_handle = tokio::spawn(async move {
        let mut results = Vec::new();
        while let Some(result) = result_rx.recv().await {
            println!("[New Result] {} - {}", 
                result.result.title.chars().take(50).collect::<String>(), 
                result.result.url
            );
            results.push(result);
        }
        results
    });
    
    // Research query
    let query = "What are the best practices for Rust error handling?";
    println!("Starting research for: {}\n", query);
    
    match agent.start_research(query.to_string(), progress_tx, Some(result_tx)).await {
        Ok(task_id) => {
            println!("Research started! Task ID: {}\n", task_id);
            
            // Wait for completion (up to 30 seconds)
            let mut completed = false;
            for i in 0..30 {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                
                if let Some(task) = agent.get_task(&task_id) {
                    match &task.status {
                        personalassistant_lib::models::TaskStatus::Completed => {
                            completed = true;
                            break;
                        }
                        personalassistant_lib::models::TaskStatus::Failed(e) => {
                            eprintln!("Research failed: {}", e);
                            break;
                        }
                        _ => {
                            if i % 5 == 0 {
                                println!("Still researching... ({} seconds elapsed)", i);
                            }
                        }
                    }
                }
            }
            
            // Get final results
            if completed {
                if let Some(task) = agent.get_task(&task_id) {
                    println!("\n=== Research Complete ===");
                    println!("Found {} results\n", task.results.len());
                    
                    // Show source diversity
                    let mut domains = std::collections::HashMap::new();
                    for result in &task.results {
                        if let Ok(url) = url::Url::parse(&result.url) {
                            if let Some(domain) = url.domain() {
                                *domains.entry(domain.to_string()).or_insert(0) += 1;
                            }
                        }
                    }
                    
                    println!("Source diversity:");
                    for (domain, count) in domains {
                        println!("  {}: {} article(s)", domain, count);
                    }
                    
                    // Show conclusion
                    if let Some(conclusion) = &task.conclusion {
                        println!("\n=== AI Conclusion ===");
                        println!("{}", conclusion);
                    }
                    
                    println!("\n✅ Research completed successfully!");
                    println!("The persistent profile helps avoid CAPTCHAs and improves reliability.");
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to start research: {}", e);
            eprintln!("\nTip: Run the cleanup script if you see browser launch errors:");
            eprintln!("  ./scripts/cleanup_chrome.sh");
        }
    }
    
    // Wait for spawned tasks to complete
    drop(result_handle);
    drop(progress_handle);
    
    Ok(())
}