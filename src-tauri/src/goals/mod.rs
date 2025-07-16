use crate::database::SqliteDatabase;
use crate::error::{AppError, Result};
use crate::models::{Goal, GoalSession};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

// Default Master Goal constants
pub const DEFAULT_GOAL_NAME: &str = "Master Goal";
pub const DEFAULT_GOAL_ID_STR: &str = "00000000-0000-0000-0000-000000000001";

pub struct GoalService {
    db: Option<Arc<Mutex<SqliteDatabase>>>,
    sessions: HashMap<Uuid, Vec<GoalSession>>,
    active_goal_id: Option<Uuid>,
    // Cache for performance
    goals_cache: HashMap<Uuid, Goal>,
}

impl GoalService {
    pub fn new() -> Self {
        Self {
            db: None,
            sessions: HashMap::new(),
            active_goal_id: None,
            goals_cache: HashMap::new(),
        }
    }

    pub fn set_database(&mut self, db: Arc<Mutex<SqliteDatabase>>) {
        self.db = Some(db);
    }

    pub async fn load_from_database(&mut self) -> Result<()> {
        if let Some(db) = &self.db {
            {
                let db = db.lock().await;
                let goals = db.get_all_goals().await?;

                // Update cache and find active goal
                self.goals_cache.clear();
                for goal in goals {
                    if goal.is_active {
                        self.active_goal_id = Some(goal.id);
                    }
                    self.goals_cache.insert(goal.id, goal);
                }
            } // Release the db lock here

            // Ensure default Master Goal exists
            self.ensure_default_goal().await?;
        }
        Ok(())
    }

    async fn ensure_default_goal(&mut self) -> Result<()> {
        let default_id = Uuid::parse_str(DEFAULT_GOAL_ID_STR)
            .map_err(|e| AppError::Database(format!("Invalid default goal UUID: {}", e)))?;

        // Check if default goal exists
        if !self.goals_cache.contains_key(&default_id) {
            println!("Creating default Master Goal...");

            // Create the default goal with a special UUID
            let mut default_goal = Goal::new(
                DEFAULT_GOAL_NAME.to_string(),
                0,      // Unlimited duration for master goal
                vec![], // All apps allowed
            );
            default_goal.id = default_id;

            // Save to database
            self.save_goal(&default_goal).await?;

            // Add to cache
            self.goals_cache.insert(default_id, default_goal);
            self.sessions.insert(default_id, Vec::new());

            // If no other goal is active, make this the active goal
            if self.active_goal_id.is_none() {
                self.active_goal_id = Some(default_id);
                if let Some(goal) = self.goals_cache.get_mut(&default_id) {
                    goal.is_active = true;
                    let goal_clone = goal.clone();
                    self.save_goal(&goal_clone).await?;
                }
            }

            println!("Default Master Goal created and activated");
        }

        Ok(())
    }

    pub fn get_default_goal_id() -> Uuid {
        Uuid::parse_str(DEFAULT_GOAL_ID_STR).expect("Default goal ID should be a valid UUID")
    }

    pub fn get_current_or_default_goal_id(&self) -> Uuid {
        self.active_goal_id
            .unwrap_or_else(Self::get_default_goal_id)
    }

    async fn save_goal(&self, goal: &Goal) -> Result<()> {
        if let Some(db) = &self.db {
            let db = db.lock().await;
            db.save_goal(goal).await?;
        }
        Ok(())
    }

    pub async fn create_goal(
        &mut self,
        name: String,
        target_duration_minutes: u32,
        allowed_apps: Vec<String>,
    ) -> Result<Goal> {
        let goal = Goal::new(name, target_duration_minutes, allowed_apps);
        self.sessions.insert(goal.id, Vec::new());

        // Save to database
        self.save_goal(&goal).await?;

        // Update cache
        self.goals_cache.insert(goal.id, goal.clone());

        Ok(goal)
    }

    pub async fn update_goal(
        &mut self,
        goal_id: Uuid,
        name: String,
        target_duration_minutes: u32,
        allowed_apps: Vec<String>,
    ) -> Result<Goal> {
        // Check if goal exists
        if let Some(goal) = self.goals_cache.get_mut(&goal_id) {
            // Update goal properties
            goal.name = name;
            goal.target_duration_minutes = target_duration_minutes;
            goal.allowed_apps = allowed_apps;
            goal.updated_at = Utc::now();

            // Clone the goal before saving to avoid borrow checker issues
            let goal_to_save = goal.clone();

            // Save to database
            self.save_goal(&goal_to_save).await?;

            Ok(goal_to_save)
        } else {
            Err(AppError::NotFound(format!("Goal {} not found", goal_id)))
        }
    }

