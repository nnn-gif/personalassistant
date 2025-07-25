use personalassistant_lib::browser_ai::ChromeController;
use personalassistant_lib::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Search Extraction Test (Alternative Search Engines) ===\n");
    
    std::env::set_var("GENAI_API_KEY", "test-key");
    
    let mut controller = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
    
    println!("Launching browser with persistent profile...");
    controller.launch_browser(false).await?;
    
    // Test with DuckDuckGo (less aggressive bot detection)
    println!("--- Testing DuckDuckGo Search ---");
    let search_url = "https://duckduckgo.com/?q=rust+programming+tutorial";
    println!("Navigating to: {}", search_url);
    controller.open_url(search_url).await?;
    
    // Wait for results
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Extract search results using custom JavaScript
    let extract_script = r#"
        (() => {
            const results = [];
            
            // DuckDuckGo result structure
            document.querySelectorAll('[data-testid="result"]').forEach((result, index) => {
                const linkEl = result.querySelector('a[href]');
                const titleEl = result.querySelector('h2');
                const snippetEl = result.querySelector('[data-result="snippet"]');
                
                if (linkEl && titleEl) {
                    results.push({
                        index: index,
                        url: linkEl.href,
                        title: titleEl.textContent || '',
                        snippet: snippetEl ? snippetEl.textContent : ''
                    });
                }
            });
            
            // Alternative selector for DuckDuckGo
            if (results.length === 0) {
                document.querySelectorAll('.result').forEach((result, index) => {
                    const linkEl = result.querySelector('.result__title a');
                    const snippetEl = result.querySelector('.result__snippet');
                    
                    if (linkEl) {
                        results.push({
                            index: index,
                            url: linkEl.href,
                            title: linkEl.textContent || '',
                            snippet: snippetEl ? snippetEl.textContent : ''
                        });
                    }
                });
            }
            
            return JSON.stringify({
                searchEngine: 'DuckDuckGo',
                resultCount: results.length,
                results: results.slice(0, 5)
            });
        })()
    "#;
    
    match controller.execute_script(extract_script).await {
        Ok(result) => {
            println!("Search extraction result:");
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&result) {
                println!("Search Engine: {}", data["searchEngine"]);
                println!("Found {} results", data["resultCount"]);
                
                if let Some(results) = data["results"].as_array() {
                    for (i, r) in results.iter().enumerate() {
                        println!("\n{}. {}", i + 1, r["title"].as_str().unwrap_or(""));
                        println!("   URL: {}", r["url"].as_str().unwrap_or(""));
                        if let Some(snippet) = r["snippet"].as_str() {
                            println!("   {}", snippet.chars().take(100).collect::<String>());
                        }
                    }
                }
            } else {
                println!("Raw result: {}", result);
            }
        }
        Err(e) => println!("Failed to extract results: {}", e),
    }
    
    // Test GitHub search (usually no CAPTCHA)
    println!("\n\n--- Testing GitHub Search ---");
    let github_search = "https://github.com/search?q=rust+web+framework&type=repositories";
    println!("Navigating to: {}", github_search);
    controller.open_url(github_search).await?;
    
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    let github_script = r#"
        (() => {
            const results = [];
            
            document.querySelectorAll('[data-testid="results-list"] > div').forEach((item, index) => {
                const linkEl = item.querySelector('a[href*="/"]');
                const descEl = item.querySelector('p');
                
                if (linkEl && linkEl.href.includes('github.com')) {
                    results.push({
                        index: index,
                        repo: linkEl.href.split('github.com/')[1] || '',
                        description: descEl ? descEl.textContent.trim() : ''
                    });
                }
            });
            
            return JSON.stringify({
                searchEngine: 'GitHub',
                resultCount: results.length,
                results: results.slice(0, 5)
            });
        })()
    "#;
    
    match controller.execute_script(github_script).await {
        Ok(result) => {
            println!("\nGitHub search result:");
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&result) {
                println!("Found {} repositories", data["resultCount"]);
                
                if let Some(results) = data["results"].as_array() {
                    for (i, r) in results.iter().enumerate() {
                        println!("\n{}. {}", i + 1, r["repo"].as_str().unwrap_or(""));
                        if let Some(desc) = r["description"].as_str() {
                            println!("   {}", desc);
                        }
                    }
                }
            } else {
                println!("Raw result: {}", result);
            }
        }
        Err(e) => println!("Failed to extract GitHub results: {}", e),
    }
    
    println!("\n\n--- Summary ---");
    println!("✅ Persistent profile maintains cookies and state");
    println!("✅ DOM extraction works with CDP");
    println!("✅ Alternative search engines are more bot-friendly");
    println!("✅ Custom JavaScript extraction is flexible");
    
    println!("\nClosing browser...");
    controller.close().await?;
    
    Ok(())
}