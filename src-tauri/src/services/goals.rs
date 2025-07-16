use crate::error::Result;
use crate::goals::GoalService;
use crate::models::Goal;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use uuid::Uuid;

#[tauri::command]
pub async fn create_goal(
    service: State<'_, Arc<Mutex<GoalService>>>,
    name: String,
    target_duration_minutes: u32,
    allowed_apps: Vec<String>,
) -> Result<Goal> {
    let mut service = service.lock().await;
    let goal = service
        .create_goal(name, target_duration_minutes, allowed_apps)
        .await?;
    Ok(goal)
}

#[tauri::command]
pub async fn activate_goal(
    service: State<'_, Arc<Mutex<GoalService>>>,
    goal_id: Uuid,
) -> Result<()> {
    let mut service = service.lock().await;
    service.activate_goal(goal_id).await?;
    Ok(())
}

#[tauri::command]
pub async fn deactivate_goal(
    service: State<'_, Arc<Mutex<GoalService>>>,
    goal_id: Uuid,
) -> Result<()> {
    let mut service = service.lock().await;
    service.deactivate_goal(goal_id).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_goals(service: State<'_, Arc<Mutex<GoalService>>>) -> Result<Vec<Goal>> {
    let service = service.lock().await;
    Ok(service.get_all_goals().into_iter().cloned().collect())
}

#[tauri::command]
pub async fn get_goal_progress(
    service: State<'_, Arc<Mutex<GoalService>>>,
    goal_id: Uuid,
) -> Result<Option<Goal>> {
    let service = service.lock().await;
    Ok(service.get_goal(&goal_id).cloned())
}

#[tauri::command]
pub async fn update_goal(
    service: State<'_, Arc<Mutex<GoalService>>>,
    goal_id: Uuid,
    name: String,
    target_duration_minutes: u32,
    allowed_apps: Vec<String>,
) -> Result<Goal> {
    let mut service = service.lock().await;
    let goal = service
        .update_goal(goal_id, name, target_duration_minutes, allowed_apps)
        .await?;
    Ok(goal)
}
