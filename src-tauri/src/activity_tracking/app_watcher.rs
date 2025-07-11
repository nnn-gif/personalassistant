use crate::error::Result;
use crate::models::{AppUsage, AppCategory, TerminalInfo};
use objc2_foundation::NSString;
use objc2_app_kit::NSWorkspace;

pub struct AppWatcher;

impl AppWatcher {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get_current_app(&self) -> Result<AppUsage> {
        // Simplified version - in production would use proper macOS APIs
        let app_name = "Finder".to_string();
        let bundle_id = "com.apple.finder".to_string();
        let window_title = "Finder Window".to_string();
        let category = self.categorize_app(&app_name, &bundle_id);
        let is_productive = self.is_productive(&category);
        
        Ok(AppUsage {
            app_name,
            bundle_id,
            window_title,
            category,
            is_productive,
            browser_url: None,
            editor_file: None,
            terminal_info: None,
        })
    }
    
    fn get_window_title(&self, app_name: &str) -> Result<String> {
        // This is simplified - getting window titles on macOS requires more complex APIs
        // For now, return a placeholder
        Ok(format!("{} - Window", app_name))
    }
    
    fn categorize_app(&self, app_name: &str, bundle_id: &str) -> AppCategory {
        match app_name.to_lowercase().as_str() {
            name if name.contains("code") || name.contains("xcode") || name.contains("intellij") => AppCategory::Development,
            name if name.contains("slack") || name.contains("teams") || name.contains("zoom") => AppCategory::Communication,
            name if name.contains("twitter") || name.contains("facebook") => AppCategory::SocialMedia,
            name if name.contains("spotify") || name.contains("youtube") => AppCategory::Entertainment,
            name if name.contains("notion") || name.contains("obsidian") => AppCategory::Productivity,
            name if name.contains("finder") || name.contains("system") => AppCategory::System,
            _ => AppCategory::Other,
        }
    }
    
    fn is_productive(&self, category: &AppCategory) -> bool {
        matches!(
            category,
            AppCategory::Development | AppCategory::Productivity | AppCategory::Communication
        )
    }
    
    fn is_browser(&self, app_name: &str) -> bool {
        let name = app_name.to_lowercase();
        name.contains("safari") || name.contains("chrome") || name.contains("firefox") || name.contains("edge")
    }
    
    fn is_code_editor(&self, app_name: &str) -> bool {
        let name = app_name.to_lowercase();
        name.contains("code") || name.contains("xcode") || name.contains("intellij") || name.contains("sublime")
    }
    
    fn is_terminal(&self, app_name: &str) -> bool {
        let name = app_name.to_lowercase();
        name.contains("terminal") || name.contains("iterm") || name.contains("warp")
    }
    
    fn detect_browser_url(&self, window_title: &str) -> Option<String> {
        // Extract URL from window title - this is a simplified version
        // Real implementation would use accessibility APIs
        if window_title.contains(" - ") {
            Some(window_title.split(" - ").next()?.to_string())
        } else {
            None
        }
    }
    
    fn detect_editor_file(&self, window_title: &str) -> Option<String> {
        // Extract file path from window title
        if window_title.contains(" — ") {
            Some(window_title.split(" — ").next()?.to_string())
        } else {
            None
        }
    }
    
    fn detect_terminal_details(&self, window_title: &str) -> Option<TerminalInfo> {
        // Extract terminal info from window title
        Some(TerminalInfo {
            current_directory: window_title.to_string(),
            last_command: None, // Would require AppleScript or similar
        })
    }
}