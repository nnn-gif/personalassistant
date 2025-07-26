use super::{ActivityTracker, OptimizedActivityTracker};
use crate::config::Config;
use crate::database::SqliteDatabase;
use crate::error::Result;
use crate::models::Activity;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Wrapper that uses either regular or optimized tracker based on configuration
pub enum TrackerWrapper {
    Regular(ActivityTracker),
    Optimized(OptimizedActivityTracker),
}

impl TrackerWrapper {
    pub fn new() -> Self {
        let config = Config::get();
        
        if config.tracking.use_optimized_tracker {
            println!("[TrackerWrapper] Using optimized activity tracker");
            TrackerWrapper::Optimized(OptimizedActivityTracker::new())
        } else {
            println!("[TrackerWrapper] Using regular activity tracker");
            TrackerWrapper::Regular(ActivityTracker::new())
        }
    }

    pub fn set_database(&mut self, db: Arc<Mutex<SqliteDatabase>>) {
        match self {
            TrackerWrapper::Regular(tracker) => tracker.set_database(db),
            TrackerWrapper::Optimized(tracker) => tracker.set_database(db),
        }
    }

    pub async fn start_tracking(&mut self) -> Result<()> {
        match self {
            TrackerWrapper::Regular(tracker) => tracker.start_tracking().await,
            TrackerWrapper::Optimized(tracker) => tracker.start_tracking().await,
        }
    }

    pub async fn stop_tracking(&mut self) -> Result<()> {
        match self {
            TrackerWrapper::Regular(tracker) => tracker.stop_tracking().await,
            TrackerWrapper::Optimized(tracker) => tracker.stop_tracking().await,
        }
    }

    pub fn is_tracking(&self) -> bool {
        match self {
            TrackerWrapper::Regular(tracker) => tracker.is_tracking(),
            TrackerWrapper::Optimized(tracker) => tracker.is_tracking(),
        }
    }

    pub async fn collect_activity(
        &mut self,
        active_goal: Option<(Uuid, Vec<String>)>,
    ) -> Result<Activity> {
        match self {
            TrackerWrapper::Regular(tracker) => {
                tracker.collect_activity(active_goal).await
            }
            TrackerWrapper::Optimized(tracker) => {
                // For optimized tracker, we need to collect the activity but return a dummy
                // since the optimized tracker doesn't expose the current activity
                tracker.collect_activity(active_goal.clone()).await?;
                
                // Create a minimal activity for compatibility
                // In real usage, the optimized tracker handles everything internally
                Ok(Activity {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    duration_seconds: 5,
                    app_usage: crate::models::AppUsage {
                        app_name: String::from("Unknown"),
                        bundle_id: String::new(),
                        window_title: String::from("Unknown"),
                        category: crate::models::AppCategory::Other,
                        is_productive: false,
                        browser_url: None,
                        editor_file: None,
                        terminal_info: None,
                    },
                    input_metrics: crate::models::InputMetrics {
                        keystrokes: 0,
                        mouse_clicks: 0,
                        mouse_distance_pixels: 0.0,
                        active_typing_seconds: 0,
                    },
                    system_state: crate::models::SystemState {
                        idle_time_seconds: 0,
                        is_screen_locked: false,
                        battery_percentage: None,
                        is_on_battery: false,
                        cpu_usage_percent: 0.0,
                        memory_usage_mb: 0,
                    },
                    project_context: None,
                    goal_id: active_goal.map(|(id, _)| id),
                })
            }
        }
    }

    pub fn get_current_activity(&self) -> Option<&Activity> {
        match self {
            TrackerWrapper::Regular(tracker) => tracker.get_current_activity(),
            TrackerWrapper::Optimized(tracker) => tracker.get_current_activity(),
        }
    }

    pub fn get_recent_activities(&self, limit: usize) -> Vec<Activity> {
        match self {
            TrackerWrapper::Regular(tracker) => tracker.get_recent_activities(limit),
            TrackerWrapper::Optimized(tracker) => tracker.get_recent_activities(limit),
        }
    }

    pub fn get_productivity_stats(&self, hours: u32) -> (u32, u32) {
        match self {
            TrackerWrapper::Regular(tracker) => tracker.get_productivity_stats(hours as i64),
            TrackerWrapper::Optimized(_) => (0, 0), // Would need to calculate from cache/db
        }
    }

    pub async fn get_activities_by_date_range(
        &mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        db: &Arc<Mutex<SqliteDatabase>>,
    ) -> Result<Vec<Activity>> {
        match self {
            TrackerWrapper::Regular(_) => {
                // Regular tracker fetches from database directly
                let db = db.lock().await;
                db.get_activities_by_date_range(start, end).await
            }
            TrackerWrapper::Optimized(tracker) => {
                // Optimized tracker uses cache when possible
                tracker.get_activities_by_date_range(start, end, db).await
            }
        }
    }
    
    /// Force flush any pending activities (for optimized tracker)
    pub async fn flush_pending(&mut self) -> Result<()> {
        if let TrackerWrapper::Optimized(tracker) = self {
            tracker.flush_pending().await?;
        }
        Ok(())
    }

    /// Print statistics (only available for optimized tracker)
    pub fn print_stats(&self) {
        if let TrackerWrapper::Optimized(tracker) = self {
            tracker.print_stats();
        }
    }
}