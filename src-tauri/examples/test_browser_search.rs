use personalassistant_lib::browser_ai::ChromeController;
use personalassistant_lib::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing Browser Search with DOM Extraction ===\n");

    // Initialize Chrome controller with LLM support
    println!("1. Creating ChromeController with LLM support...");
    let mut chrome = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
    
    // Launch browser in visible mode
    println!("\n2. Launching browser...");
    chrome.launch_browser(false).await?;
    
    // Search for something
    let query = "Rust programming language";
    println!("\n3. Searching Google for: {}", query);
    chrome.search_google(query).await?;
    
    // Wait a bit more to ensure page is fully loaded
    println!("\n4. Waiting for page to fully load...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Extract search results
    println!("\n5. Extracting search results from DOM...");
    let results = chrome.get_search_results().await?;
    
    println!("\n=== Search Results ===");
    if results.is_empty() {
        println!("No results found!");
        
        // Try to debug by getting page info
        println!("\n=== Debugging Page State ===");
        let debug_script = r#"
            JSON.stringify({
                url: window.location.href,
                title: document.title,
                hasGoogleElements: document.querySelectorAll('.g').length > 0,
                bodyTextPreview: document.body.innerText.substring(0, 200)
            })
        "#;
        
        if let Ok(debug_info) = chrome.execute_script(debug_script).await {
            println!("Page debug info: {}", debug_info);
        }
    } else {
        println!("Found {} results:", results.len());
        for (i, (url, title)) in results.iter().enumerate() {
            println!("\n{}. {}", i + 1, title);
            println!("   URL: {}", url);
        }
    }
    
    // Keep browser open for manual inspection
    println!("\n6. Keeping browser open for 10 seconds for inspection...");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    
    println!("\n7. Closing browser...");
    chrome.close().await?;
    
    println!("\nTest completed!");
    Ok(())
}