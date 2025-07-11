use crate::error::Result;
use crate::models::InputMetrics;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

pub struct InputMonitor {
    last_activity: Arc<Mutex<Instant>>,
    keystrokes: Arc<Mutex<u32>>,
    mouse_clicks: Arc<Mutex<u32>>,
    mouse_distance: Arc<Mutex<f32>>,
    last_mouse_pos: Arc<Mutex<Option<(f32, f32)>>>,
}

impl InputMonitor {
    pub fn new() -> Self {
        Self {
            last_activity: Arc::new(Mutex::new(Instant::now())),
            keystrokes: Arc::new(Mutex::new(0)),
            mouse_clicks: Arc::new(Mutex::new(0)),
            mouse_distance: Arc::new(Mutex::new(0.0)),
            last_mouse_pos: Arc::new(Mutex::new(None)),
        }
    }
    
    pub fn get_metrics_and_reset(&self) -> Result<InputMetrics> {
        let keystrokes = {
            let mut k = self.keystrokes.lock().unwrap();
            let val = *k;
            *k = 0;
            val
        };
        
        let mouse_clicks = {
            let mut c = self.mouse_clicks.lock().unwrap();
            let val = *c;
            *c = 0;
            val
        };
        
        let mouse_distance_pixels = {
            let mut d = self.mouse_distance.lock().unwrap();
            let val = *d;
            *d = 0.0;
            val
        };
        
        let active_typing_seconds = {
            let last = self.last_activity.lock().unwrap();
            let elapsed = last.elapsed();
            if elapsed < Duration::from_secs(30) {
                elapsed.as_secs() as u32
            } else {
                0
            }
        };
        
        Ok(InputMetrics {
            keystrokes,
            mouse_clicks,
            mouse_distance_pixels: mouse_distance_pixels as f64,
            active_typing_seconds,
        })
    }
    
    // These would be called by the event monitoring system
    pub fn record_keystroke(&self) {
        *self.keystrokes.lock().unwrap() += 1;
        *self.last_activity.lock().unwrap() = Instant::now();
    }
    
    pub fn record_mouse_click(&self) {
        *self.mouse_clicks.lock().unwrap() += 1;
        *self.last_activity.lock().unwrap() = Instant::now();
    }
    
    pub fn record_mouse_move(&self, x: f32, y: f32) {
        let mut last_pos = self.last_mouse_pos.lock().unwrap();
        
        if let Some((last_x, last_y)) = *last_pos {
            let dx = x - last_x;
            let dy = y - last_y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            *self.mouse_distance.lock().unwrap() += distance;
        }
        
        *last_pos = Some((x, y));
        *self.last_activity.lock().unwrap() = Instant::now();
    }
}