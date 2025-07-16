use crate::{
    activity_tracking::ActivityTracker,
    audio::SimpleAudioRecorder,
    browser_ai::BrowserAIAgent,
    database::SqliteDatabase,
    error::Result,
    goals::GoalService,
    llm::LlmClient,
    rag::{RAGSystem, RAGSystemWrapper},
    storage::LocalStorage,
};
use std::sync::Arc;
use tauri::{App, Manager};
use tokio::sync::Mutex;

pub struct AppServices {
    pub activity_tracker: Arc<Mutex<ActivityTracker>>,
    pub browser_ai: Arc<Mutex<BrowserAIAgent>>,
    pub llm_client: Arc<LlmClient>,
    pub audio_recorder: Option<Arc<SimpleAudioRecorder>>,
    pub database: Option<Arc<Mutex<SqliteDatabase>>>,
    pub rag_system: Option<Arc<Mutex<RAGSystemWrapper>>>,
    pub goal_service: Arc<Mutex<GoalService>>,
}

impl AppServices {
    pub async fn initialize(app: &App) -> Result<Self> {
        let activity_tracker = Arc::new(Mutex::new(ActivityTracker::new()));
        let browser_ai = Arc::new(Mutex::new(BrowserAIAgent::new()));
        let llm_client = Arc::new(LlmClient::new());

        let audio_recorder = match SimpleAudioRecorder::new() {
            Ok(recorder) => {
                tracing::info!("Audio recorder initialized successfully");
                Some(Arc::new(recorder))
            }
            Err(e) => {
                tracing::error!("Failed to initialize audio recorder: {}", e);
                None
            }
        };

        let database = match SqliteDatabase::new().await {
            Ok(db) => {
                tracing::info!("SQLite database initialized successfully");
                Some(Arc::new(Mutex::new(db)))
            }
            Err(e) => {
                tracing::error!("Failed to initialize SQLite database: {}", e);
                None
            }
        };

        let rag_system = match RAGSystem::new_with_automatic_fallback().await {
            Ok(mut rag_wrapper) => {
                tracing::info!("RAG system wrapper created successfully");

                if let Some(db) = &database {
                    tracing::info!("Setting database for RAG system");
                    rag_wrapper.set_database(db.clone()).await;

                    if let Err(e) = rag_wrapper.load_from_database().await {
                        tracing::error!("Failed to load documents from database: {}", e);
                    } else {
                        tracing::info!(
                            "RAG system initialized successfully with database persistence"
                        );
                    }
                } else {
                    tracing::info!("RAG system initialized without database");
                }

                Some(Arc::new(Mutex::new(rag_wrapper)))
            }
            Err(e) => {
                tracing::error!("Failed to initialize RAG system: {}", e);
                None
            }
        };

        let goal_service = Arc::new(Mutex::new(GoalService::new()));

        if let Some(db) = &database {
            let mut service = goal_service.lock().await;
            service.set_database(db.clone());

            if let Err(e) = service.load_from_database().await {
                tracing::error!("Failed to load goals from database: {}", e);
            } else {
                tracing::info!("Goals loaded from database successfully");
            }
        }

        if let Some(db) = &database {
            let mut tracker = activity_tracker.lock().await;
            tracker.set_database(db.clone());
        }

        let services = Self {
            activity_tracker,
            browser_ai,
            llm_client,
            audio_recorder,
            database,
            rag_system,
            goal_service,
        };

        services.register_with_app(app);

        Ok(services)
    }

    fn register_with_app(&self, app: &App) {
        app.manage(self.activity_tracker.clone());
        app.manage(self.browser_ai.clone());
        app.manage(self.llm_client.clone());

        if let Some(recorder) = &self.audio_recorder {
            app.manage(recorder.clone());
        }

        if let Some(db) = &self.database {
            app.manage(db.clone());
        }

        if let Some(rag) = &self.rag_system {
            app.manage(rag.clone());
        }

        app.manage(self.goal_service.clone());
    }

    pub fn spawn_activity_tracking(&self) {
        let tracker_clone = self.activity_tracker.clone();
        let goal_service_clone = self.goal_service.clone();

        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));

            loop {
                interval.tick().await;
                let mut tracker = tracker_clone.lock().await;

                if tracker.is_tracking() {
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

                            tracing::info!(
                                "Activity collected: {} - {}{}",
                                activity.app_usage.app_name,
                                activity.app_usage.window_title,
                                goal_msg
                            );

                            if activity.goal_id.is_some() {
                                let mut goal_service = goal_service_clone.lock().await;
                                let _ = goal_service.update_active_goal_progress_seconds(
                                    &activity.app_usage.app_name,
                                    activity.duration_seconds as u32,
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to collect activity: {}", e);
                        }
                    }
                }
            }
        });
    }

    pub fn spawn_migration(&self) {
        let database = self.database.clone();

        tauri::async_runtime::spawn(async move {
            if let Some(db) = database {
                match LocalStorage::new() {
                    Ok(storage) => {
                        tracing::info!("Local storage initialized successfully");
                        let db = db.lock().await;
                        if let Err(e) = db.import_from_storage(&storage).await {
                            tracing::error!("Failed to migrate data from storage: {}", e);
                        } else {
                            tracing::info!("Successfully migrated data to SQLite");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize local storage: {}", e);
                    }
                }
            }
        });
    }
}