    pub async fn activate_goal(&mut self, goal_id: Uuid) -> Result<()> {
        println!("Activating goal: {}", goal_id);

        // Deactivate any currently active goal
        if let Some(active_id) = self.active_goal_id {
            println!("Deactivating currently active goal: {}", active_id);
            if let Some(active_goal) = self.goals_cache.get_mut(&active_id) {
                active_goal.is_active = false;
                active_goal.updated_at = Utc::now();
            }
            // Save the deactivated goal
            if let Some(active_goal) = self.goals_cache.get(&active_id) {
                let goal_to_save = active_goal.clone();
                self.save_goal(&goal_to_save).await?;
            }
        }

        // Activate the requested goal
        if let Some(goal) = self.goals_cache.get_mut(&goal_id) {
            println!("Found goal in cache, activating: {}", goal.name);
            goal.is_active = true;
            goal.updated_at = Utc::now();
        } else {
            println!("Goal not found in cache: {}", goal_id);
            return Err(AppError::NotFound(format!("Goal {} not found", goal_id)));
        }
        self.active_goal_id = Some(goal_id);

        // Save to database
        let goal = self
            .goals_cache
            .get(&goal_id)
            .ok_or_else(|| AppError::NotFound(format!("Goal {} not found in cache", goal_id)))?
            .clone();
        self.save_goal(&goal).await?;
        println!("Goal saved to database: {}", goal.name);

        // Start a new session
        let session = GoalSession {
            id: Uuid::new_v4(),
            goal_id,
            start_time: Utc::now(),
            end_time: None,
            duration_minutes: 0,
        };

        if let Some(sessions) = self.sessions.get_mut(&goal_id) {
            sessions.push(session);
        } else {
            self.sessions.insert(goal_id, vec![session]);
        }
        println!("Goal activation completed: {}", goal_id);

        Ok(())
    }

    pub async fn deactivate_goal(&mut self, goal_id: Uuid) -> Result<()> {
        println!("Deactivating goal: {}", goal_id);
        let mut additional_minutes = 0u32;

        // End the current session and calculate additional minutes
        if let Some(sessions) = self.sessions.get_mut(&goal_id) {
            if let Some(session) = sessions.last_mut() {
                if session.end_time.is_none() {
                    let end_time = Utc::now();
                    session.end_time = Some(end_time);
                    let duration = end_time.signed_duration_since(session.start_time);
                    session.duration_minutes = (duration.num_seconds() / 60) as u32;
                    additional_minutes = session.duration_minutes;
                }
            }
        }

        // Update the goal
        if let Some(goal) = self.goals_cache.get_mut(&goal_id) {
            println!("Found goal in cache, deactivating: {}", goal.name);
            goal.is_active = false;
            goal.updated_at = Utc::now();
            if additional_minutes > 0 {
                goal.update_progress(additional_minutes);
            }
        } else {
            println!("Goal not found in cache: {}", goal_id);
            return Err(AppError::NotFound(format!("Goal {} not found", goal_id)));
        }

        if self.active_goal_id == Some(goal_id) {
            self.active_goal_id = None;
            println!("Cleared active goal ID");
        }

        // Save to database
        let goal = self
            .goals_cache
            .get(&goal_id)
            .ok_or_else(|| AppError::NotFound(format!("Goal {} not found in cache", goal_id)))?
            .clone();
        self.save_goal(&goal).await?;
        println!("Goal deactivation completed: {}", goal_id);

        Ok(())
    }

    pub fn get_goal(&self, goal_id: &Uuid) -> Option<&Goal> {
        self.goals_cache.get(goal_id)
    }

    pub fn get_all_goals(&self) -> Vec<&Goal> {
        let goals = self.goals_cache.values().collect::<Vec<_>>();
        println!("Getting all goals, found {} goals", goals.len());
        for goal in &goals {
            println!("Goal: {} - Active: {}", goal.name, goal.is_active);
        }
        goals
    }

    pub fn get_active_goal(&self) -> Option<&Goal> {
        self.active_goal_id.and_then(|id| self.goals_cache.get(&id))
    }

    pub fn get_active_goal_info(&self) -> Option<(Uuid, Vec<String>)> {
        // Always return a goal - either the active one or the default Master Goal
        let goal_id = self.get_current_or_default_goal_id();

        self.goals_cache
            .get(&goal_id)
            .map(|goal| (goal.id, goal.allowed_apps.clone()))
            .or_else(|| {
                // Fallback: if for some reason the goal isn't in cache, return default with all apps allowed
                println!(
                    "Warning: Goal {} not found in cache, using fallback",
                    goal_id
                );
                Some((goal_id, vec![]))
            })
    }

    pub async fn update_active_goal_progress(
        &mut self,
        app_name: &str,
        minutes: u32,
    ) -> Result<()> {
        if let Some(goal_id) = self.active_goal_id {
            let should_update = self
                .goals_cache
                .get(&goal_id)
                .map(|goal| goal.is_app_allowed(app_name))
                .unwrap_or(false);

            if should_update {
                if let Some(goal) = self.goals_cache.get_mut(&goal_id) {
                    goal.update_progress(minutes);
                }
                // Save to database
                if let Some(goal) = self.goals_cache.get(&goal_id) {
                    self.save_goal(goal).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn update_active_goal_progress_seconds(
        &mut self,
        app_name: &str,
        seconds: u32,
    ) -> Result<()> {
        // Always use the current or default goal
        let goal_id = self.get_current_or_default_goal_id();

        let should_update = self
            .goals_cache
            .get(&goal_id)
            .map(|goal| goal.is_app_allowed(app_name))
            .unwrap_or(false);

        if should_update {
            if let Some(goal) = self.goals_cache.get_mut(&goal_id) {
                goal.update_progress_seconds(seconds);
            }
            // Save to database
            if let Some(goal) = self.goals_cache.get(&goal_id) {
                self.save_goal(goal).await?;
            }
        }

        Ok(())
    }

    pub fn get_goal_sessions(&self, goal_id: &Uuid) -> Option<&Vec<GoalSession>> {
        self.sessions.get(goal_id)
    }
}
