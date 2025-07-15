use crate::error::Result;
use scraper::{Html, Selector};

pub struct ScraperEngine {
    // Playwright will be initialized on demand
}

impl ScraperEngine {
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn scrape_url(&self, url: &str) -> Result<String> {
        // For now, using a simple HTTP client approach
        // In production, would use Playwright for JavaScript-heavy sites
        let response = reqwest::get(url).await?;
        let html = response.text().await?;
        
        // Parse HTML and extract main content
        let document = Html::parse_document(&html);
        let content = self.extract_content(&document);
        
        Ok(content)
    }
    
    fn extract_content(&self, document: &Html) -> String {
        // Try to find main content areas
        let selectors = vec![
            "main",
            "article",
            "[role='main']",
            ".content",
            "#content",
            ".post",
            ".entry-content",
        ];
        
        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    return element.text().collect::<Vec<_>>().join(" ");
                }
            }
        }
        
        // Fallback to body content
        if let Ok(body_selector) = Selector::parse("body") {
            if let Some(body) = document.select(&body_selector).next() {
                return body.text().collect::<Vec<_>>().join(" ");
            }
        }
        
        // Last resort - return all text
        document.root_element().text().collect::<Vec<_>>().join(" ")
    }
}