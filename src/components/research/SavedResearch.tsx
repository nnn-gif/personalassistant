import { useState, useEffect } from 'react'
import { motion } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { Search, Tag, Calendar, ChevronRight } from 'lucide-react'

interface SavedResearchTask {
  id: string
  task: {
    query: string
    results: any[]
    conclusion: string
  }
  tags: string[]
  notes?: string
  saved_at: string
}

export default function SavedResearch() {
  const [savedTasks, setSavedTasks] = useState<SavedResearchTask[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedTask, setSelectedTask] = useState<SavedResearchTask | null>(null)

  useEffect(() => {
    loadSavedResearch()
  }, [searchQuery])

  const loadSavedResearch = async () => {
    try {
      const tasks = await invoke<SavedResearchTask[]>('get_saved_research', {
        searchQuery: searchQuery || undefined
      })
      setSavedTasks(tasks)
    } catch (error) {
      console.error('Failed to load saved research:', error)
    }
  }

  return (
    <div className="space-y-6">
      <div className="card">
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search saved research..."
          className="input w-full"
        />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {savedTasks.map((task, index) => (
          <motion.div
            key={task.id}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: index * 0.05 }}
            className="card cursor-pointer hover:bg-dark-bg transition-colors"
            onClick={() => setSelectedTask(task)}
          >
            <h4 className="font-semibold mb-2">{task.task.query}</h4>
            
            <div className="space-y-2 text-sm">
              <div className="flex items-center space-x-2 text-gray-400">
                <Calendar className="w-4 h-4" />
                <span>{new Date(task.saved_at).toLocaleDateString()}</span>
              </div>
              
              {task.tags.length > 0 && (
                <div className="flex items-center space-x-2">
                  <Tag className="w-4 h-4 text-gray-400" />
                  <div className="flex flex-wrap gap-1">
                    {task.tags.map((tag) => (
                      <span
                        key={tag}
                        className="px-2 py-1 bg-dark-bg rounded text-xs"
                      >
                        {tag}
                      </span>
                    ))}
                  </div>
                </div>
              )}
              
              <div className="flex items-center justify-between pt-2">
                <span className="text-gray-400">
                  {task.task.results.length} sources
                </span>
                <ChevronRight className="w-4 h-4 text-gray-400" />
              </div>
            </div>
          </motion.div>
        ))}
      </div>

      {savedTasks.length === 0 && (
        <div className="card text-center py-12">
          <Search className="w-12 h-12 text-gray-600 mx-auto mb-4" />
          <p className="text-gray-400">No saved research found</p>
        </div>
      )}

      {selectedTask && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
          onClick={() => setSelectedTask(null)}
        >
          <motion.div
            initial={{ scale: 0.9 }}
            animate={{ scale: 1 }}
            className="card max-w-4xl w-full max-h-[80vh] overflow-y-auto"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-xl font-semibold mb-4">{selectedTask.task.query}</h3>
            
            {selectedTask.task.conclusion && (
              <div className="mb-6 p-4 bg-primary/10 rounded-lg">
                <h4 className="font-semibold mb-2">Conclusion</h4>
                <p className="text-gray-300 whitespace-pre-wrap">
                  {selectedTask.task.conclusion}
                </p>
              </div>
            )}
            
            <button
              onClick={() => setSelectedTask(null)}
              className="btn-secondary mt-6"
            >
              Close
            </button>
          </motion.div>
        </motion.div>
      )}
    </div>
  )
}