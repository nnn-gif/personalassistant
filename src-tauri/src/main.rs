#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod activity_tracking;
mod audio;
mod browser_ai;
mod database;
mod error;
mod goals;
mod llm;
mod models;
mod services;
mod storage;

use error::AppError;
use std::sync::Arc;
use tauri::{generate_context, generate_handler, Manager};
use database::Database;
use storage::LocalStorage;

fn main() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize services synchronously
            let activity_tracker = Arc::new(tokio::sync::Mutex::new(
                activity_tracking::ActivityTracker::new()
            ));
            app.manage(activity_tracker.clone());
            
            let browser_ai = browser_ai::BrowserAIAgent::new();
            app.manage(Arc::new(tokio::sync::Mutex::new(browser_ai)));
            
            let goal_service = goals::GoalService::new();
            app.manage(Arc::new(tokio::sync::Mutex::new(goal_service)));
            
            let llm_client = llm::LlmClient::new();
            app.manage(Arc::new(llm_client));
            
            match audio::SimpleAudioRecorder::new() {
                Ok(recorder) => {
                    app.manage(Arc::new(recorder));
                },
                Err(e) => {
                    eprintln!("Failed to initialize audio recorder: {}", e);
                }
            }
            
            // Initialize storage and database
            match LocalStorage::new() {
                Ok(storage) => {
                    app.manage(Arc::new(tokio::sync::Mutex::new(storage)));
                    println!("Local storage initialized successfully");
                }
                Err(e) => {
                    eprintln!("Failed to initialize local storage: {}", e);
                }
            }
            
            // Initialize database synchronously in a blocking task
            let (tx, rx) = std::sync::mpsc::channel();
            tauri::async_runtime::spawn(async move {
                match Database::new("personalassistant").await {
                    Ok(db) => {
                        tx.send(Ok(db)).unwrap();
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize database: {}", e);
                        tx.send(Err(e)).unwrap();
                    }
                }
            });
            
            // Wait for database initialization
            match rx.recv() {
                Ok(Ok(db)) => {
                    app.manage(Arc::new(tokio::sync::Mutex::new(db)));
                    println!("Database initialized successfully");
                }
                Ok(Err(e)) => {
                    eprintln!("Database initialization failed: {}", e);
                    // Continue without database - commands will handle the missing state
                }
                Err(e) => {
                    eprintln!("Failed to receive database initialization result: {}", e);
                }
            }
            
            // Start activity tracking background task
            let tracker_clone = activity_tracker.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    let mut tracker = tracker_clone.lock().await;
                    if tracker.is_tracking() {
                        match tracker.collect_activity().await {
                            Ok(activity) => {
                                println!("Activity collected: {} - {}", 
                                    activity.app_usage.app_name, 
                                    activity.app_usage.window_title
                                );
                            }
                            Err(e) => {
                                eprintln!("Failed to collect activity: {}", e);
                            }
                        }
                    }
                }
            });
            
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
            services::browser_ai::delete_saved_research,
            
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
            
            // Productivity commands
            services::productivity::get_productivity_trend,
            services::productivity::get_app_usage_stats,
            services::productivity::get_current_productivity_score,
            
            // Audio commands
            services::audio::list_audio_devices,
            services::audio::start_audio_recording,
            services::audio::stop_audio_recording,
            services::audio::pause_audio_recording,
            services::audio::resume_audio_recording,
            services::audio::get_recording_status,
            services::audio::get_recordings,
            services::audio::transcribe_recording,
            services::audio::generate_meeting_summary,
            services::audio::process_audio_file,
            services::audio::get_audio_info,
            services::audio::delete_recording,
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}