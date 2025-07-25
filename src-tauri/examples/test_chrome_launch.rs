use personalassistant_lib::browser_ai::ChromeController;
use personalassistant_lib::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing Chrome Launch Fix ===\n");

    // Create controller with LLM
    let mut controller = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
    
    println!("Launching browser...");
    controller.launch_browser(false).await?;
    
    println!("Browser launched successfully!");
    
    // Navigate to a test page
    println!("Navigating to Google...");
    controller.open_url("https://www.google.com").await?;
    
    println!("Navigation successful!");
    
    // Wait a bit
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Close browser
    println!("Closing browser...");
    controller.close().await?;
    
    println!("Browser closed successfully!");
    
    Ok(())
}