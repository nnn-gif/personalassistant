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
mod rag;
mod services;
mod storage;

use error::AppError;
use std::sync::Arc;
use tauri::{generate_context, generate_handler, Manager};
use database::SqliteDatabase;
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
            
            let llm_client = llm::LlmClient::new();
            app.manage(Arc::new(llm_client));
            
            // Initialize RAG system
            let rag_system = tauri::async_runtime::block_on(async {
                match rag::RAGSystem::new().await {
                    Ok(rag) => {
                        println!("RAG system initialized successfully");
                        Some(Arc::new(tokio::sync::Mutex::new(rag)))
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize RAG system: {}", e);
                        None
                    }
                }
            });
            
            if let Some(rag) = rag_system {
                app.manage(rag);
            }
            
            match audio::SimpleAudioRecorder::new() {
                Ok(recorder) => {
                    app.manage(Arc::new(recorder));
                },
                Err(e) => {
                    eprintln!("Failed to initialize audio recorder: {}", e);
                }
            }
            
            // Initialize SQLite database
            let db = tauri::async_runtime::block_on(async {
                match SqliteDatabase::new().await {
                    Ok(db) => {
                        println!("SQLite database initialized successfully");
                        Some(Arc::new(tokio::sync::Mutex::new(db)))
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize SQLite database: {}", e);
                        None
                    }
                }
            });
            
            if let Some(db) = db.clone() {
                app.manage(db.clone());
            }
            
            // Initialize storage (for migration purposes)
            let storage = match LocalStorage::new() {
                Ok(storage) => {
                    println!("Local storage initialized successfully");
                    Some(storage)
                }
                Err(e) => {
                    eprintln!("Failed to initialize local storage: {}", e);
                    None
                }
            };
            
            // Initialize goal service with database
            let goal_service = Arc::new(tokio::sync::Mutex::new(goals::GoalService::new()));
            if let Some(db) = &db {
                let mut service = goal_service.blocking_lock();
                service.set_database(db.clone());
                // Load existing goals
                tauri::async_runtime::block_on(async {
                    if let Err(e) = service.load_from_database().await {
                        eprintln!("Failed to load goals from database: {}", e);
                    } else {
                        println!("Goals loaded from database successfully");
                    }
                });
            }
            app.manage(goal_service.clone());
            
            // Update activity tracker to use database
            if let Some(db) = &db {
                let mut tracker = activity_tracker.blocking_lock();
                tracker.set_database(db.clone());
            }
            
            // Migrate existing data if needed
            if let (Some(db), Some(storage)) = (db, storage) {
                tauri::async_runtime::spawn(async move {
                    let db = db.lock().await;
                    if let Err(e) = db.import_from_storage(&storage).await {
                        eprintln!("Failed to migrate data from storage: {}", e);
                    } else {
                        println!("Successfully migrated data to SQLite");
                    }
                });
            }
            
            // Start activity tracking background task
            let tracker_clone = activity_tracker.clone();
            let goal_service_clone = goal_service.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    let mut tracker = tracker_clone.lock().await;
                    if tracker.is_tracking() {
                        // Get active goal info
                        let active_goal_info = {
                            let goal_service = goal_service_clone.lock().await;
                            goal_service.get_active_goal_info()
                        };
                        
                        match tracker.collect_activity(active_goal_info).await {
                            Ok(activity) => {
                                let goal_msg = if activity.goal_id.is_some() { 
                                    " [Goal tracked]" 
                                } else { 
                                    "" 
                                };
                                println!("Activity collected: {} - {}{}", 
                                    activity.app_usage.app_name, 
                                    activity.app_usage.window_title,
                                    goal_msg
                                );
                                
                                // Update goal progress if activity is part of active goal
                                if activity.goal_id.is_some() {
                                    let mut goal_service = goal_service_clone.lock().await;
                                    let _ = goal_service.update_active_goal_progress_seconds(
                                        &activity.app_usage.app_name, 
                                        activity.duration_seconds as u32
                                    );
                                }
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
            
            // RAG commands
            services::rag::initialize_rag,
            services::rag::index_document,
            services::rag::search_documents,
            services::rag::get_goal_context,
            services::rag::list_indexed_documents,
            services::rag::remove_document,
            services::rag::update_document_index,
            services::rag::get_supported_file_types,
            services::rag::check_file_supported,
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}