use personalassistant_lib::browser_ai::BrowserAIAgent;
use personalassistant_lib::error::Result;
use personalassistant_lib::models::{BrowserAIProgressLight, BrowserAINewResult};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing Research After Chrome Fix ===\n");

    // Clean up any existing Chrome processes first
    println!("Cleaning up any existing Chrome processes...");
    std::process::Command::new("sh")
        .arg("-c")
        .arg("ps aux | grep -E '(chromiumoxide-runner|Chrome.*--remote-debugging-port)' | grep -v grep | awk '{print $2}' | xargs -r kill -9 2>/dev/null")
        .output()
        .ok();

    // Create agent
    let mut agent = BrowserAIAgent::new();
    
    // Create progress channels
    let (progress_tx, mut progress_rx) = mpsc::channel::<BrowserAIProgressLight>(100);
    let (result_tx, mut result_rx) = mpsc::channel::<BrowserAINewResult>(100);
    
    // Spawn task to print lightweight progress
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            println!("[Progress] {} - {}%", 
                progress.current_operation.as_ref().unwrap_or(&"Working".to_string()), 
                progress.percentage
            );
        }
    });
    
    // Spawn task to print new results
    tokio::spawn(async move {
        while let Some(result) = result_rx.recv().await {
            println!("[New Result] {} - {}", result.result.title, result.result.url);
        }
    });
    
    // Test with a simple query
    let query = "What is Rust programming language?";
    println!("Starting research for: {}\n", query);
    
    match agent.start_research(query.to_string(), progress_tx, Some(result_tx)).await {
        Ok(task_id) => {
            println!("Research started successfully! Task ID: {}", task_id);
            
            // Wait for completion
            tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
            
            // Get results
            if let Some(task) = agent.get_task(&task_id) {
                println!("\n=== Research Complete ===");
                println!("Status: {:?}", task.status);
                println!("Found {} results", task.results.len());
                
                if let Some(conclusion) = &task.conclusion {
                    println!("\nConclusion: {}", conclusion);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to start research: {}", e);
        }
    }
    
    Ok(())
}