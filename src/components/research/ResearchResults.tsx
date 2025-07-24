import { useState, useEffect } from 'react'
import { motion } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { ExternalLink, Save, ChevronDown, ChevronUp, Check, Layers } from 'lucide-react'

interface ResearchResultsProps {
  taskId: string
}

interface ResearchTask {
  id: string
  query: string
  status: string
  subtasks: ResearchSubtask[]
  results: ResearchResult[]
  conclusion: string
}

interface ResearchSubtask {
  id: string
  query: string
  status: string
  search_results: SearchResult[]
}

interface SearchResult {
  url: string
  title: string
  description: string
}

interface ResearchResult {
  id: string
  subtask_id: string
  url: string
  title: string
  content: string
  relevance_score: number
  scraped_at: string
}

export default function ResearchResults({ taskId }: ResearchResultsProps) {
  const [task, setTask] = useState<ResearchTask | null>(null)
  const [expandedResults, setExpandedResults] = useState<Set<string>>(new Set())
  const [saving, setSaving] = useState(false)
  const [saved, setSaved] = useState(false)
  const [groupBySubtask, setGroupBySubtask] = useState(false)

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
    if (!task || saved) return
    
    console.log('Saving research for task:', task.id)
    setSaving(true)
    try {
      const result = await invoke('save_research', {
        taskId: task.id,
        tags: task.query.split(' ').slice(0, 3),
        notes: ''
      })
      console.log('Research saved successfully:', result)
      setSaved(true)
      // Reset saved state after 3 seconds
      setTimeout(() => setSaved(false), 3000)
    } catch (error) {
      console.error('Failed to save research:', error)
      alert('Failed to save research: ' + error)
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

  const getStatusColor = (status: string): string => {
    switch (status) {
      case 'Completed':
        return 'bg-green-600 text-green-100'
      case 'Searching':
      case 'Scraping':
      case 'Analyzing':
        return 'bg-blue-600 text-blue-100'
      case 'SplittingTasks':
        return 'bg-yellow-600 text-yellow-100'
      case 'Pending':
        return 'bg-gray-600 text-gray-100'
      default:
        return 'bg-gray-600 text-gray-100'
    }
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
            disabled={saving || saved}
            className={`btn-secondary flex items-center space-x-2 transition-colors ${
              saved ? 'bg-success/20 text-success' : ''
            }`}
          >
            {saved ? (
              <Check className="w-4 h-4" />
            ) : (
              <Save className="w-4 h-4" />
            )}
            <span>{saving ? 'Saving...' : saved ? 'Saved!' : 'Save Research'}</span>
          </button>
        </div>

        {task.conclusion && (
          <div className="mb-6 p-4 bg-primary/10 rounded-lg">
            <h4 className="font-semibold mb-2">AI Conclusion</h4>
            <p className="text-gray-300 whitespace-pre-wrap">{task.conclusion}</p>
          </div>
        )}

        {task.subtasks && task.subtasks.length > 0 && (
          <div className="mb-6">
            <h4 className="font-semibold mb-3">Research Subtasks ({task.subtasks.length})</h4>
            <div className="space-y-3">
              {task.subtasks.map((subtask, index) => (
                <motion.div
                  key={subtask.id}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: index * 0.05 }}
                  className="bg-dark-bg rounded-lg p-4 border border-gray-700"
                >
                  <div className="flex items-center justify-between mb-2">
                    <h5 className="text-sm font-medium text-gray-200">{subtask.query}</h5>
                    <span className={`text-xs px-2 py-1 rounded-full ${getStatusColor(subtask.status)}`}>
                      {subtask.status}
                    </span>
                  </div>
                  {subtask.search_results && subtask.search_results.length > 0 && (
                    <div className="text-xs text-gray-400">
                      Found {subtask.search_results.length} search results
                    </div>
                  )}
                </motion.div>
              ))}
            </div>
          </div>
        )}

        <div className="space-y-4">
          <div className="flex items-center justify-between mb-2">
            <h4 className="font-semibold">Sources ({task.results.length})</h4>
            <button
              onClick={() => setGroupBySubtask(!groupBySubtask)}
              className={`btn-secondary text-sm flex items-center space-x-2 ${
                groupBySubtask ? 'bg-primary/20 text-primary' : ''
              }`}
            >
              <Layers className="w-4 h-4" />
              <span>{groupBySubtask ? 'Grouped' : 'Group by Subtask'}</span>
            </button>
          </div>
          
          {groupBySubtask ? (
            // Grouped by subtask view
            <div className="space-y-6">
              {task.subtasks.map((subtask) => {
                const subtaskResults = task.results.filter(r => r.subtask_id === subtask.id)
                if (subtaskResults.length === 0) return null
                
                return (
                  <div key={subtask.id} className="space-y-3">
                    <h5 className="text-sm font-medium text-gray-300 border-b border-gray-700 pb-2">
                      {subtask.query} ({subtaskResults.length} results)
                    </h5>
                    {subtaskResults.map((result, index) => (
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
                )
              })}
            </div>
          ) : (
            // Regular view
            task.results.map((result, index) => (
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
                      {result.subtask_id && task.subtasks && (
                        <div className="text-xs text-gray-500 mt-1">
                          From: {task.subtasks.find(st => st.id === result.subtask_id)?.query || 'Unknown subtask'}
                        </div>
                      )}
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
            ))
          )}
        </div>
      </div>
    </div>
  )
}