import { useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { X, Plus } from 'lucide-react'

interface CreateGoalModalProps {
  onClose: () => void
  onCreate: (name: string, duration: number, apps: string[]) => void
}

export default function CreateGoalModal({ onClose, onCreate }: CreateGoalModalProps) {
  const [name, setName] = useState('')
  const [duration, setDuration] = useState(60)
  const [apps, setApps] = useState<string[]>([])
  const [newApp, setNewApp] = useState('')

  const addApp = () => {
    if (newApp.trim() && !apps.includes(newApp.trim())) {
      setApps([...apps, newApp.trim()])
      setNewApp('')
    }
  }

  const removeApp = (app: string) => {
    setApps(apps.filter(a => a !== app))
  }

  const handleCreate = () => {
    if (name.trim() && duration > 0 && apps.length > 0) {
      onCreate(name.trim(), duration, apps)
    }
  }

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
        onClick={onClose}
      >
        <motion.div
          initial={{ scale: 0.9, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          exit={{ scale: 0.9, opacity: 0 }}
          className="card max-w-md w-full mx-4"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="flex items-center justify-between mb-6">
            <h3 className="text-xl font-semibold">Create New Goal</h3>
            <button
              onClick={onClose}
              className="p-1 hover:bg-dark-bg rounded-lg transition-colors"
            >
              <X className="w-5 h-5" />
            </button>
          </div>

          <div className="space-y-4">
            <div>
              <label className="block text-sm text-gray-400 mb-2">Goal Name</label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g., Deep Work Session"
                className="input w-full"
              />
            </div>

            <div>
              <label className="block text-sm text-gray-400 mb-2">
                Duration (minutes)
              </label>
              <input
                type="number"
                value={duration}
                onChange={(e) => setDuration(parseInt(e.target.value) || 0)}
                min="1"
                className="input w-full"
              />
            </div>

            <div>
              <label className="block text-sm text-gray-400 mb-2">
                Allowed Applications
              </label>
              <div className="flex space-x-2 mb-2">
                <input
                  type="text"
                  value={newApp}
                  onChange={(e) => setNewApp(e.target.value)}
                  onKeyPress={(e) => e.key === 'Enter' && addApp()}
                  placeholder="e.g., VS Code"
                  className="input flex-1"
                />
                <button
                  onClick={addApp}
                  className="btn-secondary"
                >
                  <Plus className="w-4 h-4" />
                </button>
              </div>
              
              <div className="flex flex-wrap gap-2">
                {apps.map((app) => (
                  <span
                    key={app}
                    className="px-3 py-1 bg-dark-bg rounded-lg text-sm flex items-center space-x-2"
                  >
                    <span>{app}</span>
                    <button
                      onClick={() => removeApp(app)}
                      className="text-gray-400 hover:text-white"
                    >
                      <X className="w-3 h-3" />
                    </button>
                  </span>
                ))}
              </div>
            </div>
          </div>

          <div className="flex space-x-3 mt-6">
            <button
              onClick={onClose}
              className="btn-secondary flex-1"
            >
              Cancel
            </button>
            <button
              onClick={handleCreate}
              disabled={!name.trim() || duration <= 0 || apps.length === 0}
              className="btn-primary flex-1"
            >
              Create Goal
            </button>
          </div>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  )
}