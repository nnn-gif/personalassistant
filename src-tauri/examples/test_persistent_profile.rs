use personalassistant_lib::browser_ai::ChromeController;
use personalassistant_lib::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing Persistent Chrome Profile ===\n");

    // Test 1: Launch with persistent profile
    println!("Test 1: First launch with persistent profile");
    {
        let mut controller = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
        
        println!("Launching browser with persistent profile...");
        controller.launch_browser(false).await?;
        
        println!("Browser launched successfully!");
        
        // Navigate to a site that sets cookies
        println!("Navigating to GitHub (will set cookies)...");
        controller.open_url("https://github.com").await?;
        
        // Wait a bit
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        
        // Close browser
        println!("Closing browser...");
        controller.close().await?;
        
        println!("First session complete!\n");
    }
    
    // Wait a moment between sessions
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Test 2: Launch again with same profile
    println!("Test 2: Second launch with same persistent profile");
    {
        let mut controller = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
        
        println!("Launching browser with persistent profile (should have cookies from previous session)...");
        controller.launch_browser(false).await?;
        
        println!("Browser launched successfully!");
        
        // Navigate to GitHub again - should show as logged in if you were logged in before
        println!("Navigating to GitHub (should remember previous session)...");
        controller.open_url("https://github.com").await?;
        
        // Wait to observe
        println!("\nCheck if the browser remembers your previous session (cookies, login state, etc.)");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        // Close browser
        println!("Closing browser...");
        controller.close().await?;
        
        println!("Second session complete!");
    }
    
    println!("\n=== Test Complete ===");
    println!("The persistent profile should maintain state between sessions.");
    println!("Profile location: ~/Library/Application Support/PersonalAssistant/ChromeProfile/");
    
    Ok(())
}