import { useState, useEffect } from 'react'
import { motion } from 'framer-motion'
import { Plus, Play, Pause } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import CreateGoalModal from './goals/CreateGoalModal'
// import GoalProgress from './goals/GoalProgress'

interface Goal {
  id: string
  name: string
  target_duration_minutes: number
  allowed_apps: string[]
  current_duration_minutes: number
  is_active: boolean
  created_at: string
  updated_at: string
}

export default function GoalsManager() {
  const [goals, setGoals] = useState<Goal[]>([])
  const [showCreateModal, setShowCreateModal] = useState(false)

  useEffect(() => {
    loadGoals()
  }, [])

  const loadGoals = async () => {
    try {
      const goalsData = await invoke<Goal[]>('get_goals')
      setGoals(goalsData)
    } catch (error) {
      console.error('Failed to load goals:', error)
    }
  }

  const toggleGoal = async (goal: Goal) => {
    try {
      if (goal.is_active) {
        await invoke('deactivate_goal', { goalId: goal.id })
      } else {
        await invoke('activate_goal', { goalId: goal.id })
      }
      await loadGoals()
    } catch (error) {
      console.error('Failed to toggle goal:', error)
    }
  }

  const createGoal = async (name: string, duration: number, apps: string[]) => {
    try {
      await invoke('create_goal', {
        name,
        targetDurationMinutes: duration,
        allowedApps: apps
      })
      setShowCreateModal(false)
      await loadGoals()
    } catch (error) {
      console.error('Failed to create goal:', error)
    }
  }

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold">Goals</h2>
          <p className="text-gray-400 mt-2">Track your productivity goals</p>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className="btn-primary flex items-center space-x-2"
        >
          <Plus className="w-5 h-5" />
          <span>New Goal</span>
        </button>
      </header>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {goals.map((goal, index) => (
          <motion.div
            key={goal.id}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: index * 0.1 }}
            className="card"
          >
            <div className="flex items-start justify-between mb-4">
              <div>
                <div className="flex items-center space-x-2">
                  <h3 className="text-xl font-semibold">{goal.name}</h3>
                  {goal.is_active && (
                    <span className="text-xs bg-success/20 text-success px-2 py-1 rounded">
                      ACTIVE
                    </span>
                  )}
                </div>
                <p className="text-gray-400 text-sm mt-1">
                  Active for {goal.current_duration_minutes} minutes
                </p>
              </div>
              <button
                onClick={() => toggleGoal(goal)}
                className={`p-2 rounded-lg transition-colors ${
                  goal.is_active 
                    ? 'bg-success/20 text-success' 
                    : 'bg-dark-bg text-gray-400 hover:text-white'
                }`}
              >
                {goal.is_active ? (
                  <>
                    <Pause className="w-5 h-5" />
                    <span className="sr-only">Active</span>
                  </>
                ) : (
                  <Play className="w-5 h-5" />
                )}
              </button>
            </div>

            {/* Remove progress bar since we don't have duration-based progress anymore */}

            <div className="mt-4">
              <p className="text-sm text-gray-400 mb-2">Allowed Apps:</p>
              <div className="flex flex-wrap gap-2">
                {goal.allowed_apps.map((app) => (
                  <span
                    key={app}
                    className="px-2 py-1 bg-dark-bg rounded text-xs text-gray-300"
                  >
                    {app}
                  </span>
                ))}
              </div>
            </div>
          </motion.div>
        ))}
      </div>

      {goals.length === 0 && (
        <div className="card text-center py-12">
          <p className="text-gray-400 mb-4">No goals created yet</p>
          <button
            onClick={() => setShowCreateModal(true)}
            className="btn-primary mx-auto"
          >
            Create Your First Goal
          </button>
        </div>
      )}

      {showCreateModal && (
        <CreateGoalModal
          onClose={() => setShowCreateModal(false)}
          onCreate={createGoal}
        />
      )}
    </div>
  )
}