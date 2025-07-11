import { useState, useEffect } from 'react'
import { motion } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { ExternalLink, Save, ChevronDown, ChevronUp } from 'lucide-react'

interface ResearchResultsProps {
  taskId: string
}

interface ResearchTask {
  id: string
  query: string
  status: string
  subtasks: any[]
  results: any[]
  conclusion: string
}

export default function ResearchResults({ taskId }: ResearchResultsProps) {
  const [task, setTask] = useState<ResearchTask | null>(null)
  const [expandedResults, setExpandedResults] = useState<Set<string>>(new Set())
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    loadTask()
  }, [taskId])

  const loadTask = async () => {
    try {
      const taskData = await invoke<ResearchTask>('get_research_status', { taskId })
      setTask(taskData)
    } catch (error) {
      console.error('Failed to load research task:', error)
    }
  }

  const saveResearch = async () => {
    if (!task) return
    
    setSaving(true)
    try {
      await invoke('save_research', {
        taskId: task.id,
        tags: task.query.split(' ').slice(0, 3),
        notes: ''
      })
      // Show success notification
    } catch (error) {
      console.error('Failed to save research:', error)
    } finally {
      setSaving(false)
    }
  }

  const toggleResult = (resultId: string) => {
    const newExpanded = new Set(expandedResults)
    if (newExpanded.has(resultId)) {
      newExpanded.delete(resultId)
    } else {
      newExpanded.add(resultId)
    }
    setExpandedResults(newExpanded)
  }

  if (!task) return null

  return (
    <div className="space-y-6">
      <div className="card">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h3 className="text-xl font-semibold">Research Results</h3>
            <p className="text-gray-400 mt-1">{task.query}</p>
          </div>
          <button
            onClick={saveResearch}
            disabled={saving}
            className="btn-secondary flex items-center space-x-2"
          >
            <Save className="w-4 h-4" />
            <span>{saving ? 'Saving...' : 'Save Research'}</span>
          </button>
        </div>

        {task.conclusion && (
          <div className="mb-6 p-4 bg-primary/10 rounded-lg">
            <h4 className="font-semibold mb-2">AI Conclusion</h4>
            <p className="text-gray-300 whitespace-pre-wrap">{task.conclusion}</p>
          </div>
        )}

        <div className="space-y-4">
          <h4 className="font-semibold">Sources ({task.results.length})</h4>
          {task.results.map((result, index) => (
            <motion.div
              key={result.id}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: index * 0.05 }}
              className="border border-dark-border rounded-lg overflow-hidden"
            >
              <div
                className="p-4 cursor-pointer hover:bg-dark-bg transition-colors"
                onClick={() => toggleResult(result.id)}
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1 mr-4">
                    <h5 className="font-medium">{result.title}</h5>
                    <a
                      href={result.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-sm text-primary hover:underline mt-1 inline-flex items-center"
                      onClick={(e) => e.stopPropagation()}
                    >
                      {new URL(result.url).hostname}
                      <ExternalLink className="w-3 h-3 ml-1" />
                    </a>
                  </div>
                  <div className="flex items-center space-x-2">
                    <span className="text-sm text-gray-400">
                      {(result.relevance_score * 100).toFixed(0)}% relevant
                    </span>
                    {expandedResults.has(result.id) ? (
                      <ChevronUp className="w-5 h-5 text-gray-400" />
                    ) : (
                      <ChevronDown className="w-5 h-5 text-gray-400" />
                    )}
                  </div>
                </div>
              </div>
              
              {expandedResults.has(result.id) && (
                <motion.div
                  initial={{ height: 0 }}
                  animate={{ height: 'auto' }}
                  exit={{ height: 0 }}
                  className="px-4 pb-4 border-t border-dark-border"
                >
                  <p className="text-gray-300 text-sm mt-4 whitespace-pre-wrap">
                    {result.content.slice(0, 500)}...
                  </p>
                </motion.div>
              )}
            </motion.div>
          ))}
        </div>
      </div>
    </div>
  )
}