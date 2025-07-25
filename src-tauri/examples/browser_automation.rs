use personalassistant_lib::browser_ai::ChromeController;
use personalassistant_lib::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Chrome Browser Automation Examples\n");

    // Example 1: Basic browser control without LLM
    basic_browser_control().await?;

    // Example 2: LLM-driven browser automation
    llm_browser_automation().await?;

    // Example 3: Smart form filling
    smart_form_filling().await?;

    // Example 4: Web scraping with LLM
    web_scraping_with_llm().await?;

    Ok(())
}

async fn basic_browser_control() -> Result<()> {
    println!("=== Example 1: Basic Browser Control ===");
    
    let mut chrome = ChromeController::new();
    
    // Launch browser in non-headless mode
    chrome.launch_browser(false).await?;
    
    // Navigate to a website
    chrome.open_url("https://example.com").await?;
    
    // Wait a bit to see the page
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Close the browser
    chrome.close().await?;
    
    println!("Basic browser control completed!\n");
    Ok(())
}

async fn llm_browser_automation() -> Result<()> {
    println!("=== Example 2: LLM-driven Browser Automation ===");
    
    // Initialize with your preferred LLM model
    // Make sure to set the appropriate API key in your environment
    let mut chrome = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
    
    // Launch browser
    chrome.launch_browser(false).await?;
    
    // Execute a natural language task
    chrome.execute_task(
        "Go to Google and search for 'Rust programming language'"
    ).await?;
    
    // Wait to see results
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    
    chrome.close().await?;
    
    println!("LLM-driven automation completed!\n");
    Ok(())
}

async fn smart_form_filling() -> Result<()> {
    println!("=== Example 3: Smart Form Filling ===");
    
    let mut chrome = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
    chrome.launch_browser(false).await?;
    
    // Navigate to a form page (example)
    chrome.open_url("https://www.w3schools.com/html/html_forms.asp").await?;
    
    // Fill form using natural language description
    chrome.fill_form_smart(r#"
        First name: John
        Last name: Doe
        Submit the form
    "#).await?;
    
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    chrome.close().await?;
    
    println!("Smart form filling completed!\n");
    Ok(())
}

async fn web_scraping_with_llm() -> Result<()> {
    println!("=== Example 4: Web Scraping with LLM ===");
    
    let mut chrome = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
    chrome.launch_browser(true).await?; // Headless mode for scraping
    
    // Navigate to a news website
    chrome.open_url("https://news.ycombinator.com").await?;
    
    // Extract structured data using natural language
    let data = chrome.extract_structured_data(r#"
        Extract the top 5 news items with:
        - title
        - points (upvotes)
        - number of comments
        - link URL
    "#).await?;
    
    println!("Extracted data:");
    println!("{}", serde_json::to_string_pretty(&data)?);
    
    chrome.close().await?;
    
    println!("\nWeb scraping completed!\n");
    Ok(())
}

// Additional example: Complex multi-step automation
#[allow(dead_code)]
async fn complex_automation_example() -> Result<()> {
    let mut chrome = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
    
    // Enable vision mode for better understanding of complex pages
    chrome.set_vision_mode(true);
    
    chrome.launch_browser(false).await?;
    
    // Complex multi-step task
    chrome.execute_task(r#"
        1. Go to Amazon.com
        2. Search for "wireless headphones"
        3. Filter results to show only items under $50
        4. Sort by customer ratings
        5. Take a screenshot of the results
    "#).await?;
    
    chrome.close().await?;
    Ok(())
}