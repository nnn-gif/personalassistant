use crate::error::{AppError, Result};
use crate::models::{SavedResearchTask, ResearchTask, Goal, GoalSession, Activity};
use surrealdb::Surreal;
use surrealdb::engine::any::{connect, Any};
use uuid::Uuid;

pub struct Database {
    db: Surreal<Any>,
}

impl Database {
    pub async fn new(namespace: &str) -> Result<Self> {
        let db = connect("mem://").await?;
        
        db.use_ns(namespace).use_db("assistant").await?;
        
        // Create tables
        db.query("DEFINE TABLE research_tasks").await?;
        db.query("DEFINE TABLE goals").await?;
        db.query("DEFINE TABLE goal_sessions").await?;
        db.query("DEFINE TABLE activities").await?;
        
        Ok(Self { db })
    }
    
    // Research task methods
    pub async fn save_research_task(&self, task: SavedResearchTask) -> Result<SavedResearchTask> {
        let created: Option<SavedResearchTask> = self.db
            .create("research_tasks")
            .content(task)
            .await?;
        
        created.ok_or_else(|| AppError::Database("Failed to save research task".into()))
    }
    
    pub async fn get_research_task(&self, id: Uuid) -> Result<Option<SavedResearchTask>> {
        let result: Option<SavedResearchTask> = self.db
            .select(("research_tasks", id.to_string()))
            .await?;
        
        Ok(result)
    }
    
    pub async fn get_all_research_tasks(&self) -> Result<Vec<SavedResearchTask>> {
        let result: Vec<SavedResearchTask> = self.db
            .select("research_tasks")
            .await?;
        
        Ok(result)
    }
    
    pub async fn search_research_tasks(&self, query: &str) -> Result<Vec<SavedResearchTask>> {
        let query_string = query.to_string();
        let result: Vec<SavedResearchTask> = self.db
            .query("SELECT * FROM research_tasks WHERE task.query CONTAINS $query OR notes CONTAINS $query OR $query IN tags")
            .bind(("query", query_string))
            .await?
            .take(0)?;
        
        Ok(result)
    }
    
    // Goal methods
    pub async fn save_goal(&self, goal: Goal) -> Result<Goal> {
        let created: Option<Goal> = self.db
            .create("goals")
            .content(goal)
            .await?;
        
        created.ok_or_else(|| AppError::Database("Failed to save goal".into()))
    }
    
    pub async fn update_goal(&self, goal: Goal) -> Result<Goal> {
        let updated: Option<Goal> = self.db
            .update(("goals", goal.id.to_string()))
            .content(goal)
            .await?;
        
        updated.ok_or_else(|| AppError::Database("Failed to update goal".into()))
    }
    
    pub async fn get_goal(&self, id: Uuid) -> Result<Option<Goal>> {
        let result: Option<Goal> = self.db
            .select(("goals", id.to_string()))
            .await?;
        
        Ok(result)
    }
    
    pub async fn get_all_goals(&self) -> Result<Vec<Goal>> {
        let result: Vec<Goal> = self.db
            .select("goals")
            .await?;
        
        Ok(result)
    }
    
    pub async fn get_active_goal(&self) -> Result<Option<Goal>> {
        let result: Vec<Goal> = self.db
            .query("SELECT * FROM goals WHERE is_active = true LIMIT 1")
            .await?
            .take(0)?;
        
        Ok(result.into_iter().next())
    }
    
    // Goal session methods
    pub async fn save_goal_session(&self, session: GoalSession) -> Result<GoalSession> {
        let created: Option<GoalSession> = self.db
            .create("goal_sessions")
            .content(session)
            .await?;
        
        created.ok_or_else(|| AppError::Database("Failed to save goal session".into()))
    }
    
    pub async fn get_goal_sessions(&self, goal_id: Uuid) -> Result<Vec<GoalSession>> {
        let result: Vec<GoalSession> = self.db
            .query("SELECT * FROM goal_sessions WHERE goal_id = $goal_id")
            .bind(("goal_id", goal_id))
            .await?
            .take(0)?;
        
        Ok(result)
    }
    
    // Activity methods
    pub async fn save_activity(&self, activity: Activity) -> Result<Activity> {
        let created: Option<Activity> = self.db
            .create("activities")
            .content(activity)
            .await?;
        
        created.ok_or_else(|| AppError::Database("Failed to save activity".into()))
    }
    
    pub async fn get_recent_activities(&self, limit: usize) -> Result<Vec<Activity>> {
        let result: Vec<Activity> = self.db
            .query("SELECT * FROM activities ORDER BY timestamp DESC LIMIT $limit")
            .bind(("limit", limit))
            .await?
            .take(0)?;
        
        Ok(result)
    }
}