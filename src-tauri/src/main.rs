#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod activity_tracking;
mod browser_ai;
// mod database; // Temporarily disabled
mod error;
mod goals;
mod llm;
mod models;
mod services;

use error::AppError;
use std::sync::Arc;
use tauri::{generate_context, generate_handler, Manager};

fn main() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize services synchronously
            let activity_tracker = activity_tracking::ActivityTracker::new();
            app.manage(Arc::new(tokio::sync::Mutex::new(activity_tracker)));
            
            let browser_ai = browser_ai::BrowserAIAgent::new();
            app.manage(Arc::new(tokio::sync::Mutex::new(browser_ai)));
            
            let goal_service = goals::GoalService::new();
            app.manage(Arc::new(tokio::sync::Mutex::new(goal_service)));
            
            let llm_client = llm::LlmClient::new();
            app.manage(Arc::new(llm_client));
            
            Ok(())
        })
        .invoke_handler(generate_handler![
            // Activity tracking commands
            services::activity::get_current_activity,
            services::activity::get_activity_history,
            services::activity::start_tracking,
            services::activity::stop_tracking,
            
            // Browser AI commands
            services::browser_ai::test_research,
            services::browser_ai::start_research,
            services::browser_ai::get_research_status,
            services::browser_ai::save_research,
            services::browser_ai::get_saved_research,
            
            // Goal commands
            services::goals::create_goal,
            services::goals::activate_goal,
            services::goals::deactivate_goal,
            services::goals::get_goals,
            services::goals::get_goal_progress,
            
            // LLM commands
            services::llm::get_productivity_insights,
            services::llm::get_productivity_score,
            services::llm::get_recommendations,
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}