import { useState, useEffect } from 'react'
import { motion } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { Clock, Monitor, Target } from 'lucide-react'

interface Activity {
  id: string
  timestamp: string
  duration_seconds: number
  app_usage: {
    app_name: string
    window_title: string
    category: string
    is_productive: boolean
  }
  goal_id?: string
}

interface AggregatedActivity {
  app_name: string
  total_duration_seconds: number
  window_titles: string[]
  is_productive: boolean
  has_goal: boolean
  last_timestamp: string
}

export default function ActivityHistory() {
  const [aggregatedActivities, setAggregatedActivities] = useState<AggregatedActivity[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadHistory()
  }, [])

  const loadHistory = async () => {
    try {
      const history = await invoke<Activity[]>('get_activity_history', { limit: 100 })
      
      // Aggregate activities by app
      const appMap = new Map<string, AggregatedActivity>()
      
      history.forEach(activity => {
        const appName = activity.app_usage.app_name
        const existing = appMap.get(appName)
        
        if (existing) {
          existing.total_duration_seconds += activity.duration_seconds
          if (!existing.window_titles.includes(activity.app_usage.window_title)) {
            existing.window_titles.push(activity.app_usage.window_title)
          }
          existing.has_goal = existing.has_goal || !!activity.goal_id
          // Update to latest timestamp
          if (new Date(activity.timestamp) > new Date(existing.last_timestamp)) {
            existing.last_timestamp = activity.timestamp
          }
        } else {
          appMap.set(appName, {
            app_name: appName,
            total_duration_seconds: activity.duration_seconds,
            window_titles: [activity.app_usage.window_title],
            is_productive: activity.app_usage.is_productive,
            has_goal: !!activity.goal_id,
            last_timestamp: activity.timestamp
          })
        }
      })
      
      // Sort by total duration (descending)
      const aggregated = Array.from(appMap.values())
        .sort((a, b) => b.total_duration_seconds - a.total_duration_seconds)
      
      setAggregatedActivities(aggregated)
    } catch (error) {
      console.error('Failed to load activity history:', error)
    } finally {
      setLoading(false)
    }
  }

  const formatDuration = (seconds: number) => {
    if (seconds < 60) return `${seconds}s`
    const minutes = Math.floor(seconds / 60)
    return `${minutes}m ${seconds % 60}s`
  }

  return (
    <div className="card">
      <h3 className="text-xl font-semibold mb-4">Activity History</h3>
      
      {loading ? (
        <p className="text-gray-400 text-center py-8">Loading history...</p>
      ) : aggregatedActivities.length === 0 ? (
        <p className="text-gray-400 text-center py-8">No activity history yet.</p>
      ) : (
        <div className="space-y-3">
          {aggregatedActivities.map((activity, index) => (
            <motion.div
              key={activity.app_name}
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: index * 0.05 }}
              className="flex items-center space-x-4 p-3 rounded-lg hover:bg-dark-bg transition-colors"
            >
              <div className={`p-2 rounded-lg ${
                activity.is_productive 
                  ? 'bg-success/20 text-success' 
                  : 'bg-gray-600/20 text-gray-400'
              }`}>
                <Monitor className="w-4 h-4" />
              </div>
              
              <div className="flex-1 min-w-0">
                <div className="flex items-center space-x-2">
                  <p className="font-medium">{activity.app_name}</p>
                  {activity.has_goal && (
                    <Target className="w-3 h-3 text-success" />
                  )}
                </div>
                <p className="text-sm text-gray-400 truncate" title={activity.window_titles.join(', ')}>
                  {activity.window_titles.length > 1 
                    ? `${activity.window_titles[0]} (+${activity.window_titles.length - 1} more)`
                    : activity.window_titles[0]
                  }
                </p>
              </div>
              
              <div className="text-right text-sm">
                <p className="text-gray-400">
                  Last: {new Date(activity.last_timestamp).toLocaleTimeString()}
                </p>
                <p className="flex items-center space-x-1 text-gray-500">
                  <Clock className="w-3 h-3" />
                  <span className="font-medium">{formatDuration(activity.total_duration_seconds)}</span>
                </p>
              </div>
            </motion.div>
          ))}
        </div>
      )}
    </div>
  )
}