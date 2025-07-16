import { useState, useEffect } from 'react'
import { motion } from 'framer-motion'
import { Plus, Play, Pause, Edit } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import CreateGoalModal from './goals/CreateGoalModal'
import { formatTime } from '../lib/timeUtils'
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

const MASTER_GOAL_ID = "00000000-0000-0000-0000-000000000001"

export default function GoalsManager() {
  const [goals, setGoals] = useState<Goal[]>([])
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [editingGoal, setEditingGoal] = useState<Goal | null>(null)
  const [loading, setLoading] = useState(false)
  const [refreshKey, setRefreshKey] = useState(0)

  const isMasterGoal = (goal: Goal) => goal.id === MASTER_GOAL_ID

  useEffect(() => {
    loadGoals()
  }, [])

  const loadGoals = async () => {
    try {
      console.log('Loading goals...')
      setLoading(true)
      const goalsData = await invoke<Goal[]>('get_goals')
      console.log('Goals loaded:', goalsData)
      setGoals([...goalsData]) // Force array recreation
      setRefreshKey(prev => prev + 1) // Force re-render
    } catch (error) {
      console.error('Failed to load goals:', error)
    } finally {
      setLoading(false)
    }
  }

  const toggleGoal = async (goal: Goal) => {
    if (loading) return
    
    try {
      // Don't allow deactivating the Master Goal
      if (goal.is_active && isMasterGoal(goal)) {
        console.log('Cannot deactivate Master Goal')
        return
      }
      
      console.log('Toggling goal:', goal.name, 'is_active:', goal.is_active)
      
      // Optimistic update for immediate UI feedback
      const newGoals = goals.map(g => {
        if (g.id === goal.id) {
          return { ...g, is_active: !g.is_active }
        }
        // If activating this goal, deactivate all others except Master Goal
        if (!goal.is_active && g.is_active && !isMasterGoal(g)) {
          return { ...g, is_active: false }
        }
        return g
      })
      setGoals(newGoals)
      
      if (goal.is_active) {
        console.log('Deactivating goal:', goal.id)
        await invoke('deactivate_goal', { goalId: goal.id })
      } else {
        console.log('Activating goal:', goal.id)
        await invoke('activate_goal', { goalId: goal.id })
      }
      
      console.log('Goal toggle completed, reloading goals...')
      // Small delay to ensure backend has processed the change
      await new Promise(resolve => setTimeout(resolve, 100))
      await loadGoals()
    } catch (error) {
      console.error('Failed to toggle goal:', error)
      // Revert optimistic update on error
      await loadGoals()
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

  const updateGoal = async (name: string, duration: number, apps: string[]) => {
    if (!editingGoal) return
    
    try {
      await invoke('update_goal', {
        goalId: editingGoal.id,
        name,
        targetDurationMinutes: duration,
        allowedApps: apps
      })
      setEditingGoal(null)
      await loadGoals()
    } catch (error) {
      console.error('Failed to update goal:', error)
    }
  }

  const handleEdit = (goal: Goal) => {
    // Don't allow editing the Master Goal
    if (isMasterGoal(goal)) {
      return
    }
    setEditingGoal(goal)
  }

  return (
    <div className="space-y-6" key={refreshKey}>
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
            className={`card ${isMasterGoal(goal) ? 'border-2 border-primary/30 bg-primary/5' : ''}`}
          >
            <div className="flex items-start justify-between mb-4">
              <div>
                <div className="flex items-center space-x-2">
                  <h3 className="text-xl font-semibold">{goal.name}</h3>
                  {isMasterGoal(goal) && (
                    <span className="text-xs bg-primary/20 text-primary px-2 py-1 rounded">
                      DEFAULT
                    </span>
                  )}
                  {goal.is_active && (
                    <span className="text-xs bg-success/20 text-success px-2 py-1 rounded">
                      ACTIVE
                    </span>
                  )}
                </div>
                <p className="text-gray-400 text-sm mt-1">
                  Active for {formatTime(goal.current_duration_minutes)}
                </p>
              </div>
              <div className="flex space-x-2">
                {!isMasterGoal(goal) && (
                  <button
                    onClick={() => handleEdit(goal)}
                    className="p-2 rounded-lg bg-dark-bg text-gray-400 hover:text-white transition-colors"
                  >
                    <Edit className="w-5 h-5" />
                  </button>
                )}
                <button
                  onClick={() => toggleGoal(goal)}
                  className={`p-2 rounded-lg transition-colors ${
                    goal.is_active 
                      ? (isMasterGoal(goal) ? 'bg-primary/20 text-primary cursor-default' : 'bg-success/20 text-success')
                      : 'bg-dark-bg text-gray-400 hover:text-white'
                  } ${loading ? 'opacity-50 cursor-not-allowed' : ''}`}
                  disabled={(goal.is_active && isMasterGoal(goal)) || loading}
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
            </div>

            {/* Remove progress bar since we don't have duration-based progress anymore */}

            <div className="mt-4">
              <p className="text-sm text-gray-400 mb-2">Allowed Apps:</p>
              <div className="flex flex-wrap gap-2">
                {isMasterGoal(goal) ? (
                  <span className="px-2 py-1 bg-primary/20 text-primary rounded text-xs">
                    All applications
                  </span>
                ) : (
                  goal.allowed_apps.map((app) => (
                    <span
                      key={app}
                      className="px-2 py-1 bg-dark-bg rounded text-xs text-gray-300"
                    >
                      {app}
                    </span>
                  ))
                )}
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

      {editingGoal && (
        <CreateGoalModal
          onClose={() => setEditingGoal(null)}
          onCreate={updateGoal}
          editingGoal={editingGoal}
        />
      )}
    </div>
  )
}