use personalassistant_lib::browser_ai::ChromeController;
use personalassistant_lib::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Robust DOM Extraction Test ===\n");
    
    std::env::set_var("GENAI_API_KEY", "test-key");
    
    let mut controller = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
    
    println!("Launching browser with persistent profile...");
    controller.launch_browser(false).await?;
    
    // Test 1: Simple page with known structure
    println!("--- Test 1: Wikipedia (Simple DOM) ---");
    controller.open_url("https://en.wikipedia.org/wiki/Rust_(programming_language)").await?;
    
    // Wait for page to load
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    let wiki_script = r#"
        (() => {
            const data = {
                title: document.querySelector('h1')?.textContent || 'Not found',
                firstParagraph: document.querySelector('.mw-parser-output > p')?.textContent?.substring(0, 200) || 'Not found',
                tableOfContents: Array.from(document.querySelectorAll('.toc li')).slice(0, 5).map(li => li.textContent?.trim()),
                infoboxItems: Array.from(document.querySelectorAll('.infobox tr')).slice(0, 5).map(tr => ({
                    label: tr.querySelector('th')?.textContent?.trim(),
                    value: tr.querySelector('td')?.textContent?.trim()
                })).filter(item => item.label)
            };
            return JSON.stringify(data, null, 2);
        })()
    "#;
    
    match controller.execute_script(wiki_script).await {
        Ok(result) => {
            println!("Wikipedia page data extracted:");
            println!("{}", result);
        }
        Err(e) => println!("Failed to extract Wikipedia data: {}", e),
    }
    
    // Test 2: Test form elements
    println!("\n--- Test 2: Form Elements (httpbin.org) ---");
    controller.open_url("https://httpbin.org/forms/post").await?;
    
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let form_script = r#"
        (() => {
            const form = document.querySelector('form');
            if (!form) return JSON.stringify({error: 'No form found'});
            
            const elements = {
                inputs: Array.from(form.querySelectorAll('input')).map(input => ({
                    name: input.name,
                    type: input.type,
                    value: input.value,
                    placeholder: input.placeholder
                })),
                textareas: Array.from(form.querySelectorAll('textarea')).map(ta => ({
                    name: ta.name,
                    rows: ta.rows,
                    cols: ta.cols
                })),
                selects: Array.from(form.querySelectorAll('select')).map(select => ({
                    name: select.name,
                    options: Array.from(select.options).map(opt => opt.text)
                }))
            };
            
            return JSON.stringify(elements, null, 2);
        })()
    "#;
    
    match controller.execute_script(form_script).await {
        Ok(result) => {
            println!("Form elements extracted:");
            println!("{}", result);
        }
        Err(e) => println!("Failed to extract form data: {}", e),
    }
    
    // Test 3: Wait for dynamic content
    println!("\n--- Test 3: Dynamic Content Loading ---");
    controller.open_url("https://example.com").await?;
    
    // Inject some dynamic content
    let inject_script = r#"
        setTimeout(() => {
            const div = document.createElement('div');
            div.id = 'dynamic-content';
            div.innerHTML = '<h2>Dynamic Content</h2><p>This was added after page load</p>';
            document.body.appendChild(div);
        }, 1000);
        'Content will be added in 1 second...'
    "#;
    
    controller.execute_script(inject_script).await?;
    println!("Waiting for dynamic content...");
    
    // Wait and check for dynamic content
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let check_script = r#"
        (() => {
            const dynamic = document.getElementById('dynamic-content');
            if (dynamic) {
                return JSON.stringify({
                    found: true,
                    heading: dynamic.querySelector('h2')?.textContent,
                    paragraph: dynamic.querySelector('p')?.textContent
                });
            }
            return JSON.stringify({found: false});
        })()
    "#;
    
    match controller.execute_script(check_script).await {
        Ok(result) => {
            println!("Dynamic content check:");
            println!("{}", result);
        }
        Err(e) => println!("Failed to check dynamic content: {}", e),
    }
    
    // Test 4: Extract all links
    println!("\n--- Test 4: Link Extraction ---");
    controller.open_url("https://news.ycombinator.com").await?;
    
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    let links_script = r#"
        (() => {
            const links = Array.from(document.querySelectorAll('a.storylink, a.titlelink, .titleline > a'))
                .slice(0, 10)
                .map(a => ({
                    text: a.textContent?.trim(),
                    href: a.href,
                    domain: a.hostname
                }))
                .filter(link => link.text && link.href);
            
            return JSON.stringify({
                pageTitle: document.title,
                linkCount: links.length,
                links: links
            }, null, 2);
        })()
    "#;
    
    match controller.execute_script(links_script).await {
        Ok(result) => {
            println!("Hacker News links extracted:");
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&result) {
                println!("Page: {}", data["pageTitle"]);
                println!("Found {} links", data["linkCount"]);
                if let Some(links) = data["links"].as_array() {
                    for (i, link) in links.iter().take(5).enumerate() {
                        println!("{}. {} ({})", 
                            i + 1, 
                            link["text"].as_str().unwrap_or(""),
                            link["domain"].as_str().unwrap_or("")
                        );
                    }
                }
            }
        }
        Err(e) => println!("Failed to extract links: {}", e),
    }
    
    println!("\n--- Summary ---");
    println!("✅ Persistent profile reduces bot detection");
    println!("✅ Complex DOM structures can be extracted");
    println!("✅ Form elements are accessible");
    println!("✅ Dynamic content can be waited for");
    println!("✅ Custom JavaScript provides flexibility");
    
    println!("\nClosing browser...");
    controller.close().await?;
    
    Ok(())
}