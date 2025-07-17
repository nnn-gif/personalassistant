use super::{
    ActivityAggregator, ActivityCache, AppWatcher, InputMonitor, ProjectDetector,
    SharedBatchWriter, SystemMonitor,
};
use crate::database::SqliteDatabase;
use crate::error::Result;
use crate::models::Activity;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct OptimizedActivityTracker {
    app_watcher: AppWatcher,
    project_detector: ProjectDetector,
    system_monitor: SystemMonitor,
    input_monitor: InputMonitor,
    aggregator: ActivityAggregator,
    cache: ActivityCache,
    batch_writer: Option<SharedBatchWriter>,
    is_tracking: bool,
    stats: TrackerStats,
}

#[derive(Debug, Default)]
pub struct TrackerStats {
    pub activities_collected: u64,
    pub activities_aggregated: u64,
    pub activities_saved: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl OptimizedActivityTracker {
    pub fn new() -> Self {
        Self {
            app_watcher: AppWatcher::new(),
            project_detector: ProjectDetector::new(),
            system_monitor: SystemMonitor::new(),
            input_monitor: InputMonitor::new(),
            aggregator: ActivityAggregator::new(),
            cache: ActivityCache::new(1000), // Cache last 1000 activities (~1.4 hours)
            batch_writer: None,
            is_tracking: true,
            stats: TrackerStats::default(),
        }
    }

    pub fn set_database(&mut self, db: Arc<Mutex<SqliteDatabase>>) {
        let batch_writer = SharedBatchWriter::new(db);
        // Start periodic flush task
        batch_writer.start_periodic_flush();
        self.batch_writer = Some(batch_writer);
    }

    pub async fn start_tracking(&mut self) -> Result<()> {
        self.is_tracking = true;
        println!("[OptimizedTracker] Activity tracking started");
        Ok(())
    }

    pub async fn stop_tracking(&mut self) -> Result<()> {
        self.is_tracking = false;
        
        // Flush any pending aggregated activity
        if let Some(activity) = self.aggregator.flush() {
            self.save_activity(activity).await;
        }
        
        // Flush batch writer
        if let Some(writer) = &self.batch_writer {
            writer.flush().await?;
        }
        
        println!("[OptimizedTracker] Activity tracking stopped");
        self.print_stats();
        Ok(())
    }

    pub fn is_tracking(&self) -> bool {
        self.is_tracking
    }

    pub async fn collect_activity(
        &mut self,
        active_goal: Option<(Uuid, Vec<String>)>,
    ) -> Result<()> {
        if !self.is_tracking {
            return Ok(());
        }

        // Collect current activity data
        let app_usage = self.app_watcher.get_current_app()?;
        let project_context = self
            .project_detector
            .detect_project(&app_usage.app_name, &app_usage.window_title)?;
        let input_metrics = self.input_monitor.get_metrics_and_reset()?;
        let system_state = self.system_monitor.get_system_state()?;

        // Check goal association
        let goal_id = if let Some((goal_id, allowed_apps)) = active_goal {
            if allowed_apps
                .iter()
                .any(|app| app.eq_ignore_ascii_case(&app_usage.app_name))
            {
                Some(goal_id)
            } else {
                None
            }
        } else {
            None
        };

        let activity = Activity {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            duration_seconds: 5,
            app_usage,
            input_metrics,
            system_state,
            project_context,
            goal_id,
        };

        self.stats.activities_collected += 1;

        // Process through aggregator
        if let Some(completed_activity) = self.aggregator.process_activity(activity.clone()) {
            // Aggregation completed, save the aggregated activity
            self.stats.activities_aggregated += 1;
            self.save_activity(completed_activity).await;
        }

        // Always add to cache for real-time queries
        self.cache.add_activity(activity);

        Ok(())
    }

    async fn save_activity(&mut self, activity: Activity) {
        if let Some(writer) = &self.batch_writer {
            writer.add_activity(activity).await;
            self.stats.activities_saved += 1;
        }
    }

    /// Get activities from cache if possible, otherwise from database
    pub async fn get_activities_by_date_range(
        &mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        db: &Arc<Mutex<SqliteDatabase>>,
    ) -> Result<Vec<Activity>> {
        println!("[OptimizedTracker] get_activities_by_date_range called");
        println!("[OptimizedTracker] Start: {}, End: {}", start.format("%Y-%m-%d %H:%M:%S"), end.format("%Y-%m-%d %H:%M:%S"));
        
        // Check if cache can serve this request
        if self.cache.covers_range(start, end) {
            self.stats.cache_hits += 1;
            println!(
                "[OptimizedTracker] Serving {} to {} from cache",
                start.format("%H:%M:%S"),
                end.format("%H:%M:%S")
            );
            Ok(self.cache.get_activities_in_range(start, end))
        } else {
            self.stats.cache_misses += 1;
            println!(
                "[OptimizedTracker] Cache miss for {} to {}, fetching from database",
                start.format("%H:%M:%S"),
                end.format("%H:%M:%S")
            );
            
            println!("[OptimizedTracker] Attempting to lock database...");
            // Fetch from database
            let db = db.lock().await;
            println!("[OptimizedTracker] Database locked, executing query...");
            
            let activities = db.get_activities_by_date_range(start, end).await?;
            println!("[OptimizedTracker] Query completed, got {} activities", activities.len());
            
            // Add to cache for future queries
            for activity in &activities {
                self.cache.add_activity(activity.clone());
            }
            
            Ok(activities)
        }
    }

    /// Get current statistics
    pub fn get_stats(&self) -> &TrackerStats {
        &self.stats
    }

    /// Print statistics
    pub fn print_stats(&self) {
        println!("\n[OptimizedTracker] Statistics:");
        println!("  Activities collected: {}", self.stats.activities_collected);
        println!("  Activities aggregated: {}", self.stats.activities_aggregated);
        println!("  Activities saved: {}", self.stats.activities_saved);
        println!(
            "  Aggregation ratio: {:.1}%",
            if self.stats.activities_collected > 0 {
                (self.stats.activities_aggregated as f64 / self.stats.activities_collected as f64)
                    * 100.0
            } else {
                0.0
            }
        );
        println!("  Cache hits: {}", self.stats.cache_hits);
        println!("  Cache misses: {}", self.stats.cache_misses);
        println!(
            "  Cache hit rate: {:.1}%",
            if self.stats.cache_hits + self.stats.cache_misses > 0 {
                (self.stats.cache_hits as f64
                    / (self.stats.cache_hits + self.stats.cache_misses) as f64)
                    * 100.0
            } else {
                0.0
            }
        );
        
        let cache_stats = self.cache.stats();
        println!("\n  Cache status:");
        println!("    Activities in cache: {}", cache_stats.activity_count);
        if let Some(oldest) = cache_stats.oldest_timestamp {
            println!("    Oldest: {}", oldest.format("%Y-%m-%d %H:%M:%S"));
        }
        if let Some(newest) = cache_stats.newest_timestamp {
            println!("    Newest: {}", newest.format("%Y-%m-%d %H:%M:%S"));
        }
        println!(
            "    Duration covered: {:.1} hours",
            cache_stats.total_duration_seconds as f64 / 3600.0
        );
    }
}