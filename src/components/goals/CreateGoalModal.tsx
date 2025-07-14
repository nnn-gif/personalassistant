import { useState, useEffect } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { X, Plus, ChevronDown } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'

interface CreateGoalModalProps {
  onClose: () => void
  onCreate: (name: string, duration: number, apps: string[]) => void
}

interface Activity {
  app_usage: {
    app_name: string
  }
}

export default function CreateGoalModal({ onClose, onCreate }: CreateGoalModalProps) {
  const [name, setName] = useState('')
  const [apps, setApps] = useState<string[]>([])
  const [newApp, setNewApp] = useState('')
  const [existingApps, setExistingApps] = useState<string[]>([])
  const [showDropdown, setShowDropdown] = useState(false)

  useEffect(() => {
    loadExistingApps()
  }, [])

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement
      if (!target.closest('.app-dropdown')) {
        setShowDropdown(false)
      }
    }

    document.addEventListener('click', handleClickOutside)
    return () => document.removeEventListener('click', handleClickOutside)
  }, [])

  const loadExistingApps = async () => {
    try {
      const activities = await invoke<Activity[]>('get_activity_history', { limit: 100 })
      const uniqueApps = [...new Set(activities.map(a => a.app_usage.app_name))]
      setExistingApps(uniqueApps.sort())
    } catch (error) {
      console.error('Failed to load apps:', error)
    }
  }

  const addApp = () => {
    if (newApp.trim() && !apps.includes(newApp.trim())) {
      setApps([...apps, newApp.trim()])
      setNewApp('')
    }
  }

  const addExistingApp = (app: string) => {
    if (!apps.includes(app)) {
      setApps([...apps, app])
    }
    setShowDropdown(false)
    setNewApp('')
  }

  const removeApp = (app: string) => {
    setApps(apps.filter(a => a !== app))
  }

  const handleCreate = () => {
    if (name.trim() && apps.length > 0) {
      // Pass 0 for duration since we're removing it
      onCreate(name.trim(), 0, apps)
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
                Allowed Applications
              </label>
              <div className="relative app-dropdown">
                <div className="flex space-x-2 mb-2">
                  <div className="relative flex-1">
                    <input
                      type="text"
                      value={newApp}
                      onChange={(e) => setNewApp(e.target.value)}
                      onKeyPress={(e) => e.key === 'Enter' && addApp()}
                      onFocus={() => setShowDropdown(true)}
                      placeholder="Type or select an app"
                      className="input w-full pr-8"
                    />
                    <button
                      type="button"
                      onClick={() => setShowDropdown(!showDropdown)}
                      className="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-white"
                    >
                      <ChevronDown className={`w-4 h-4 transition-transform ${showDropdown ? 'rotate-180' : ''}`} />
                    </button>
                  </div>
                  <button
                    onClick={addApp}
                    className="btn-secondary"
                  >
                    <Plus className="w-4 h-4" />
                  </button>
                </div>
                
                {showDropdown && existingApps.length > 0 && (
                  <div className="absolute z-10 w-full bg-dark-surface border border-dark-border rounded-lg max-h-48 overflow-y-auto">
                    {existingApps
                      .filter(app => app.toLowerCase().includes(newApp.toLowerCase()))
                      .filter(app => !apps.includes(app))
                      .map(app => (
                        <button
                          key={app}
                          onClick={() => addExistingApp(app)}
                          className="w-full text-left px-4 py-2 hover:bg-dark-bg transition-colors"
                        >
                          {app}
                        </button>
                      ))}
                  </div>
                )}
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
              disabled={!name.trim() || apps.length === 0}
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