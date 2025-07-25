use personalassistant_lib::browser_ai::ChromeController;
use personalassistant_lib::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing Persistent Profile with CAPTCHA-prone Sites ===\n");

    // Sites that often show CAPTCHAs for new browsers
    let test_sites = vec![
        ("Google Search", "https://www.google.com/search?q=rust+programming"),
        ("Reddit", "https://www.reddit.com"),
        ("Amazon", "https://www.amazon.com"),
    ];

    println!("Testing with persistent profile (should reduce CAPTCHA occurrences):\n");

    let mut controller = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
    
    println!("Launching browser with persistent profile...");
    controller.launch_browser(false).await?;
    
    for (name, url) in test_sites {
        println!("\n--- Testing {} ---", name);
        println!("Navigating to: {}", url);
        
        controller.open_url(url).await?;
        
        // Wait for page to load
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        
        // Try to get page content to check for CAPTCHA
        match controller.get_page_content().await {
            Ok(content) => {
                let content_lower = content.to_lowercase();
                
                // Check for common CAPTCHA indicators
                if content_lower.contains("captcha") || 
                   content_lower.contains("verify you're human") ||
                   content_lower.contains("i'm not a robot") ||
                   content_lower.contains("unusual traffic") {
                    println!("⚠️  CAPTCHA detected on {}", name);
                } else {
                    println!("✅ No CAPTCHA detected on {}", name);
                }
            }
            Err(e) => {
                println!("Failed to get page content: {}", e);
            }
        }
        
        // Wait a bit before next site
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    
    println!("\n--- Test Complete ---");
    println!("With persistent profile, you should see fewer CAPTCHAs over time.");
    println!("The profile builds trust as it accumulates browsing history.\n");
    
    // Wait before closing
    println!("Browser will close in 5 seconds...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    
    controller.close().await?;
    
    Ok(())
}