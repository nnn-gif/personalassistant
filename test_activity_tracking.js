const { invoke } = require('@tauri-apps/api/core');

async function testActivityTracking() {
  console.log('Testing Activity Tracking...\n');
  
  try {
    // 1. Check if tracking is active
    console.log('1. Getting tracking stats...');
    const stats = await invoke('get_tracking_stats');
    console.log('Tracking stats:', JSON.stringify(stats, null, 2));
    
    // 2. Get current activity
    console.log('\n2. Getting current activity...');
    const currentActivity = await invoke('get_current_activity');
    console.log('Current activity:', JSON.stringify(currentActivity, null, 2));
    
    // 3. Get today's stats
    console.log('\n3. Getting today stats (this might take a moment)...');
    const todayStats = await invoke('get_today_stats');
    console.log('Today stats:', JSON.stringify(todayStats, null, 2));
    
    // Calculate hours
    if (todayStats && todayStats.total_tracked_seconds) {
      const hours = todayStats.total_tracked_seconds / 3600;
      console.log(`\nTotal hours today: ${hours.toFixed(2)} hours`);
      console.log(`Total minutes today: ${(todayStats.total_tracked_seconds / 60).toFixed(0)} minutes`);
    }
    
    // 4. Get activity history
    console.log('\n4. Getting recent activities...');
    const history = await invoke('get_activity_history', { limit: 5 });
    console.log(`Found ${history.length} recent activities`);
    if (history.length > 0) {
      console.log('First activity:', JSON.stringify(history[0], null, 2));
    }
    
  } catch (error) {
    console.error('Error:', error);
  }
}

// Run the test
if (typeof window !== 'undefined' && window.__TAURI__) {
  testActivityTracking();
} else {
  console.error('This script must be run in a Tauri environment');
}