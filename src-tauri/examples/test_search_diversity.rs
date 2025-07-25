use personalassistant_lib::browser_ai::BrowserAIAgent;
use personalassistant_lib::error::Result;
use personalassistant_lib::models::{BrowserAIProgressLight, BrowserAINewResult};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing Search Result Diversity ===\n");

    // Create agent
    let mut agent = BrowserAIAgent::new();
    
    // Create progress channels
    let (progress_tx, mut progress_rx) = mpsc::channel::<BrowserAIProgressLight>(100);
    let (result_tx, mut result_rx) = mpsc::channel::<BrowserAINewResult>(100);
    
    // Spawn task to print lightweight progress
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            println!("Progress: {} - {}%", 
                progress.current_operation.as_ref().unwrap_or(&"Working".to_string()), 
                progress.percentage
            );
        }
    });
    
    // Spawn task to print new results as they arrive
    let result_handle = tokio::spawn(async move {
        let mut results = Vec::new();
        while let Some(result) = result_rx.recv().await {
            println!("\n[NEW RESULT] {} - {}", result.result.title, result.result.url);
            results.push(result);
        }
        results
    });
    
    // Test with a query that often returns Wikipedia-heavy results
    let query = "Rust programming language tutorial";
    println!("Starting research for: {}\n", query);
    
    let task_id = agent.start_research(query.to_string(), progress_tx, Some(result_tx)).await?;
    
    // Wait a bit for research to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    
    // Get the task results
    if let Some(task) = agent.get_task(&task_id) {
        println!("\n=== Research Results ===");
        println!("Status: {:?}", task.status);
        println!("Found {} results\n", task.results.len());
        
        // Group results by domain
        let mut domains = std::collections::HashMap::new();
        for result in &task.results {
            if let Ok(url) = url::Url::parse(&result.url) {
                if let Some(domain) = url.domain() {
                    *domains.entry(domain.to_string()).or_insert(0) += 1;
                }
            }
        }
        
        println!("Result diversity by domain:");
        for (domain, count) in domains {
            println!("  {}: {} results", domain, count);
        }
        
        println!("\nDetailed results:");
        for (i, result) in task.results.iter().enumerate() {
            println!("\n{}. {}", i + 1, result.title);
            println!("   URL: {}", result.url);
            println!("   Content preview: {}", 
                result.content.chars().take(150).collect::<String>()
            );
        }
        
        if let Some(conclusion) = &task.conclusion {
            println!("\n=== Conclusion ===");
            println!("{}", conclusion);
        }
    } else {
        println!("Task not found!");
    }
    
    Ok(())
}