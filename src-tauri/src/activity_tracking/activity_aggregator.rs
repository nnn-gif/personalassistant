use crate::models::Activity;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;

/// Aggregates consecutive activities with the same app/window to reduce storage
pub struct ActivityAggregator {
    pending_activity: Option<Activity>,
    max_aggregation_seconds: i64,
}

impl ActivityAggregator {
    pub fn new() -> Self {
        Self {
            pending_activity: None,
            max_aggregation_seconds: 300, // Max 5 minutes per aggregated activity
        }
    }

    /// Process a new activity, either aggregating it or returning the previous one to save
    pub fn process_activity(&mut self, new_activity: Activity) -> Option<Activity> {
        if let Some(pending) = self.pending_activity.take() {
            if self.should_aggregate(&pending, &new_activity) {
                // Put it back and aggregate
                self.pending_activity = Some(pending);
                self.aggregate_into_pending(&new_activity);
                None
            } else {
                // Can't aggregate, return the pending activity and store the new one
                self.pending_activity = Some(new_activity);
                Some(pending)
            }
        } else {
            // First activity, just store it
            self.pending_activity = Some(new_activity);
            None
        }
    }

    /// Force flush any pending activity
    pub fn flush(&mut self) -> Option<Activity> {
        self.pending_activity.take()
    }

    /// Check if two activities can be aggregated
    fn should_aggregate(&self, existing: &Activity, new: &Activity) -> bool {
        // Don't aggregate if duration would exceed max
        if existing.duration_seconds + new.duration_seconds > self.max_aggregation_seconds {
            return false;
        }

        // Only aggregate if core properties match
        existing.app_usage.app_name == new.app_usage.app_name
            && existing.app_usage.window_title == new.app_usage.window_title
            && existing.goal_id == new.goal_id
            && existing.project_context.as_ref().map(|p| &p.project_name)
                == new.project_context.as_ref().map(|p| &p.project_name)
    }

    /// Merge new activity data into the pending activity
    fn aggregate_into_pending(&mut self, new_activity: &Activity) {
        if let Some(pending) = &mut self.pending_activity {
            // Extend duration
            pending.duration_seconds += new_activity.duration_seconds;

            // Aggregate input metrics
            pending.input_metrics.keystrokes += new_activity.input_metrics.keystrokes;
            pending.input_metrics.mouse_clicks += new_activity.input_metrics.mouse_clicks;
            pending.input_metrics.mouse_distance_pixels +=
                new_activity.input_metrics.mouse_distance_pixels;
            pending.input_metrics.active_typing_seconds +=
                new_activity.input_metrics.active_typing_seconds;

            // Update system state to latest
            pending.system_state = new_activity.system_state.clone();

            // Keep other fields from the original activity
        }
    }
}

/// In-memory cache for recent activities
pub struct ActivityCache {
    activities: VecDeque<Activity>,
    max_size: usize,
    total_duration_seconds: i64,
}

impl ActivityCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            activities: VecDeque::with_capacity(max_size),
            max_size,
            total_duration_seconds: 0,
        }
    }

    /// Add an activity to the cache
    pub fn add_activity(&mut self, activity: Activity) {
        self.total_duration_seconds += activity.duration_seconds;
        self.activities.push_back(activity);

        // Remove old activities if cache is full
        while self.activities.len() > self.max_size {
            if let Some(removed) = self.activities.pop_front() {
                self.total_duration_seconds -= removed.duration_seconds;
            }
        }
    }

    /// Get activities within a time range from cache
    pub fn get_activities_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<Activity> {
        self.activities
            .iter()
            .filter(|a| a.timestamp >= start && a.timestamp <= end)
            .cloned()
            .collect()
    }

    /// Check if a time range is fully covered by the cache
    pub fn covers_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> bool {
        if let (Some(first), Some(last)) = (self.activities.front(), self.activities.back()) {
            // Check if cache covers the requested range
            first.timestamp <= start && last.timestamp >= end
        } else {
            false
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            activity_count: self.activities.len(),
            oldest_timestamp: self.activities.front().map(|a| a.timestamp),
            newest_timestamp: self.activities.back().map(|a| a.timestamp),
            total_duration_seconds: self.total_duration_seconds,
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub activity_count: usize,
    pub oldest_timestamp: Option<DateTime<Utc>>,
    pub newest_timestamp: Option<DateTime<Utc>>,
    pub total_duration_seconds: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppCategory, ProjectContext, ProjectType};

    fn create_test_activity(app_name: &str, window_title: &str) -> Activity {
        Activity {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            duration_seconds: 5,
            app_usage: AppUsage {
                app_name: app_name.to_string(),
                bundle_id: String::new(),
                window_title: window_title.to_string(),
                category: AppCategory::Development,
                is_productive: true,
                browser_url: None,
                editor_file: None,
                terminal_info: None,
            },
            input_metrics: InputMetrics {
                keystrokes: 10,
                mouse_clicks: 2,
                mouse_distance_pixels: 100.0,
                active_typing_seconds: 3,
            },
            system_state: SystemState {
                idle_time_seconds: 0,
                is_screen_locked: false,
                battery_percentage: Some(80.0),
                is_on_battery: false,
                cpu_usage_percent: 10.0,
                memory_usage_mb: 1000,
            },
            project_context: None,
            goal_id: None,
        }
    }

    #[test]
    fn test_activity_aggregation() {
        let mut aggregator = ActivityAggregator::new();

        // First activity should be stored
        let act1 = create_test_activity("VSCode", "main.rs");
        assert!(aggregator.process_activity(act1.clone()).is_none());

        // Same app/window should aggregate
        let act2 = create_test_activity("VSCode", "main.rs");
        assert!(aggregator.process_activity(act2).is_none());

        // Different app should return previous and store new
        let act3 = create_test_activity("Chrome", "Google");
        let returned = aggregator.process_activity(act3).unwrap();
        assert_eq!(returned.duration_seconds, 10); // 5 + 5
        assert_eq!(returned.input_metrics.keystrokes, 20); // 10 + 10
    }
}