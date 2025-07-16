use crate::activity_tracking::ActivityTracker;
use crate::error::Result;
use crate::models::Activity;
use crate::database::SqliteDatabase;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};

#[tauri::command]
pub async fn get_current_activity(
    tracker: State<'_, Arc<Mutex<ActivityTracker>>>,
) -> Result<Option<Activity>> {
    let tracker = tracker.lock().await;
    Ok(tracker.get_current_activity().cloned())
}

#[tauri::command]
pub async fn get_activity_history(
    tracker: State<'_, Arc<Mutex<ActivityTracker>>>,
    limit: Option<usize>,
) -> Result<Vec<Activity>> {
    let tracker = tracker.lock().await;
    let limit = limit.unwrap_or(50);

    Ok(tracker.get_recent_activities(limit))
}

#[tauri::command]
pub async fn start_tracking(tracker: State<'_, Arc<Mutex<ActivityTracker>>>) -> Result<()> {
    let mut tracker = tracker.lock().await;
    tracker.start_tracking().await
}

#[tauri::command]
pub async fn stop_tracking(tracker: State<'_, Arc<Mutex<ActivityTracker>>>) -> Result<()> {
    let mut tracker = tracker.lock().await;
    tracker.stop_tracking().await
}

#[tauri::command]
pub async fn get_tracking_stats(tracker: State<'_, Arc<Mutex<ActivityTracker>>>) -> Result<TrackingStats> {
    let tracker = tracker.lock().await;
    let recent_activities = tracker.get_recent_activities(100);
    let (productive_time, total_time) = tracker.get_productivity_stats(24);
    
    Ok(TrackingStats {
        is_tracking: tracker.is_tracking(),
        total_activities: recent_activities.len(),
        productive_time_seconds: productive_time,
        total_time_seconds: total_time,
        last_activity: tracker.get_current_activity().cloned(),
    })
}


#[derive(Debug, Serialize, Deserialize)]
pub struct TrackingStats {
    pub is_tracking: bool,
    pub total_activities: usize,
    pub productive_time_seconds: u32,
    pub total_time_seconds: u32,
    pub last_activity: Option<Activity>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodayStats {
    pub total_minutes: u32,
    pub productive_minutes: u32,
    pub total_activities: usize,
}

#[tauri::command]
pub async fn get_today_stats(
    db: State<'_, Arc<Mutex<SqliteDatabase>>>,
) -> Result<TodayStats> {
    let db = db.lock().await;
    
    // Get today's activities
    let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
    let now = Utc::now();
    
    let activities = db.get_activities_by_date_range(today_start, now).await?;
    
    let mut total_seconds = 0u32;
    let mut productive_seconds = 0u32;
    
    for activity in &activities {
        total_seconds += activity.duration_seconds as u32;
        if activity.app_usage.is_productive {
            productive_seconds += activity.duration_seconds as u32;
        }
    }
    
    Ok(TodayStats {
        total_minutes: total_seconds / 60,
        productive_minutes: productive_seconds / 60,
        total_activities: activities.len(),
    })
}
