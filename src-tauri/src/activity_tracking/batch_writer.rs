use crate::database::SqliteDatabase;
use crate::error::Result;
use crate::models::Activity;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

/// Batches activities for efficient database writes
pub struct BatchWriter {
    buffer: Vec<Activity>,
    max_batch_size: usize,
    flush_interval_secs: u64,
    db: Arc<Mutex<SqliteDatabase>>,
}

impl BatchWriter {
    pub fn new(db: Arc<Mutex<SqliteDatabase>>) -> Self {
        Self {
            buffer: Vec::with_capacity(20),
            max_batch_size: 20,
            flush_interval_secs: 30,
            db,
        }
    }

    /// Add an activity to the batch
    pub fn add_activity(&mut self, activity: Activity) {
        self.buffer.push(activity);
        
        // Flush immediately if batch is full
        if self.buffer.len() >= self.max_batch_size {
            let activities = std::mem::take(&mut self.buffer);
            self.spawn_flush(activities);
        }
    }

    /// Start the periodic flush task
    pub fn start_periodic_flush(&self) -> tokio::task::JoinHandle<()> {
        let db = self.db.clone();
        let flush_interval = self.flush_interval_secs;
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(flush_interval));
            let mut buffer = Vec::new();
            
            loop {
                interval.tick().await;
                
                // Swap buffers to minimize lock time
                if !buffer.is_empty() {
                    let activities = std::mem::take(&mut buffer);
                    if let Err(e) = Self::flush_to_db(&db, activities).await {
                        eprintln!("[BatchWriter] Failed to flush activities: {}", e);
                    }
                }
            }
        })
    }

    /// Force flush all pending activities
    pub async fn flush(&mut self) -> Result<()> {
        if !self.buffer.is_empty() {
            let activities = std::mem::take(&mut self.buffer);
            Self::flush_to_db(&self.db, activities).await
        } else {
            Ok(())
        }
    }

    /// Spawn an async task to flush activities
    fn spawn_flush(&self, activities: Vec<Activity>) {
        let db = self.db.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::flush_to_db(&db, activities).await {
                eprintln!("[BatchWriter] Failed to flush activities: {}", e);
            }
        });
    }

    /// Write activities to database in a batch
    async fn flush_to_db(db: &Arc<Mutex<SqliteDatabase>>, activities: Vec<Activity>) -> Result<()> {
        if activities.is_empty() {
            return Ok(());
        }

        let count = activities.len();
        let start = std::time::Instant::now();
        
        println!("[BatchWriter] Flushing {} activities to database", count);
        
        let db = db.lock().await;
        
        // TODO: Implement batch insert for better performance
        // For now, save individually but in a single transaction would be better
        for activity in activities {
            db.save_activity(&activity).await?;
        }
        
        let duration = start.elapsed();
        println!(
            "[BatchWriter] Successfully flushed {} activities in {:?} ({:.2} activities/sec)",
            count,
            duration,
            count as f64 / duration.as_secs_f64()
        );
        
        Ok(())
    }
}

/// Thread-safe wrapper for BatchWriter
pub struct SharedBatchWriter {
    inner: Arc<Mutex<BatchWriter>>,
}

impl SharedBatchWriter {
    pub fn new(db: Arc<Mutex<SqliteDatabase>>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BatchWriter::new(db))),
        }
    }

    pub async fn add_activity(&self, activity: Activity) {
        let mut writer = self.inner.lock().await;
        writer.add_activity(activity);
    }

    pub async fn flush(&self) -> Result<()> {
        let mut writer = self.inner.lock().await;
        writer.flush().await
    }

    pub fn start_periodic_flush(&self) -> tokio::task::JoinHandle<()> {
        let writer = self.inner.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                let mut w = writer.lock().await;
                if let Err(e) = w.flush().await {
                    eprintln!("[SharedBatchWriter] Periodic flush failed: {}", e);
                }
            }
        })
    }
}