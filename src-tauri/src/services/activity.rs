use crate::activity_tracking::ActivityTracker;
use crate::database::SqliteDatabase;
use crate::error::Result;
use crate::models::Activity;
use chrono::{Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[tauri::command]
pub async fn get_current_activity(
    tracker: State<'_, Arc<Mutex<ActivityTracker>>>,
) -> Result<Option<Activity>> {
    let tracker = tracker.lock().await;
    Ok(tracker.get_current_activity().cloned())
}

#[tauri::command]
pub async fn get_activity_history(
    tracker: State<'_, Arc<Mutex<ActivityTracker>>>,
    limit: Option<usize>,
) -> Result<Vec<Activity>> {
    let tracker = tracker.lock().await;
    let limit = limit.unwrap_or(50);

    Ok(tracker.get_recent_activities(limit))
}

#[tauri::command]
pub async fn start_tracking(tracker: State<'_, Arc<Mutex<ActivityTracker>>>) -> Result<()> {
    let mut tracker = tracker.lock().await;
    tracker.start_tracking().await
}

#[tauri::command]
pub async fn stop_tracking(tracker: State<'_, Arc<Mutex<ActivityTracker>>>) -> Result<()> {
    let mut tracker = tracker.lock().await;
    tracker.stop_tracking().await
}

#[tauri::command]
pub async fn get_tracking_stats(
    tracker: State<'_, Arc<Mutex<ActivityTracker>>>,
) -> Result<TrackingStats> {
    let tracker = tracker.lock().await;
    let recent_activities = tracker.get_recent_activities(100);
    let (productive_time, total_time) = tracker.get_productivity_stats(24);

    Ok(TrackingStats {
        is_tracking: tracker.is_tracking(),
        total_activities: recent_activities.len(),
        productive_time_seconds: productive_time,
        total_time_seconds: total_time,
        last_activity: tracker.get_current_activity().cloned(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackingStats {
    pub is_tracking: bool,
    pub total_activities: usize,
    pub productive_time_seconds: u32,
    pub total_time_seconds: u32,
    pub last_activity: Option<Activity>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodayStats {
    pub total_tracked_seconds: u32,
    pub active_time_seconds: u32,
    pub productivity_score: f64,
    pub top_apps: Vec<AppUsageStats>,
    pub hourly_breakdown: Vec<HourlyBreakdown>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppUsageStats {
    pub app_name: String,
    pub total_seconds: u32,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HourlyBreakdown {
    pub hour: u32,
    pub productive_seconds: u32,
    pub total_seconds: u32,
}

#[tauri::command]
pub async fn get_today_stats(db: State<'_, Arc<Mutex<SqliteDatabase>>>) -> Result<TodayStats> {
    let db = db.lock().await;

    // Get today's activities
    let today_start = Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let now = Utc::now();

    println!("[get_today_stats] Fetching activities for today");
    println!("[get_today_stats] Today start: {}", today_start.format("%Y-%m-%d %H:%M:%S"));
    println!("[get_today_stats] Current time: {}", now.format("%Y-%m-%d %H:%M:%S"));

    let start_time = std::time::Instant::now();
    let activities = db.get_activities_by_date_range(today_start, now).await?;
    let fetch_duration = start_time.elapsed();
    
    println!("[get_today_stats] Fetched {} activities in {:?}", activities.len(), fetch_duration);

    let mut total_seconds = 0u32;
    let mut productive_seconds = 0u32;
    let mut app_breakdown: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for activity in &activities {
        total_seconds += activity.duration_seconds as u32;
        if activity.app_usage.is_productive {
            productive_seconds += activity.duration_seconds as u32;
        }
        
        // Track app usage
        *app_breakdown.entry(activity.app_usage.app_name.clone()).or_insert(0) += activity.duration_seconds as u32;
    }
    
    // Log top 5 apps by usage
    if !app_breakdown.is_empty() {
        let mut app_vec: Vec<_> = app_breakdown.iter().collect();
        app_vec.sort_by(|a, b| b.1.cmp(a.1));
        
        println!("[get_today_stats] Top apps by usage:");
        for (i, (app, seconds)) in app_vec.iter().take(5).enumerate() {
            println!("  {}. {} - {} minutes", i + 1, app, *seconds / 60);
        }
    }

    // Calculate top apps
    let total_app_seconds = app_breakdown.values().sum::<u32>() as f64;
    let mut top_apps: Vec<AppUsageStats> = app_breakdown
        .into_iter()
        .map(|(app_name, total_seconds)| AppUsageStats {
            app_name,
            total_seconds,
            percentage: if total_app_seconds > 0.0 {
                (total_seconds as f64 / total_app_seconds) * 100.0
            } else {
                0.0
            },
        })
        .collect();
    top_apps.sort_by(|a, b| b.total_seconds.cmp(&a.total_seconds));
    top_apps.truncate(5); // Keep only top 5 apps

    // Calculate hourly breakdown
    let mut hourly_stats: std::collections::HashMap<u32, (u32, u32)> = std::collections::HashMap::new();
    for activity in &activities {
        let hour = activity.timestamp.hour();
        let entry = hourly_stats.entry(hour).or_insert((0, 0));
        entry.1 += activity.duration_seconds as u32; // total seconds
        if activity.app_usage.is_productive {
            entry.0 += activity.duration_seconds as u32; // productive seconds
        }
    }
    
    let hourly_breakdown: Vec<HourlyBreakdown> = (0..24)
        .map(|hour| {
            let (productive, total) = hourly_stats.get(&hour).unwrap_or(&(0, 0));
            HourlyBreakdown {
                hour,
                productive_seconds: *productive,
                total_seconds: *total,
            }
        })
        .collect();

    // Calculate productivity score (percentage of productive time)
    let productivity_score = if total_seconds > 0 {
        (productive_seconds as f64 / total_seconds as f64) * 100.0
    } else {
        0.0
    };

    let stats = TodayStats {
        total_tracked_seconds: total_seconds,
        active_time_seconds: total_seconds, // For now, treating all tracked time as active
        productivity_score,
        top_apps,
        hourly_breakdown,
    };
    
    println!("[get_today_stats] Summary: {} total seconds ({:.2} hours), {:.1}% productive, {} activities",
             stats.total_tracked_seconds, 
             stats.total_tracked_seconds as f64 / 3600.0,
             stats.productivity_score, 
             activities.len());
    
    Ok(stats)
}
