use crate::error::{AppError, Result};
use std::process::Command;

pub struct ChromeController;

impl ChromeController {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn open_url(&self, url: &str) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg("-a")
                .arg("Google Chrome")
                .arg(url)
                .output()
                .map_err(|e| AppError::BrowserAI(format!("Failed to open Chrome: {}", e)))?;
        }
        
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(&["/C", "start", "chrome", url])
                .output()
                .map_err(|e| AppError::BrowserAI(format!("Failed to open Chrome: {}", e)))?;
        }
        
        #[cfg(target_os = "linux")]
        {
            Command::new("google-chrome")
                .arg(url)
                .output()
                .map_err(|e| AppError::BrowserAI(format!("Failed to open Chrome: {}", e)))?;
        }
        
        Ok(())
    }
    
    pub async fn search_google(&self, query: &str) -> Result<()> {
        let search_url = format!("https://www.google.com/search?q={}", urlencoding::encode(query));
        self.open_url(&search_url).await
    }
    
    pub async fn execute_script(&self, script: &str) -> Result<String> {
        // This would require Chrome DevTools Protocol integration
        // For now, we'll use AppleScript on macOS as a simple solution
        
        #[cfg(target_os = "macos")]
        {
            let applescript = format!(
                r#"tell application "Google Chrome"
                    execute front window's active tab javascript "{}"
                end tell"#,
                script.replace('"', r#"\""#)
            );
            
            let output = Command::new("osascript")
                .arg("-e")
                .arg(&applescript)
                .output()
                .map_err(|e| AppError::BrowserAI(format!("Failed to execute script: {}", e)))?;
            
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(result)
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(AppError::BrowserAI("Chrome scripting not implemented for this platform".into()))
        }
    }
    
    pub async fn get_page_content(&self) -> Result<String> {
        let script = "document.body.innerText";
        self.execute_script(script).await
    }
    
    pub async fn get_search_results(&self) -> Result<Vec<(String, String)>> {
        // Extract Google search results using JavaScript
        let script = r#"
            Array.from(document.querySelectorAll('.g')).slice(0, 10).map(el => {
                const link = el.querySelector('a');
                const title = el.querySelector('h3');
                return {
                    url: link ? link.href : '',
                    title: title ? title.innerText : ''
                };
            })
        "#;
        
        let _result = self.execute_script(script).await?;
        
        // Parse the result (simplified for now)
        Ok(vec![])
    }
}