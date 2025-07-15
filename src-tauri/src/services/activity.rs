use crate::activity_tracking::ActivityTracker;
use crate::error::Result;
use crate::models::Activity;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

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
