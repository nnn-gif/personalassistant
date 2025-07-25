#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod activity_tracking;
mod audio;
mod browser_ai;
mod config;
mod database;
mod error;
mod goals;
mod init;
mod llm;
mod models;
mod rag;
mod services;
mod storage;

use init::AppServices;
use tauri::{generate_context, generate_handler, Manager};

fn main() {
    // Load .env file if it exists
    dotenv::dotenv().ok();
    
    // Configure logging to reduce chromiumoxide noise
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    // Set default log levels
                    "personalassistant=debug,chromiumoxide=warn,chromiumoxide::conn=error,chromiumoxide::handler=error".into()
                })
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Load configuration
            if let Err(e) = config::Config::load(&app.handle()) {
                tracing::error!("Failed to load configuration: {}", e);
            }
            
            // Validate configuration
            let cfg = config::Config::get();
            if let Err(errors) = cfg.validate() {
                tracing::warn!("Configuration validation errors: {:?}", errors);
            }
            
            let services = tauri::async_runtime::block_on(async {
                match AppServices::initialize(app).await {
                    Ok(services) => services,
                    Err(e) => {
                        tracing::error!("Failed to initialize app services: {}", e);
                        panic!("Failed to initialize app services: {}", e);
                    }
                }
            });

            // Start background tasks
            services.spawn_activity_tracking();
            services.spawn_migration();
            
            // Show the main window
            println!("App setup complete");
            if let Some(window) = app.get_webview_window("main") {
                println!("Main window found, showing...");
                window.show().unwrap();
                window.set_focus().unwrap();
            } else {
                println!("Main window not found!");
            }

            Ok(())
        })
        .invoke_handler(generate_handler![
            // Activity tracking commands
            services::activity::get_current_activity,
            services::activity::get_activity_history,
            services::activity::start_tracking,
            services::activity::stop_tracking,
            services::activity::get_tracking_stats,
            services::activity::get_today_stats,
            services::activity_flush::flush_pending_activities,
            // Browser AI commands
            services::browser_ai::test_research,
            services::browser_ai::start_research,
            services::browser_ai::get_research_status,
            services::browser_ai::save_research,
            services::browser_ai::get_saved_research,
            services::browser_ai::delete_saved_research,
            // Chat commands
            services::chat::create_chat_conversation,
            services::chat::save_chat_message,
            services::chat::get_chat_conversations,
            services::chat::get_chat_messages,
            services::chat::delete_chat_conversation,
            services::chat::update_chat_conversation_title,
            // Config commands
            services::config::get_config,
            services::config::update_config,
            services::config::get_user_preferences,
            services::config::update_user_preferences,
            services::config::reset_preferences,
            // Goal commands
            services::goals::create_goal,
            services::goals::update_goal,
            services::goals::activate_goal,
            services::goals::deactivate_goal,
            services::goals::get_goals,
            services::goals::get_goal_progress,
            // LLM commands
            services::llm::get_productivity_insights,
            services::llm::get_productivity_score,
            services::llm::get_recommendations,
            services::llm::chat_with_documents,
            services::llm::get_available_models,
            services::llm::general_chat,
            // Embedding commands
            services::embeddings::test_embeddings,
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
            services::rag::index_document_async,
            services::rag::search_documents,
            services::rag::get_goal_context,
            services::rag::list_indexed_documents,
            services::rag::remove_document,
            services::rag::update_document_index,
            services::rag::get_supported_file_types,
            services::rag::check_file_supported,
            services::rag::get_enhanced_file_info,
            services::rag::inspect_rag_database,
            services::rag::cleanup_corrupted_documents,
            services::rag::clear_vector_database,
            // Streaming chat commands
            services::streaming_chat::stream_general_chat,
            services::streaming_chat::stream_document_chat,
            // File manager commands
            services::file_manager::scan_folder_for_documents,
            services::file_manager::get_file_info,
            services::file_manager::index_multiple_documents,
            services::file_manager::get_folder_stats,
            // Inference commands
            services::inference::get_inference_config,
            services::inference::set_inference_provider,
            services::inference::get_inference_info,
            services::inference::get_candle_models,
            services::inference::get_config_path,
            services::inference::download_model,
        ])
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { .. } => {
                println!("Window close requested");
            }
            _ => {}
        })
        .run(generate_context!())
        .expect("error while running tauri application");
}
