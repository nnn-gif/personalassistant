use crate::models::Activity;
use std::collections::VecDeque;
use chrono::{DateTime, Utc, Duration};

const MAX_HISTORY_SIZE: usize = 10000; // Keep last 10k activities

pub struct ActivityHistory {
    activities: VecDeque<Activity>,
}

impl ActivityHistory {
    pub fn new() -> Self {
        Self {
            activities: VecDeque::with_capacity(MAX_HISTORY_SIZE),
        }
    }
    
    pub fn add_activity(&mut self, activity: Activity) {
        // Check if we should merge with the last activity
        if let Some(last_activity) = self.activities.back_mut() {
            // If same app and window title, merge the activities
            if last_activity.app_usage.app_name == activity.app_usage.app_name
                && last_activity.app_usage.window_title == activity.app_usage.window_title {
                // Update duration and metrics
                last_activity.duration_seconds += activity.duration_seconds;
                last_activity.input_metrics.keystrokes += activity.input_metrics.keystrokes;
                last_activity.input_metrics.mouse_clicks += activity.input_metrics.mouse_clicks;
                last_activity.input_metrics.mouse_distance_pixels += activity.input_metrics.mouse_distance_pixels;
                return;
            }
        }
        
        // Maintain size limit
        if self.activities.len() >= MAX_HISTORY_SIZE {
            self.activities.pop_front();
        }
        
        self.activities.push_back(activity);
    }
    
    pub fn get_recent(&self, limit: usize) -> Vec<&Activity> {
        self.activities
            .iter()
            .rev()
            .take(limit)
            .collect()
    }
    
    pub fn get_by_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&Activity> {
        self.activities
            .iter()
            .filter(|a| a.timestamp >= start && a.timestamp <= end)
            .collect()
    }
    
    pub fn get_by_app(&self, app_name: &str) -> Vec<&Activity> {
        self.activities
            .iter()
            .filter(|a| a.app_usage.app_name.to_lowercase().contains(&app_name.to_lowercase()))
            .collect()
    }
    
    pub fn get_by_category(&self, category: crate::models::AppCategory) -> Vec<&Activity> {
        self.activities
            .iter()
            .filter(|a| a.app_usage.category == category)
            .collect()
    }
    
    pub fn get_productive_time(&self, hours: i64) -> u32 {
        let since = Utc::now() - Duration::hours(hours);
        
        self.activities
            .iter()
            .filter(|a| a.timestamp >= since && a.app_usage.is_productive)
            .map(|a| a.duration_seconds as u32)
            .sum()
    }
    
    pub fn get_total_time(&self, hours: i64) -> u32 {
        let since = Utc::now() - Duration::hours(hours);
        
        self.activities
            .iter()
            .filter(|a| a.timestamp >= since)
            .map(|a| a.duration_seconds as u32)
            .sum()
    }
    
    pub fn get_app_usage_stats(&self, hours: i64) -> Vec<(String, u32)> {
        use std::collections::HashMap;
        
        let since = Utc::now() - Duration::hours(hours);
        let mut app_times: HashMap<String, u32> = HashMap::new();
        
        for activity in self.activities.iter().filter(|a| a.timestamp >= since) {
            *app_times.entry(activity.app_usage.app_name.clone()).or_insert(0) += activity.duration_seconds as u32;
        }
        
        let mut stats: Vec<(String, u32)> = app_times.into_iter().collect();
        stats.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by time descending
        
        stats
    }
    
    pub fn get_hourly_breakdown(&self, date: DateTime<Utc>) -> Vec<(u8, u32)> {
        use chrono::Timelike;
        
        let mut hourly_times = vec![0u32; 24];
        
        for activity in self.activities.iter() {
            if activity.timestamp.date_naive() == date.date_naive() {
                let hour = activity.timestamp.hour() as usize;
                if hour < 24 {
                    hourly_times[hour] += activity.duration_seconds as u32;
                }
            }
        }
        
        hourly_times
            .into_iter()
            .enumerate()
            .map(|(hour, time)| (hour as u8, time))
            .collect()
    }
    
    pub fn clear(&mut self) {
        self.activities.clear();
    }
    
    pub fn len(&self) -> usize {
        self.activities.len()
    }
}