import { motion } from 'framer-motion'

interface GoalProgressProps {
  progress: number
}

export default function GoalProgress({ progress }: GoalProgressProps) {
  const getProgressColor = () => {
    if (progress >= 80) return 'bg-success'
    if (progress >= 50) return 'bg-warning'
    return 'bg-primary'
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between text-sm">
        <span className="text-gray-400">Progress</span>
        <span className="font-medium">{progress.toFixed(0)}%</span>
      </div>
      <div className="w-full bg-dark-bg rounded-full h-3 overflow-hidden">
        <motion.div
          className={`h-3 rounded-full ${getProgressColor()}`}
          initial={{ width: 0 }}
          animate={{ width: `${progress}%` }}
          transition={{ duration: 0.5, ease: "easeOut" }}
        />
      </div>
    </div>
  )
}