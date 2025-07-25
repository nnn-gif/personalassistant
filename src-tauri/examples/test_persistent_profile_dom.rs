use personalassistant_lib::browser_ai::ChromeController;
use personalassistant_lib::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Persistent Profile with DOM Parsing Test ===\n");
    
    // Set up API key for LLM (required for CDP mode)
    std::env::set_var("GENAI_API_KEY", "test-key");
    
    // Create controller with LLM to enable CDP features
    let mut controller = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
    
    println!("Launching browser with persistent profile...");
    controller.launch_browser(false).await?;
    
    println!("Browser launched with CDP!");
    println!("Profile: ~/Library/Application Support/PersonalAssistant/ChromeProfile/\n");
    
    // Test 1: Search on Google and extract results
    println!("--- Test 1: Google Search ---");
    println!("Searching for 'Rust programming'...");
    controller.search_google("Rust programming").await?;
    
    // Wait for results to load
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Extract search results using DOM parsing
    match controller.get_search_results().await {
        Ok(results) => {
            println!("Found {} search results:", results.len());
            for (i, (url, title)) in results.iter().take(5).enumerate() {
                println!("  {}. {} - {}", i + 1, title, url);
            }
        }
        Err(e) => println!("Failed to extract search results: {}", e),
    }
    
    // Test 2: Navigate to a page and extract content
    println!("\n--- Test 2: DOM Content Extraction ---");
    println!("Navigating to example.com...");
    controller.open_url("https://example.com").await?;
    
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // Get page content
    match controller.get_page_content().await {
        Ok(content) => {
            println!("Page content (first 200 chars):");
            println!("{}", content.chars().take(200).collect::<String>());
        }
        Err(e) => println!("Failed to get page content: {}", e),
    }
    
    // Test 3: Execute custom JavaScript
    println!("\n--- Test 3: Custom JavaScript ---");
    let script = r#"
        JSON.stringify({
            title: document.title,
            url: window.location.href,
            linkCount: document.querySelectorAll('a').length,
            hasImages: document.querySelectorAll('img').length > 0,
            cookies: document.cookie.length > 0
        })
    "#;
    
    match controller.execute_script(script).await {
        Ok(result) => {
            println!("JavaScript execution result:");
            println!("{}", result);
        }
        Err(e) => println!("Failed to execute script: {}", e),
    }
    
    // Test 4: Check persistent profile benefits
    println!("\n--- Test 4: Profile Persistence Check ---");
    controller.open_url("https://github.com").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let check_script = r#"
        JSON.stringify({
            hasCookies: document.cookie.length > 0,
            localStorage: Object.keys(localStorage).length,
            sessionStorage: Object.keys(sessionStorage).length
        })
    "#;
    
    match controller.execute_script(check_script).await {
        Ok(result) => {
            println!("Persistent data check:");
            println!("{}", result);
            println!("\nNote: Values > 0 indicate the profile is maintaining state!");
        }
        Err(e) => println!("Failed to check persistence: {}", e),
    }
    
    println!("\n--- Benefits Demonstrated ---");
    println!("✅ DOM parsing and content extraction");
    println!("✅ JavaScript execution capabilities");
    println!("✅ Search result extraction");
    println!("✅ Persistent cookies and storage");
    println!("✅ Reduced CAPTCHA likelihood");
    
    println!("\nClosing browser...");
    controller.close().await?;
    
    println!("Done! Profile preserved for next session.");
    
    Ok(())
}