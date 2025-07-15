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

export default function ActivityHistory() {
  const [activities, setActivities] = useState<Activity[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadHistory()
  }, [])

  const loadHistory = async () => {
    try {
      const history = await invoke<Activity[]>('get_activity_history', { limit: 20 })
      setActivities(history)
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
      ) : activities.length === 0 ? (
        <p className="text-gray-400 text-center py-8">No activity history yet.</p>
      ) : (
        <div className="space-y-3">
          {activities.map((activity, index) => (
            <motion.div
              key={activity.id}
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: index * 0.05 }}
              className="flex items-center space-x-4 p-3 rounded-lg hover:bg-dark-bg transition-colors"
            >
              <div className={`p-2 rounded-lg ${
                activity.app_usage.is_productive 
                  ? 'bg-success/20 text-success' 
                  : 'bg-gray-600/20 text-gray-400'
              }`}>
                <Monitor className="w-4 h-4" />
              </div>
              
              <div className="flex-1 min-w-0">
                <div className="flex items-center space-x-2">
                  <p className="font-medium">{activity.app_usage.app_name}</p>
                  {activity.goal_id && (
                    <Target className="w-3 h-3 text-success" />
                  )}
                </div>
                <p className="text-sm text-gray-400 truncate" title={activity.app_usage.window_title}>
                  {activity.app_usage.window_title}
                </p>
              </div>
              
              <div className="text-right text-sm">
                <p className="text-gray-400">
                  {new Date(activity.timestamp).toLocaleTimeString()}
                </p>
                <p className="flex items-center space-x-1 text-gray-500">
                  <Clock className="w-3 h-3" />
                  <span>{formatDuration(activity.duration_seconds)}</span>
                </p>
              </div>
            </motion.div>
          ))}
        </div>
      )}
    </div>
  )
}