use personalassistant_lib::browser_ai::ChromeController;
use personalassistant_lib::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Simple Persistent Profile Test ===\n");
    
    // Test navigation without research/LLM
    // We need to create a controller with LLM to enable CDP features
    let mut controller = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
    
    println!("Launching browser with persistent profile...");
    controller.launch_browser(false).await?;
    
    println!("Browser launched!");
    println!("\nProfile location: ~/Library/Application Support/PersonalAssistant/ChromeProfile/");
    println!("This profile will persist cookies, cache, and browsing history between sessions.");
    
    // Test with a few sites
    let sites = vec![
        ("Google", "https://www.google.com"),
        ("Stack Overflow", "https://stackoverflow.com"),
        ("GitHub", "https://github.com"),
    ];
    
    for (name, url) in sites {
        println!("\nNavigating to {}...", name);
        controller.open_url(url).await?;
        
        // Get page content to verify no CAPTCHA
        match controller.get_page_content().await {
            Ok(content) => {
                let content_lower = content.to_lowercase();
                if content_lower.contains("captcha") || content_lower.contains("verify") {
                    println!("⚠️  Possible CAPTCHA on {}", name);
                } else {
                    println!("✅ Page loaded successfully");
                }
            }
            Err(e) => println!("Failed to check content: {}", e),
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    
    println!("\n--- Benefits of Persistent Profile ---");
    println!("1. Maintains cookies between sessions");
    println!("2. Builds browsing history over time");
    println!("3. Reduces CAPTCHA occurrences");
    println!("4. Preserves login states");
    println!("5. Improves site trust scores");
    
    println!("\nClosing browser...");
    controller.close().await?;
    
    println!("Done! The profile has been preserved for next time.");
    
    Ok(())
}