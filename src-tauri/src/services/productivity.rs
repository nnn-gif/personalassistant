use crate::activity_tracking::TrackerWrapper;
use crate::error::Result;
use crate::models::AppCategory;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductivityTrend {
    pub hour: i64,
    pub productive_minutes: u32,
    pub total_minutes: u32,
    pub productivity_percentage: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppUsageStats {
    pub app_name: String,
    pub category: AppCategory,
    pub total_minutes: u32,
    pub is_productive: bool,
}

#[tauri::command]
pub async fn get_productivity_trend(
    tracker: State<'_, Arc<Mutex<TrackerWrapper>>>,
    hours: Option<i64>,
) -> Result<Vec<ProductivityTrend>> {
    let tracker = tracker.lock().await;
    let hours = hours.unwrap_or(24);

    let mut trends = Vec::new();

    // Get productivity stats for each hour
    for h in 0..hours {
        let (productive_seconds, total_seconds) = tracker.get_productivity_stats((h + 1) as u32);
        let prev_productive = if h > 0 {
            let (p, _) = tracker.get_productivity_stats(h as u32);
            p
        } else {
            0
        };
        let prev_total = if h > 0 {
            let (_, t) = tracker.get_productivity_stats(h as u32);
            t
        } else {
            0
        };

        let hour_productive = productive_seconds.saturating_sub(prev_productive);
        let hour_total = total_seconds.saturating_sub(prev_total);

        let percentage = if hour_total > 0 {
            (hour_productive as f32 / hour_total as f32) * 100.0
        } else {
            0.0
        };

        trends.push(ProductivityTrend {
            hour: hours - h - 1,
            productive_minutes: hour_productive / 60,
            total_minutes: hour_total / 60,
            productivity_percentage: percentage,
        });
    }

    trends.reverse();
    Ok(trends)
}

#[tauri::command]
pub async fn get_app_usage_stats(
    tracker: State<'_, Arc<Mutex<TrackerWrapper>>>,
    hours: Option<i64>,
) -> Result<Vec<AppUsageStats>> {
    let tracker = tracker.lock().await;
    let _hours = hours.unwrap_or(24);

    // TrackerWrapper doesn't expose history directly, so use recent activities
    let recent_activities = tracker.get_recent_activities(1000);
    let mut app_time: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    
    for activity in &recent_activities {
        *app_time.entry(activity.app_usage.app_name.clone()).or_insert(0) += activity.duration_seconds as u32;
    }
    
    let stats = app_time;

    let mut app_stats = Vec::new();

    for (app_name, seconds) in stats {
        // Get category from recent activity
        let category = recent_activities
            .iter()
            .find(|a| a.app_usage.app_name == app_name)
            .map(|a| a.app_usage.category.clone())
            .unwrap_or(AppCategory::Other);

        let is_productive = matches!(
            category,
            AppCategory::Development | AppCategory::Productivity | AppCategory::Communication
        );

        app_stats.push(AppUsageStats {
            app_name,
            category,
            total_minutes: seconds / 60,
            is_productive,
        });
    }

    Ok(app_stats)
}

#[tauri::command]
pub async fn get_current_productivity_score(
    tracker: State<'_, Arc<Mutex<TrackerWrapper>>>,
) -> Result<f32> {
    let tracker = tracker.lock().await;
    let (productive, total) = tracker.get_productivity_stats(1); // Last hour

    if total > 0 {
        Ok((productive as f32 / total as f32) * 100.0)
    } else {
        Ok(0.0)
    }
}
