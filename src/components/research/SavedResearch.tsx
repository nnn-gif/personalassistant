import { useState, useEffect } from 'react'
import { motion } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { Search, Tag, Calendar, ExternalLink, Trash2, ChevronDown, ChevronUp } from 'lucide-react'

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
  const [loading, setLoading] = useState(true)
  const [expandedTasks, setExpandedTasks] = useState<Set<string>>(new Set())
  const [deletingId, setDeletingId] = useState<string | null>(null)

  useEffect(() => {
    loadSavedResearch()
  }, [])

  useEffect(() => {
    const timer = setTimeout(() => {
      loadSavedResearch(searchQuery)
    }, 300)
    return () => clearTimeout(timer)
  }, [searchQuery])

  const loadSavedResearch = async (query?: string) => {
    try {
      const tasks = await invoke<SavedResearchTask[]>('get_saved_research', { 
        searchQuery: query || undefined 
      })
      setSavedTasks(tasks)
    } catch (error) {
      console.error('Failed to load saved research:', error)
    } finally {
      setLoading(false)
    }
  }
  
  const deleteResearch = async (id: string) => {
    if (!confirm('Are you sure you want to delete this research?')) return
    
    setDeletingId(id)
    try {
      await invoke('delete_saved_research', { id })
      await loadSavedResearch(searchQuery)
    } catch (error) {
      console.error('Failed to delete research:', error)
    } finally {
      setDeletingId(null)
    }
  }
  
  const toggleTask = (taskId: string) => {
    const newExpanded = new Set(expandedTasks)
    if (newExpanded.has(taskId)) {
      newExpanded.delete(taskId)
    } else {
      newExpanded.add(taskId)
    }
    setExpandedTasks(newExpanded)
  }

  if (loading) {
    return (
      <div className="card text-center py-12">
        <p className="text-gray-400">Loading saved research...</p>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="card">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-5 h-5 text-gray-400" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search saved research by query, tags, or notes..."
            className="input w-full pl-10"
          />
        </div>
      </div>

      <div className="space-y-4">
        {savedTasks.length === 0 ? (
          <div className="card text-center py-12">
            <Search className="w-12 h-12 text-gray-600 mx-auto mb-4" />
            <p className="text-gray-400">
              {searchQuery ? 'No research found matching your search' : 'No saved research yet'}
            </p>
          </div>
        ) : (
          savedTasks.map((task, index) => (
            <motion.div
              key={task.id}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: index * 0.05 }}
              className="card"
            >
              <div className="flex items-start justify-between mb-4">
                <div className="flex-1">
                  <div className="flex items-center justify-between">
                    <h3 className="text-lg font-semibold">{task.task.query}</h3>
                    <div className="flex items-center space-x-2">
                      <button
                        onClick={() => toggleTask(task.id)}
                        className="p-1 hover:bg-dark-bg rounded transition-colors"
                      >
                        {expandedTasks.has(task.id) ? (
                          <ChevronUp className="w-5 h-5 text-gray-400" />
                        ) : (
                          <ChevronDown className="w-5 h-5 text-gray-400" />
                        )}
                      </button>
                      <button
                        onClick={() => deleteResearch(task.id)}
                        disabled={deletingId === task.id}
                        className="p-1 hover:bg-danger/20 hover:text-danger rounded transition-colors"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                  <p className="text-sm text-gray-400 mt-1">
                    <Calendar className="w-3 h-3 inline mr-1" />
                    {new Date(task.saved_at).toLocaleDateString()}
                    {task.task.results.length > 0 && (
                      <span className="ml-3">
                        {task.task.results.length} sources
                      </span>
                    )}
                  </p>
                </div>
              </div>

              {task.notes && (
                <p className="text-sm text-gray-300 mb-4">{task.notes}</p>
              )}

              {expandedTasks.has(task.id) && (
                <motion.div
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: 'auto', opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  className="space-y-4 mb-4"
                >
                  {task.task.conclusion && (
                    <div className="p-3 bg-primary/10 rounded-lg">
                      <h4 className="font-semibold mb-2">Conclusion</h4>
                      <p className="text-sm text-gray-300 whitespace-pre-wrap">
                        {task.task.conclusion}
                      </p>
                    </div>
                  )}
                  
                  {task.task.results.length > 0 && (
                    <div className="space-y-2">
                      <h4 className="font-semibold">Sources</h4>
                      {task.task.results.map((result, resultIndex) => (
                        <div key={result.id || resultIndex} className="p-3 bg-dark-bg rounded-lg">
                          <h5 className="font-medium text-sm">{result.title}</h5>
                          <a
                            href={result.url}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-xs text-primary hover:underline mt-1 inline-flex items-center"
                          >
                            {new URL(result.url).hostname}
                            <ExternalLink className="w-3 h-3 ml-1" />
                          </a>
                          <p className="text-xs text-gray-400 mt-2">
                            {result.content.slice(0, 200)}...
                          </p>
                        </div>
                      ))}
                    </div>
                  )}
                </motion.div>
              )}

              <div className="flex flex-wrap gap-2">
                {task.tags.map((tag, tagIndex) => (
                  <span
                    key={tagIndex}
                    className="px-2 py-1 bg-dark-bg rounded-full text-xs flex items-center"
                  >
                    <Tag className="w-3 h-3 mr-1" />
                    {tag}
                  </span>
                ))}
              </div>
            </motion.div>
          ))
        )}
      </div>
    </div>
  )
}