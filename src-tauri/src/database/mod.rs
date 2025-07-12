use crate::error::{AppError, Result};
use crate::models::{SavedResearchTask, Goal, GoalSession, Activity};
use surrealdb::{Surreal, engine::any::{connect, Any}};
use surrealdb::opt::auth::Root;
use uuid::Uuid;

pub struct Database {
    db: Surreal<Any>,
}

impl Database {
    pub async fn new(namespace: &str) -> Result<Self> {
        // Use in-memory SurrealDB for now
        // TODO: Enable file persistence when RocksDB is available
        println!("Connecting to in-memory SurrealDB");
        
        let db = connect("mem://").await
            .map_err(|e| AppError::Database(format!("Failed to connect to SurrealDB: {}", e)))?;
        
        // Sign in as root
        db.signin(Root {
            username: "root",
            password: "root",
        })
        .await
        .map_err(|e| AppError::Database(format!("Failed to sign in: {}", e)))?;
        
        // Select namespace and database
        db.use_ns(namespace).use_db("assistant").await
            .map_err(|e| AppError::Database(format!("Failed to select namespace/database: {}", e)))?;
        
        // Create tables with proper schemas
        db.query("DEFINE TABLE research_tasks SCHEMAFULL")
            .query("DEFINE FIELD id ON research_tasks TYPE uuid")
            .query("DEFINE FIELD task ON research_tasks TYPE object")
            .query("DEFINE FIELD tags ON research_tasks TYPE array")
            .query("DEFINE FIELD notes ON research_tasks TYPE option<string>")
            .query("DEFINE FIELD saved_at ON research_tasks TYPE datetime")
            .await
            .map_err(|e| AppError::Database(format!("Failed to create research_tasks table: {}", e)))?;
        
        db.query("DEFINE TABLE goals SCHEMAFULL")
            .query("DEFINE FIELD id ON goals TYPE uuid")
            .query("DEFINE FIELD title ON goals TYPE string")
            .query("DEFINE FIELD description ON goals TYPE option<string>")
            .query("DEFINE FIELD category ON goals TYPE string")
            .query("DEFINE FIELD target_hours ON goals TYPE number")
            .query("DEFINE FIELD is_active ON goals TYPE bool")
            .query("DEFINE FIELD created_at ON goals TYPE datetime")
            .query("DEFINE FIELD updated_at ON goals TYPE datetime")
            .await
            .map_err(|e| AppError::Database(format!("Failed to create goals table: {}", e)))?;
        
        db.query("DEFINE TABLE activities SCHEMAFULL")
            .query("DEFINE FIELD id ON activities TYPE uuid")
            .query("DEFINE FIELD timestamp ON activities TYPE datetime")
            .query("DEFINE FIELD duration_seconds ON activities TYPE number")
            .query("DEFINE FIELD app_usage ON activities TYPE object")
            .query("DEFINE FIELD input_metrics ON activities TYPE object")
            .query("DEFINE FIELD system_state ON activities TYPE object")
            .query("DEFINE FIELD project_context ON activities TYPE option<object>")
            .await
            .map_err(|e| AppError::Database(format!("Failed to create activities table: {}", e)))?;
        
        println!("Database initialized successfully");
        
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
        let query_lower = query.to_lowercase();
        let result: Vec<SavedResearchTask> = self.db
            .query("SELECT * FROM research_tasks WHERE string::lowercase(task.query) CONTAINS $query OR string::lowercase(notes) CONTAINS $query OR $query IN array::map(tags, |$tag| string::lowercase($tag))")
            .bind(("query", query_lower))
            .await?
            .take(0)?;
        
        Ok(result)
    }
    
    pub async fn delete_research_task(&self, id: Uuid) -> Result<()> {
        let _: Option<SavedResearchTask> = self.db
            .delete(("research_tasks", id.to_string()))
            .await?;
        
        Ok(())
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