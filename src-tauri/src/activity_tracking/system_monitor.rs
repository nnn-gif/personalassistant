use crate::error::{AppError, Result};
use crate::models::SystemState;
use std::process::Command;

pub struct SystemMonitor;

impl SystemMonitor {
    pub fn new() -> Self {
        Self
    }

    pub fn get_system_state(&self) -> Result<SystemState> {
        let cpu_usage = self.get_cpu_usage()?;
        let memory_info = self.get_memory_info()?;
        let battery_info = self.get_battery_info()?;
        let idle_time = self.get_idle_time()?;

        Ok(SystemState {
            idle_time_seconds: idle_time,
            is_screen_locked: false, // TODO: Implement screen lock detection
            battery_percentage: battery_info.0,
            is_on_battery: battery_info.1,
            cpu_usage_percent: cpu_usage,
            memory_usage_mb: memory_info as u32,
        })
    }

    fn get_cpu_usage(&self) -> Result<f32> {
        // Get CPU usage using ps command
        let output = Command::new("ps")
            .args(&["-A", "-o", "%cpu"])
            .output()
            .map_err(|e| AppError::Platform(format!("Failed to get CPU usage: {}", e)))?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = output_str.lines().collect();

            let total: f32 = lines
                .iter()
                .skip(1) // Skip header
                .filter_map(|line| line.trim().parse::<f32>().ok())
                .sum();

            // Cap at 100% (can be higher with multiple cores)
            Ok(total.min(100.0))
        } else {
            Ok(0.0)
        }
    }

    fn get_memory_info(&self) -> Result<u64> {
        // Get memory usage using vm_stat
        let output = Command::new("vm_stat")
            .output()
            .map_err(|e| AppError::Platform(format!("Failed to get memory info: {}", e)))?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);

            // Parse page size and calculate used memory
            let mut page_size = 4096u64; // Default page size
            let mut pages_active = 0u64;
            let mut pages_wired = 0u64;

            for line in output_str.lines() {
                if line.contains("page size of") {
                    if let Some(size_str) = line.split_whitespace().nth(7) {
                        page_size = size_str.parse().unwrap_or(4096);
                    }
                } else if line.starts_with("Pages active:") {
                    if let Some(pages_str) = line.split(':').nth(1) {
                        pages_active = pages_str.trim().trim_end_matches('.').parse().unwrap_or(0);
                    }
                } else if line.starts_with("Pages wired down:") {
                    if let Some(pages_str) = line.split(':').nth(1) {
                        pages_wired = pages_str.trim().trim_end_matches('.').parse().unwrap_or(0);
                    }
                }
            }

            // Convert to MB
            let used_bytes = (pages_active + pages_wired) * page_size;
            Ok(used_bytes / (1024 * 1024))
        } else {
            Ok(0)
        }
    }

    fn get_battery_info(&self) -> Result<(Option<f32>, bool)> {
        // Get battery info using pmset
        let output = Command::new("pmset")
            .args(&["-g", "batt"])
            .output()
            .map_err(|e| AppError::Platform(format!("Failed to get battery info: {}", e)))?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);

            // Parse battery percentage and power source
            for line in output_str.lines() {
                if line.contains("InternalBattery") {
                    // Extract percentage
                    let percentage = line
                        .split(';')
                        .next()
                        .and_then(|part| part.split_whitespace().find(|s| s.ends_with('%')))
                        .and_then(|s| s.trim_end_matches('%').parse::<f32>().ok());

                    // Check if on battery power
                    let is_on_battery = !line.contains("AC attached");

                    return Ok((percentage, is_on_battery));
                }
            }
        }

        // No battery (desktop) or error
        Ok((None, false))
    }

    fn get_idle_time(&self) -> Result<u32> {
        // Get idle time using ioreg
        let output = Command::new("ioreg")
            .args(&["-c", "IOHIDSystem", "-d", "4"])
            .output()
            .map_err(|e| AppError::Platform(format!("Failed to get idle time: {}", e)))?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);

            // Parse HIDIdleTime
            for line in output_str.lines() {
                if line.contains("HIDIdleTime") {
                    if let Some(value_str) = line.split('=').nth(1) {
                        if let Ok(nanoseconds) = value_str.trim().parse::<u64>() {
                            // Convert nanoseconds to seconds
                            return Ok((nanoseconds / 1_000_000_000) as u32);
                        }
                    }
                }
            }
        }

        Ok(0)
    }
}
