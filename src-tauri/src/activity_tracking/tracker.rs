use super::{ActivityHistory, AppWatcher, InputMonitor, ProjectDetector, SystemMonitor};
use crate::database::SqliteDatabase;
use crate::error::Result;
use crate::models::Activity;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct ActivityTracker {
    app_watcher: AppWatcher,
    project_detector: ProjectDetector,
    system_monitor: SystemMonitor,
    input_monitor: InputMonitor,
    history: ActivityHistory,
    is_tracking: bool,
    current_activity: Option<Activity>,
    db: Option<Arc<Mutex<SqliteDatabase>>>,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self {
            app_watcher: AppWatcher::new(),
            project_detector: ProjectDetector::new(),
            system_monitor: SystemMonitor::new(),
            input_monitor: InputMonitor::new(),
            history: ActivityHistory::new(),
            is_tracking: true,
            current_activity: None,
            db: None,
        }
    }

    pub fn set_database(&mut self, db: Arc<Mutex<SqliteDatabase>>) {
        self.db = Some(db);
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

    pub async fn collect_activity(
        &mut self,
        active_goal: Option<(Uuid, Vec<String>)>,
    ) -> Result<Activity> {
        let app_usage = self.app_watcher.get_current_app()?;
        let project_context = self
            .project_detector
            .detect_project(&app_usage.app_name, &app_usage.window_title)?;

        // Get real input metrics
        let input_metrics = self.input_monitor.get_metrics_and_reset()?;

        // Get real system state
        let system_state = self.system_monitor.get_system_state()?;

        // Check if current app is part of active goal
        let goal_id = if let Some((goal_id, allowed_apps)) = active_goal {
            if allowed_apps
                .iter()
                .any(|app| app.eq_ignore_ascii_case(&app_usage.app_name))
            {
                Some(goal_id)
            } else {
                None
            }
        } else {
            None
        };

        let activity = Activity {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            duration_seconds: 5, // Default tracking interval
            app_usage,
            input_metrics,
            system_state,
            project_context,
            goal_id,
        };

        // Store in history
        self.history.add_activity(activity.clone());

        // Save to database
        if let Some(db) = &self.db {
            let db_clone = db.clone();
            let activity_clone = activity.clone();
            // Save in background to avoid blocking
            tokio::spawn(async move {
                if let Ok(_) = db_clone.lock().await.save_activity(&activity_clone).await {
                    // Saved successfully
                } else {
                    eprintln!("Failed to save activity to database");
                }
            });
        }

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

    pub fn get_history(&self) -> &ActivityHistory {
        &self.history
    }

    pub fn get_recent_activities(&self, limit: usize) -> Vec<Activity> {
        self.history
            .get_recent(limit)
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn get_productivity_stats(&self, hours: i64) -> (u32, u32) {
        let productive = self.history.get_productive_time(hours);
        let total = self.history.get_total_time(hours);
        (productive, total)
    }
}
