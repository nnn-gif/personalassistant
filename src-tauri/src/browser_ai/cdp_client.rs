use crate::error::{AppError, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::{CaptureScreenshotFormat, CaptureScreenshotParams};
use chromiumoxide::page::Page;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use std::path::Path;
use std::fs;
use chrono;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomElement {
    pub index: usize,
    pub tag: String,
    pub text: Option<String>,
    pub attributes: HashMap<String, String>,
    pub is_interactive: bool,
    pub is_visible: bool,
    pub selector: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageState {
    pub url: String,
    pub title: String,
    pub interactive_elements: Vec<DomElement>,
    pub screenshot: Option<Vec<u8>>,
    pub console_logs: Vec<String>,
}

impl PageState {
    pub fn to_llm_prompt(&self) -> String {
        let mut prompt = format!(
            "Current Page: {}\nURL: {}\n\nInteractive Elements:\n",
            self.title, self.url
        );

        for elem in &self.interactive_elements {
            if elem.is_interactive && elem.is_visible {
                let text = elem.text.as_deref().unwrap_or("");
                let mut attrs = String::new();
                
                if elem.attributes.contains_key("placeholder") {
                    attrs.push_str(&format!(" placeholder=\"{}\"", elem.attributes["placeholder"]));
                }
                if elem.attributes.contains_key("value") && !elem.attributes["value"].is_empty() {
                    attrs.push_str(&format!(" value=\"{}\"", elem.attributes["value"]));
                }
                if elem.attributes.contains_key("type") {
                    attrs.push_str(&format!(" type=\"{}\"", elem.attributes["type"]));
                }
                if elem.attributes.contains_key("name") {
                    attrs.push_str(&format!(" name=\"{}\"", elem.attributes["name"]));
                }
                
                prompt.push_str(&format!("[{}] <{}{}> {}\n", 
                    elem.index, 
                    elem.tag,
                    attrs,
                    text
                ));
            }
        }
        
        prompt
    }
}

pub struct CdpClient {
    browser: Option<Browser>,
    page: Option<Page>,
    user_data_dir: Option<String>,
    use_persistent_profile: bool,
}

impl CdpClient {
    pub fn new() -> Self {
        println!("[CdpClient] Creating new CDP client");
        Self {
            browser: None,
            page: None,
            user_data_dir: None,
            use_persistent_profile: true, // Default to persistent profile
        }
    }
    
    pub fn with_temporary_profile() -> Self {
        println!("[CdpClient] Creating CDP client with temporary profile");
        Self {
            browser: None,
            page: None,
            user_data_dir: None,
            use_persistent_profile: false,
        }
    }

    pub async fn launch(&mut self, headless: bool) -> Result<()> {
        println!("[CdpClient] Launching Chrome with headless={}", headless);
        
        // Determine user data directory
        let user_data_dir = if self.use_persistent_profile {
            self.get_persistent_profile_dir()?
        } else {
            // Use temporary directory for one-off sessions
            let timestamp = chrono::Utc::now().timestamp_millis();
            format!("/tmp/chromiumoxide-session-{}", timestamp)
        };
        
        println!("[CdpClient] Using user data dir: {}", user_data_dir);
        
        // Handle SingletonLock for persistent profiles
        if self.use_persistent_profile {
            self.handle_singleton_lock(&user_data_dir).await?;
        }
        
        let mut browser_config = BrowserConfig::builder()
            .window_size(1280, 1024)
            .user_data_dir(&user_data_dir)
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-background-timer-throttling")
            .arg("--disable-backgrounding-occluded-windows")
            .arg("--disable-renderer-backgrounding")
            .arg("--force-color-profile=srgb")
            // Stealth mode flags to reduce automation detection
            .arg("--disable-blink-features=AutomationControlled")
            .arg("--exclude-switches=enable-automation")
            .arg("--disable-infobars")
            .arg("--disable-dev-shm-usage")
            .arg("--user-agent=Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");
            
        if !headless {
            browser_config = browser_config.with_head();
        } else {
            browser_config = browser_config
                .arg("--headless")
                .arg("--disable-gpu");
        }
        
        let config = browser_config
            .build()
            .map_err(|e| AppError::BrowserAI(format!("Failed to build browser config: {}", e)))?;

        // Clean up old temporary session directories
        if !self.use_persistent_profile {
            Self::cleanup_old_sessions().await;
        }
        
        // Try to launch with retries
        let mut last_error = None;
        for attempt in 1..=3 {
            match Browser::launch(config.clone()).await {
                Ok((browser, handler)) => {
                    self.browser = Some(browser);
                    self.user_data_dir = Some(user_data_dir);
                    
                    // Spawn handler task
                    tokio::spawn(async move {
                        let mut handler = handler;
                        while let Some(event) = handler.next().await {
                            // Only log actual errors, not deserialization warnings
                            match event {
                                Err(e) => {
                                    let error_str = format!("{:?}", e);
                                    // Only log if it's not a deserialization error
                                    if !error_str.contains("data did not match any variant") {
                                        eprintln!("[CdpClient] Browser handler error: {}", e);
                                    }
                                }
                                Ok(_) => {}
                            }
                        }
                    });
                    
                    // Create new page
                    if let Some(browser) = &self.browser {
                        let page = browser
                            .new_page("about:blank")
                            .await
                            .map_err(|e| AppError::BrowserAI(format!("Failed to create new page: {}", e)))?;
                        self.page = Some(page);
                    }
                    
                    println!("[CdpClient] Chrome launched successfully, page created");
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < 3 {
                        println!("[CdpClient] Launch attempt {} failed, retrying...", attempt);
                        tokio::time::sleep(Duration::from_millis(1000 * attempt)).await;
                        
                        // Try to clean lock again
                        if self.use_persistent_profile {
                            let _ = self.handle_singleton_lock(&user_data_dir).await;
                        }
                    }
                }
            }
        }
        
        Err(AppError::BrowserAI(format!(
            "Failed to launch browser after 3 attempts: {}",
            last_error.map(|e| e.to_string()).unwrap_or_else(|| "Unknown error".to_string())
        )))
    }

    pub async fn connect_to_existing(&mut self, ws_url: &str) -> Result<()> {
        let (browser, mut handler) = Browser::connect(ws_url)
            .await
            .map_err(|e| AppError::BrowserAI(format!("Failed to connect to browser: {}", e)))?;

        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                // Only log actual errors, not deserialization warnings
                match event {
                    Err(e) => {
                        let error_str = format!("{:?}", e);
                        // Only log if it's not a deserialization error
                        if !error_str.contains("data did not match any variant") {
                            eprintln!("[CdpClient] Browser handler error: {}", e);
                        }
                    }
                    Ok(_) => {}
                }
            }
        });

        let pages = browser.pages().await
            .map_err(|e| AppError::BrowserAI(format!("Failed to get pages: {}", e)))?;
        
        let page = if pages.is_empty() {
            browser.new_page("about:blank").await
                .map_err(|e| AppError::BrowserAI(format!("Failed to create new page: {}", e)))?
        } else {
            pages.into_iter().next().unwrap()
        };

        self.browser = Some(browser);
        self.page = Some(page);

        Ok(())
    }

    pub async fn navigate(&self, url: &str) -> Result<()> {
        println!("[CdpClient] Navigating to: {}", url);
        
        let page = self.page.as_ref()
            .ok_or_else(|| AppError::BrowserAI("No page available".into()))?;
        
        page.goto(url)
            .await
            .map_err(|e| AppError::BrowserAI(format!("Failed to navigate: {}", e)))?;
        
        // Wait a bit for initial load
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Try to wait for navigation, but don't fail if it times out
        println!("[CdpClient] Waiting for navigation to complete...");
        match tokio::time::timeout(
            Duration::from_secs(5),
            page.wait_for_navigation()
        ).await {
            Ok(Ok(_)) => println!("[CdpClient] Navigation complete"),
            Ok(Err(e)) => println!("[CdpClient] Navigation wait error (continuing anyway): {}", e),
            Err(_) => println!("[CdpClient] Navigation wait timed out (continuing anyway)"),
        }
        
        // Additional wait to ensure content is loaded
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        Ok(())
    }

    pub async fn get_page_state(&self, include_screenshot: bool) -> Result<PageState> {
        let page = self.page.as_ref()
            .ok_or_else(|| AppError::BrowserAI("No page available".into()))?;

        let url = page.url().await
            .map_err(|e| AppError::BrowserAI(format!("Failed to get URL: {}", e)))?
            .unwrap_or_default();

        let title = page.evaluate("document.title")
            .await
            .map_err(|e| AppError::BrowserAI(format!("Failed to get title: {}", e)))?
            .into_value()
            .ok()
            .and_then(|v: serde_json::Value| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let interactive_elements = self.extract_interactive_elements().await?;

        let screenshot = if include_screenshot {
            Some(self.take_screenshot().await?)
        } else {
            None
        };

        Ok(PageState {
            url,
            title,
            interactive_elements,
            screenshot,
            console_logs: vec![], // TODO: Implement console log capture
        })
    }

    async fn extract_interactive_elements(&self) -> Result<Vec<DomElement>> {
        let page = self.page.as_ref()
            .ok_or_else(|| AppError::BrowserAI("No page available".into()))?;

        let elements_result = page.evaluate(r#"
            () => {
                const interactiveSelectors = [
                    'a', 'button', 'input', 'select', 'textarea',
                    '[role="button"]', '[role="link"]', '[onclick]'
                ];
                
                const elements = [];
                let index = 0;
                
                for (const selector of interactiveSelectors) {
                    const elems = document.querySelectorAll(selector);
                    for (const elem of elems) {
                        const rect = elem.getBoundingClientRect();
                        const isVisible = rect.width > 0 && rect.height > 0 && 
                                        getComputedStyle(elem).display !== 'none' &&
                                        getComputedStyle(elem).visibility !== 'hidden';
                        
                        if (isVisible) {
                            const attributes = {};
                            for (const attr of elem.attributes) {
                                attributes[attr.name] = attr.value;
                            }
                            
                            elements.push({
                                index: index++,
                                tag: elem.tagName.toLowerCase(),
                                text: elem.textContent?.trim() || null,
                                attributes: attributes,
                                is_interactive: true,
                                is_visible: true,
                                selector: elem.tagName.toLowerCase() + 
                                         (elem.id ? '#' + elem.id : '') +
                                         (elem.className ? '.' + elem.className.split(' ').join('.') : '')
                            });
                        }
                    }
                }
                
                return elements;
            }
        "#)
        .await
        .map_err(|e| AppError::BrowserAI(format!("Failed to extract elements: {}", e)))?;

        let elements_json = elements_result.into_value()
            .map_err(|e| AppError::BrowserAI(format!("Failed to get elements value: {}", e)))?;

        let elements: Vec<DomElement> = serde_json::from_value(elements_json)
            .map_err(|e| AppError::BrowserAI(format!("Failed to parse elements: {}", e)))?;

        Ok(elements)
    }

    pub async fn take_screenshot(&self) -> Result<Vec<u8>> {
        let page = self.page.as_ref()
            .ok_or_else(|| AppError::BrowserAI("No page available".into()))?;

        let params = CaptureScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .build();

        let screenshot = page.screenshot(params)
            .await
            .map_err(|e| AppError::BrowserAI(format!("Failed to take screenshot: {}", e)))?;

        Ok(screenshot)
    }

    pub async fn execute_javascript(&self, script: &str) -> Result<serde_json::Value> {
        println!("[CdpClient] Executing JavaScript: {}", script.lines().next().unwrap_or("..."));
        
        let page = self.page.as_ref()
            .ok_or_else(|| AppError::BrowserAI("No page available".into()))?;

        let result = page.evaluate(script)
            .await
            .map_err(|e| AppError::BrowserAI(format!("Failed to execute script: {}", e)))?;

        let value: serde_json::Value = result.into_value()
            .map_err(|e| AppError::BrowserAI(format!("Failed to get script result: {}", e)))?;

        println!("[CdpClient] JavaScript result type: {}", 
            if value.is_array() { "array" } 
            else if value.is_object() { "object" } 
            else if value.is_string() { "string" } 
            else { "other" }
        );
        
        Ok(value)
    }

    pub async fn click_element(&self, index: usize) -> Result<()> {
        let page = self.page.as_ref()
            .ok_or_else(|| AppError::BrowserAI("No page available".into()))?;

        let script = format!(r#"
            () => {{
                const elements = Array.from(document.querySelectorAll('a, button, input, select, textarea, [role="button"], [role="link"], [onclick]'))
                    .filter(elem => {{
                        const rect = elem.getBoundingClientRect();
                        return rect.width > 0 && rect.height > 0 && 
                               getComputedStyle(elem).display !== 'none' &&
                               getComputedStyle(elem).visibility !== 'hidden';
                    }});
                
                if (elements[{}]) {{
                    elements[{}].click();
                    return true;
                }}
                return false;
            }}
        "#, index, index);

        let result = page.evaluate(script.as_str())
            .await
            .map_err(|e| AppError::BrowserAI(format!("Failed to click element: {}", e)))?;

        let clicked: serde_json::Value = result.into_value()
            .map_err(|e| AppError::BrowserAI(format!("Failed to get click result: {}", e)))?;

        if !clicked.as_bool().unwrap_or(false) {
            return Err(AppError::BrowserAI(format!("Element at index {} not found", index)));
        }

        Ok(())
    }

    pub async fn type_text(&self, index: usize, text: &str, delay_ms: u32) -> Result<()> {
        let page = self.page.as_ref()
            .ok_or_else(|| AppError::BrowserAI("No page available".into()))?;

        let script = format!(r#"
            () => {{
                const elements = Array.from(document.querySelectorAll('input, textarea'))
                    .filter(elem => {{
                        const rect = elem.getBoundingClientRect();
                        return rect.width > 0 && rect.height > 0 && 
                               getComputedStyle(elem).display !== 'none' &&
                               getComputedStyle(elem).visibility !== 'hidden';
                    }});
                
                if (elements[{}]) {{
                    elements[{}].focus();
                    elements[{}].value = '{}';
                    elements[{}].dispatchEvent(new Event('input', {{ bubbles: true }}));
                    elements[{}].dispatchEvent(new Event('change', {{ bubbles: true }}));
                    return true;
                }}
                return false;
            }}
        "#, index, index, index, text.replace("'", "\\'"), index, index);

        let result = page.evaluate(script.as_str())
            .await
            .map_err(|e| AppError::BrowserAI(format!("Failed to type text: {}", e)))?;

        let typed: serde_json::Value = result.into_value()
            .map_err(|e| AppError::BrowserAI(format!("Failed to get type result: {}", e)))?;

        if !typed.as_bool().unwrap_or(false) {
            return Err(AppError::BrowserAI(format!("Input element at index {} not found", index)));
        }

        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
        }

        Ok(())
    }

    pub async fn select_option(&self, index: usize, value: &str) -> Result<()> {
        let page = self.page.as_ref()
            .ok_or_else(|| AppError::BrowserAI("No page available".into()))?;

        let script = format!(r#"
            () => {{
                const elements = Array.from(document.querySelectorAll('select'))
                    .filter(elem => {{
                        const rect = elem.getBoundingClientRect();
                        return rect.width > 0 && rect.height > 0 && 
                               getComputedStyle(elem).display !== 'none' &&
                               getComputedStyle(elem).visibility !== 'hidden';
                    }});
                
                if (elements[{}]) {{
                    elements[{}].value = '{}';
                    elements[{}].dispatchEvent(new Event('change', {{ bubbles: true }}));
                    return true;
                }}
                return false;
            }}
        "#, index, index, value.replace("'", "\\'"), index);

        let result = page.evaluate(script.as_str())
            .await
            .map_err(|e| AppError::BrowserAI(format!("Failed to select option: {}", e)))?;

        let selected: serde_json::Value = result.into_value()
            .map_err(|e| AppError::BrowserAI(format!("Failed to get select result: {}", e)))?;

        if !selected.as_bool().unwrap_or(false) {
            return Err(AppError::BrowserAI(format!("Select element at index {} not found", index)));
        }

        Ok(())
    }

    pub async fn wait_for_selector(&self, selector: &str, timeout_secs: u64) -> Result<()> {
        let page = self.page.as_ref()
            .ok_or_else(|| AppError::BrowserAI("No page available".into()))?;

        let start = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(timeout_secs);
        
        loop {
            let script = format!(r#"
                () => {{
                    const element = document.querySelector('{}');
                    return element !== null;
                }}
            "#, selector.replace("'", "\\'"));
            
            let result = page.evaluate(script.as_str())
                .await
                .map_err(|e| AppError::BrowserAI(format!("Failed to check selector: {}", e)))?;
                
            let exists: serde_json::Value = result.into_value()
                .map_err(|e| AppError::BrowserAI(format!("Failed to get selector check result: {}", e)))?;
                
            if exists.as_bool().unwrap_or(false) {
                return Ok(());
            }
            
            if start.elapsed() > timeout_duration {
                return Err(AppError::BrowserAI(format!("Timeout waiting for selector: {}", selector)));
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn get_persistent_profile_dir(&self) -> Result<String> {
        // Get user's home directory
        let home_dir = dirs::home_dir()
            .ok_or_else(|| AppError::BrowserAI("Could not determine home directory".into()))?;
        
        // Create path to PersonalAssistant Chrome profile
        let profile_dir = home_dir
            .join("Library")
            .join("Application Support")
            .join("PersonalAssistant")
            .join("ChromeProfile");
        
        // Create directory if it doesn't exist
        if !profile_dir.exists() {
            fs::create_dir_all(&profile_dir)
                .map_err(|e| AppError::BrowserAI(format!("Failed to create profile directory: {}", e)))?;
            println!("[CdpClient] Created persistent profile directory at: {:?}", profile_dir);
        }
        
        Ok(profile_dir.to_string_lossy().to_string())
    }
    
    async fn handle_singleton_lock(&self, profile_dir: &str) -> Result<()> {
        let lock_path = Path::new(profile_dir).join("SingletonLock");
        
        if lock_path.exists() {
            println!("[CdpClient] SingletonLock exists at: {:?}", lock_path);
            
            // Check if the lock is stale (older than 5 minutes)
            if let Ok(metadata) = fs::metadata(&lock_path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = modified.elapsed() {
                        if elapsed.as_secs() > 300 {
                            println!("[CdpClient] SingletonLock is stale ({}s old), removing...", elapsed.as_secs());
                            let _ = fs::remove_file(&lock_path);
                            return Ok(());
                        }
                    }
                }
            }
            
            // Check if Chrome is actually running with this profile
            if !self.is_chrome_running_with_profile(profile_dir).await {
                println!("[CdpClient] No Chrome process found using this profile, removing lock...");
                let _ = fs::remove_file(&lock_path);
                return Ok(());
            }
            
            // Chrome is running, wait a bit and check again
            println!("[CdpClient] Chrome is running with this profile, waiting...");
            for i in 1..=6 {
                tokio::time::sleep(Duration::from_secs(5)).await;
                
                if !lock_path.exists() {
                    println!("[CdpClient] Lock released after {}s", i * 5);
                    return Ok(());
                }
                
                if !self.is_chrome_running_with_profile(profile_dir).await {
                    println!("[CdpClient] Chrome process ended, removing lock...");
                    let _ = fs::remove_file(&lock_path);
                    return Ok(());
                }
            }
            
            // Still locked after 30 seconds
            return Err(AppError::BrowserAI(
                "Chrome is already running with this profile. Please close it and try again.".into()
            ));
        }
        
        Ok(())
    }
    
    async fn is_chrome_running_with_profile(&self, profile_dir: &str) -> bool {
        #[cfg(unix)]
        {
            if let Ok(output) = std::process::Command::new("ps")
                .args(&["aux"])
                .output()
            {
                let ps_output = String::from_utf8_lossy(&output.stdout);
                for line in ps_output.lines() {
                    if line.contains("chrome") && line.contains("--user-data-dir") && line.contains(profile_dir) {
                        return true;
                    }
                }
            }
        }
        false
    }

    async fn kill_zombie_chrome_processes() {
        println!("[CdpClient] Checking for zombie Chrome processes");
        
        #[cfg(unix)]
        {
            // Try to find and kill Chrome processes started by chromiumoxide
            if let Ok(output) = std::process::Command::new("ps")
                .args(&["aux"])
                .output()
            {
                let ps_output = String::from_utf8_lossy(&output.stdout);
                for line in ps_output.lines() {
                    if line.contains("chromiumoxide-runner") && line.contains("chrome") {
                        // Extract PID from the ps output
                        if let Some(pid_str) = line.split_whitespace().nth(1) {
                            if let Ok(pid) = pid_str.parse::<i32>() {
                                println!("[CdpClient] Found zombie Chrome process with PID: {}", pid);
                                let _ = std::process::Command::new("kill")
                                    .args(&["-9", &pid.to_string()])
                                    .output();
                            }
                        }
                    }
                }
            }
        }
    }

    async fn cleanup_old_sessions() {
        println!("[CdpClient] Cleaning up old Chrome sessions");
        
        // Try to clean up session directories older than 1 hour
        if let Ok(entries) = std::fs::read_dir("/tmp") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("chromiumoxide-session-") {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                if let Ok(elapsed) = modified.elapsed() {
                                    // Remove directories older than 1 hour
                                    if elapsed.as_secs() > 3600 {
                                        if let Err(e) = std::fs::remove_dir_all(entry.path()) {
                                            println!("[CdpClient] Failed to remove old session dir: {}", e);
                                        } else {
                                            println!("[CdpClient] Removed old session: {}", name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn close(&mut self) -> Result<()> {
        println!("[CdpClient] Closing browser");
        
        if let Some(mut browser) = self.browser.take() {
            browser.close().await
                .map_err(|e| AppError::BrowserAI(format!("Failed to close browser: {}", e)))?;
        }
        
        self.page = None;
        
        // Only clean up temporary user data directories
        if let Some(dir) = self.user_data_dir.take() {
            if !self.use_persistent_profile && dir.contains("chromiumoxide-session-") {
                println!("[CdpClient] Cleaning up temporary user data dir: {}", dir);
                if let Err(e) = std::fs::remove_dir_all(&dir) {
                    println!("[CdpClient] Failed to remove user data dir: {}", e);
                }
            } else {
                println!("[CdpClient] Keeping persistent profile at: {}", dir);
            }
        }
        
        Ok(())
    }
}

impl Drop for CdpClient {
    fn drop(&mut self) {
        if self.browser.is_some() || self.user_data_dir.is_some() {
            let browser = self.browser.take();
            let user_data_dir = self.user_data_dir.take();
            let use_persistent_profile = self.use_persistent_profile;
            
            tokio::spawn(async move {
                if let Some(mut browser) = browser {
                    let _ = browser.close().await;
                }
                
                // Only clean up temporary directories
                if let Some(dir) = user_data_dir {
                    if !use_persistent_profile && dir.contains("chromiumoxide-session-") {
                        println!("[CdpClient] Drop: Cleaning up temporary user data dir: {}", dir);
                        if let Err(e) = std::fs::remove_dir_all(&dir) {
                            println!("[CdpClient] Drop: Failed to remove user data dir: {}", e);
                        }
                    }
                }
            });
        }
    }
}