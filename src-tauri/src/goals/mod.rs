use crate::error::{AppError, Result};
use crate::models::{Goal, GoalSession};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

pub struct GoalService {
    goals: HashMap<Uuid, Goal>,
    sessions: HashMap<Uuid, Vec<GoalSession>>,
    active_goal_id: Option<Uuid>,
}

impl GoalService {
    pub fn new() -> Self {
        Self {
            goals: HashMap::new(),
            sessions: HashMap::new(),
            active_goal_id: None,
        }
    }
    
    pub fn create_goal(&mut self, name: String, duration_minutes: u32, allowed_apps: Vec<String>) -> Result<Goal> {
        let goal = Goal::new(name, duration_minutes, allowed_apps);
        self.goals.insert(goal.id, goal.clone());
        self.sessions.insert(goal.id, Vec::new());
        Ok(goal)
    }
    
    pub fn activate_goal(&mut self, goal_id: Uuid) -> Result<()> {
        // Deactivate any currently active goal
        if let Some(active_id) = self.active_goal_id {
            if let Some(active_goal) = self.goals.get_mut(&active_id) {
                active_goal.is_active = false;
                active_goal.updated_at = Utc::now();
            }
        }
        
        // Activate the requested goal
        let goal = self.goals.get_mut(&goal_id)
            .ok_or_else(|| AppError::NotFound(format!("Goal {} not found", goal_id)))?;
        
        goal.is_active = true;
        goal.updated_at = Utc::now();
        self.active_goal_id = Some(goal_id);
        
        // Start a new session
        let session = GoalSession {
            id: Uuid::new_v4(),
            goal_id,
            start_time: Utc::now(),
            end_time: None,
            duration_minutes: 0,
        };
        
        self.sessions.get_mut(&goal_id)
            .unwrap()
            .push(session);
        
        Ok(())
    }
    
    pub fn deactivate_goal(&mut self, goal_id: Uuid) -> Result<()> {
        let goal = self.goals.get_mut(&goal_id)
            .ok_or_else(|| AppError::NotFound(format!("Goal {} not found", goal_id)))?;
        
        goal.is_active = false;
        goal.updated_at = Utc::now();
        
        // End the current session
        if let Some(sessions) = self.sessions.get_mut(&goal_id) {
            if let Some(session) = sessions.last_mut() {
                if session.end_time.is_none() {
                    session.end_time = Some(Utc::now());
                    let duration = session.end_time.unwrap().signed_duration_since(session.start_time);
                    session.duration_minutes = (duration.num_seconds() / 60) as u32;
                    
                    // Update goal progress
                    goal.update_progress(session.duration_minutes);
                }
            }
        }
        
        if self.active_goal_id == Some(goal_id) {
            self.active_goal_id = None;
        }
        
        Ok(())
    }
    
    pub fn get_goal(&self, goal_id: &Uuid) -> Option<&Goal> {
        self.goals.get(goal_id)
    }
    
    pub fn get_all_goals(&self) -> Vec<&Goal> {
        self.goals.values().collect()
    }
    
    pub fn get_active_goal(&self) -> Option<&Goal> {
        self.active_goal_id.and_then(|id| self.goals.get(&id))
    }
    
    pub fn update_active_goal_progress(&mut self, app_name: &str, minutes: u32) -> Result<()> {
        if let Some(goal_id) = self.active_goal_id {
            let goal = self.goals.get_mut(&goal_id)
                .ok_or_else(|| AppError::Goal("Active goal not found".into()))?;
            
            if goal.is_app_allowed(app_name) {
                goal.update_progress(minutes);
            }
        }
        
        Ok(())
    }
    
    pub fn get_goal_sessions(&self, goal_id: &Uuid) -> Option<&Vec<GoalSession>> {
        self.sessions.get(goal_id)
    }
}