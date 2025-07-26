use crate::activity_tracking::TrackerWrapper;
use crate::error::Result;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Force flush any pending activities to the database
#[tauri::command]
pub async fn flush_pending_activities(
    tracker: State<'_, Arc<Mutex<TrackerWrapper>>>,
) -> Result<()> {
    let mut tracker = tracker.lock().await;
    tracker.flush_pending().await?;
    Ok(())
}