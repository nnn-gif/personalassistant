use personalassistant_lib::browser_ai::CdpClient;
use personalassistant_lib::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing Chrome DevTools Protocol integration...\n");

    let mut cdp = CdpClient::new();
    
    println!("Launching Chrome...");
    cdp.launch(false).await?;
    
    println!("Navigating to example.com...");
    cdp.navigate("https://example.com").await?;
    
    println!("Getting page state...");
    let page_state = cdp.get_page_state(false).await?;
    
    println!("Page Title: {}", page_state.title);
    println!("Page URL: {}", page_state.url);
    println!("Interactive elements found: {}", page_state.interactive_elements.len());
    
    println!("\nInteractive elements:");
    for elem in &page_state.interactive_elements[..5.min(page_state.interactive_elements.len())] {
        println!("  [{}] <{}> {}", elem.index, elem.tag, elem.text.as_deref().unwrap_or(""));
    }
    
    println!("\nTaking screenshot...");
    let screenshot = cdp.take_screenshot().await?;
    println!("Screenshot size: {} bytes", screenshot.len());
    
    println!("\nExecuting JavaScript...");
    let result = cdp.execute_javascript("document.querySelectorAll('a').length").await?;
    println!("Number of links on page: {}", result);
    
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    println!("\nClosing browser...");
    cdp.close().await?;
    
    println!("Test completed successfully!");
    Ok(())
}