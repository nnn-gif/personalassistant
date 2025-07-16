import { motion } from 'framer-motion'
import { Monitor, Folder, Globe, Terminal, Target } from 'lucide-react'
import { Activity } from '../../types'

interface CurrentActivityProps {
  activity: Activity | null
}

export default function CurrentActivity({ activity }: CurrentActivityProps) {
  if (!activity) {
    return (
      <div className="card">
        <h3 className="text-xl font-semibold mb-4">Current Activity</h3>
        <p className="text-gray-400 text-center py-8">
          No activity detected. Start tracking to see your current activity.
        </p>
      </div>
    )
  }

  const getCategoryIcon = () => {
    switch (activity.app_usage?.category) {
      case 'Development':
        return <Terminal className="w-5 h-5" />
      case 'Communication':
        return <Monitor className="w-5 h-5" />
      case 'Browser':
        return <Globe className="w-5 h-5" />
      default:
        return <Monitor className="w-5 h-5" />
    }
  }

  const getCategoryColor = () => {
    if (activity.app_usage?.is_productive) return 'text-success'
    return 'text-gray-400'
  }

  return (
    <div className="card">
      <h3 className="text-xl font-semibold mb-4">Current Activity</h3>
      
      <div className="space-y-4">
        <div className="flex items-start space-x-4">
          <div className={`p-3 rounded-lg bg-dark-bg ${getCategoryColor()}`}>
            {getCategoryIcon()}
          </div>
          <div className="flex-1 min-w-0">
            <h4 className="font-semibold text-lg">{activity.app_usage?.app_name}</h4>
            <p className="text-gray-400 text-sm mt-1 truncate" title={activity.app_usage?.window_title}>
              {activity.app_usage?.window_title}
            </p>
            
            {activity.project_context && (
              <motion.div
                initial={{ opacity: 0, y: -10 }}
                animate={{ opacity: 1, y: 0 }}
                className="mt-3 flex items-center space-x-2"
              >
                <Folder className="w-4 h-4 text-primary" />
                <span className="text-sm">
                  {activity.project_context.project_name}
                  {activity.project_context.git_branch && (
                    <span className="text-gray-400 ml-2">
                      ({activity.project_context.git_branch})
                    </span>
                  )}
                </span>
              </motion.div>
            )}
            
            {activity.goal_id && (
              <motion.div
                initial={{ opacity: 0, y: -10 }}
                animate={{ opacity: 1, y: 0 }}
                className="mt-3 flex items-center space-x-2 text-success"
              >
                <Target className="w-4 h-4" />
                <span className="text-sm">Activity tracked for goal</span>
              </motion.div>
            )}
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4 pt-4 border-t border-dark-border">
          <div>
            <p className="text-gray-400 text-sm">Category</p>
            <p className="font-medium mt-1">{activity.app_usage?.category}</p>
          </div>
          <div>
            <p className="text-gray-400 text-sm">Productivity</p>
            <p className={`font-medium mt-1 ${getCategoryColor()}`}>
              {activity.app_usage?.is_productive ? 'Productive' : 'Unproductive'}
            </p>
          </div>
          <div>
            <p className="text-gray-400 text-sm">Duration</p>
            <p className="font-medium mt-1">{activity.duration_seconds}s</p>
          </div>
          <div>
            <p className="text-gray-400 text-sm">Time</p>
            <p className="font-medium mt-1">
              {new Date(activity.timestamp).toLocaleTimeString()}
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}