import { motion } from 'framer-motion'
import { Loader2 } from 'lucide-react'

export interface ResearchProgress {
  task_id: string
  status: string
  current_subtask?: string
  completed_subtasks: number
  total_subtasks: number
  percentage: number
  current_operation?: string
}

interface ResearchProgressProps {
  progress: ResearchProgress
}

export default function ResearchProgressComponent({ progress }: ResearchProgressProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: -20 }}
      animate={{ opacity: 1, y: 0 }}
      className="mx-4 mb-4 p-4 bg-blue-500/10 border border-blue-500/30 rounded-lg"
    >
      <div className="flex items-center justify-between mb-2">
        <h4 className="text-sm font-medium text-blue-400 flex items-center">
          <Loader2 className="w-4 h-4 mr-2 animate-spin" />
          Research in Progress
        </h4>
        <span className="text-xs text-gray-400">
          {progress.completed_subtasks}/{progress.total_subtasks} tasks
        </span>
      </div>
      
      <div className="w-full bg-dark-bg rounded-full h-2 mb-2">
        <motion.div
          className="bg-blue-500 h-2 rounded-full"
          initial={{ width: 0 }}
          animate={{ width: `${progress.percentage}%` }}
          transition={{ duration: 0.3 }}
        />
      </div>
      
      {progress.current_subtask && (
        <p className="text-xs text-gray-400">
          {progress.current_subtask}
        </p>
      )}
      {progress.current_operation && (
        <p className="text-xs text-gray-500 mt-1">
          {progress.current_operation}
        </p>
      )}
    </motion.div>
  )
}