use crate::error::Result;
use crate::models::{Activity, InputMetrics, SystemState};
use super::{AppWatcher, ProjectDetector};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use uuid::Uuid;

pub struct ActivityTracker {
    app_watcher: AppWatcher,
    project_detector: ProjectDetector,
    is_tracking: bool,
    current_activity: Option<Activity>,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self {
            app_watcher: AppWatcher::new(),
            project_detector: ProjectDetector::new(),
            is_tracking: false,
            current_activity: None,
        }
    }
    
    pub async fn start_tracking(&mut self) -> Result<()> {
        self.is_tracking = true;
        Ok(())
    }
    
    pub async fn stop_tracking(&mut self) -> Result<()> {
        self.is_tracking = false;
        Ok(())
    }
    
    pub fn is_tracking(&self) -> bool {
        self.is_tracking
    }
    
    pub async fn collect_activity(&mut self) -> Result<Activity> {
        let app_usage = self.app_watcher.get_current_app()?;
        let project_context = self.project_detector
            .detect_project(&app_usage.app_name, &app_usage.window_title)?;
        
        // Placeholder for input metrics - would need platform-specific implementation
        let input_metrics = InputMetrics {
            keystrokes: 0,
            mouse_clicks: 0,
            mouse_distance_pixels: 0.0,
            active_typing_seconds: 0,
        };
        
        // Placeholder for system state - would need platform-specific implementation
        let system_state = SystemState {
            idle_time_seconds: 0,
            is_screen_locked: false,
            battery_percentage: Some(100.0),
            is_on_battery: false,
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0,
        };
        
        let activity = Activity {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            duration_seconds: 5, // Default tracking interval
            app_usage,
            input_metrics,
            system_state,
            project_context,
        };
        
        self.current_activity = Some(activity.clone());
        Ok(activity)
    }
    
    pub fn get_current_activity(&self) -> Option<&Activity> {
        self.current_activity.as_ref()
    }
    
    pub fn get_current_project(&self) -> Option<String> {
        self.current_activity
            .as_ref()
            .and_then(|a| a.project_context.as_ref())
            .map(|p| p.project_name.clone())
    }
}