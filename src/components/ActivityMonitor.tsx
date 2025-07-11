import { useState, useEffect } from 'react'
import { motion } from 'framer-motion'
import { Play, Pause, Monitor, Clock, MousePointer } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import CurrentActivity from './activity/CurrentActivity'
import ActivityHistory from './activity/ActivityHistory'

export default function ActivityMonitor() {
  const [isTracking, setIsTracking] = useState(false)
  const [currentActivity, setCurrentActivity] = useState<any>(null)

  useEffect(() => {
    loadCurrentActivity()
    const interval = setInterval(loadCurrentActivity, 5000) // Update every 5 seconds
    return () => clearInterval(interval)
  }, [])

  const loadCurrentActivity = async () => {
    try {
      const activity = await invoke('get_current_activity')
      setCurrentActivity(activity)
    } catch (error) {
      console.error('Failed to load current activity:', error)
    }
  }

  const toggleTracking = async () => {
    try {
      if (isTracking) {
        await invoke('stop_tracking')
      } else {
        await invoke('start_tracking')
      }
      setIsTracking(!isTracking)
    } catch (error) {
      console.error('Failed to toggle tracking:', error)
    }
  }

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold">Activity Monitor</h2>
          <p className="text-gray-400 mt-2">Track your digital activity in real-time</p>
        </div>
        <button
          onClick={toggleTracking}
          className={`btn-primary flex items-center space-x-2 ${
            isTracking ? 'bg-danger hover:bg-red-600' : ''
          }`}
        >
          {isTracking ? (
            <Pause className="w-5 h-5" />
          ) : (
            <Play className="w-5 h-5" />
          )}
          <span>{isTracking ? 'Stop Tracking' : 'Start Tracking'}</span>
        </button>
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-6">
          <CurrentActivity activity={currentActivity} />
          <ActivityHistory />
        </div>

        <div className="space-y-6">
          <div className="card">
            <h3 className="text-lg font-semibold mb-4">Tracking Status</h3>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-gray-400">Status</span>
                <span className={`font-medium ${
                  isTracking ? 'text-success' : 'text-gray-500'
                }`}>
                  {isTracking ? 'Active' : 'Inactive'}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-gray-400">Privacy Mode</span>
                <span className="font-medium">On-Device Only</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-gray-400">Data Retention</span>
                <span className="font-medium">90 Days</span>
              </div>
            </div>
          </div>

          {currentActivity && (
            <div className="card">
              <h3 className="text-lg font-semibold mb-4">System Metrics</h3>
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-2">
                    <Monitor className="w-4 h-4 text-gray-400" />
                    <span className="text-gray-400">CPU Usage</span>
                  </div>
                  <span className="font-medium">
                    {currentActivity.system_state?.cpu_usage_percent.toFixed(1)}%
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-2">
                    <Clock className="w-4 h-4 text-gray-400" />
                    <span className="text-gray-400">Idle Time</span>
                  </div>
                  <span className="font-medium">
                    {currentActivity.system_state?.idle_time_seconds}s
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-2">
                    <MousePointer className="w-4 h-4 text-gray-400" />
                    <span className="text-gray-400">Input Activity</span>
                  </div>
                  <span className="font-medium">
                    {currentActivity.input_metrics?.keystrokes || 0} keys
                  </span>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}