import { useState } from 'react'
import { motion } from 'framer-motion'
import { Search, Loader2, Tag } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import ResearchResults from './research/ResearchResults'
import SavedResearch from './research/SavedResearch'

interface ResearchProgress {
  task_id: string
  status: string
  current_subtask?: string
  completed_subtasks: number
  total_subtasks: number
  percentage: number
  current_operation?: string
  subtasks_progress: SubtaskProgress[]
  intermediate_results: ResearchResult[]
  phase_details?: PhaseDetails
}

interface SubtaskProgress {
  id: string
  query: string
  status: string
  current_operation?: string
  search_results_count: number
  scraped_pages_count: number
  results: ResearchResult[]
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

interface PhaseDetails {
  phase: string
  details: string
  estimated_completion?: string
}

export default function ResearchAssistant() {
  const [query, setQuery] = useState('')
  const [isSearching, setIsSearching] = useState(false)
  const [progress, setProgress] = useState<ResearchProgress | null>(null)
  const [currentTaskId, setCurrentTaskId] = useState<string | null>(null)
  const [showSaved, setShowSaved] = useState(false)

  const startResearch = async () => {
    if (!query.trim()) return

    console.log('Starting research for:', query)
    setIsSearching(true)
    setProgress(null)

    try {
      // Set up progress listener
      const unlisten = await listen<ResearchProgress>('browser-ai-progress', (event) => {
        console.log('Progress event:', event.payload)
        setProgress(event.payload)
      })

      // Start research
      console.log('Invoking start_research command...')
      const taskId = await invoke<string>('start_research', { query })
      console.log('Research task ID:', taskId)
      setCurrentTaskId(taskId)

      // Clean up listener when done
      setTimeout(() => {
        unlisten()
        setIsSearching(false)
      }, 60000) // 1 minute timeout
    } catch (error) {
      console.error('Research failed:', error)
      setIsSearching(false)
    }
  }

  const testCommand = async () => {
    try {
      const result = await invoke<string>('test_research')
      console.log('Test result:', result)
    } catch (error) {
      console.error('Test failed:', error)
    }
  }

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold">Research Assistant</h2>
          <p className="text-gray-400 mt-2">AI-powered web research at your fingertips</p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={testCommand}
            className="btn-secondary"
          >
            Test
          </button>
          <button
            onClick={() => setShowSaved(!showSaved)}
            className="btn-secondary"
          >
            <Tag className="w-4 h-4 mr-2" />
            {showSaved ? 'New Research' : 'Saved Research'}
          </button>
        </div>
      </header>

      {showSaved ? (
        <SavedResearch />
      ) : (
        <>
          <div className="card">
            <div className="flex space-x-4">
              <input
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyPress={(e) => e.key === 'Enter' && startResearch()}
                placeholder="What would you like to research?"
                className="input flex-1"
                disabled={isSearching}
              />
              <button
                onClick={startResearch}
                disabled={isSearching || !query.trim()}
                className="btn-primary flex items-center space-x-2"
              >
                {isSearching ? (
                  <Loader2 className="w-5 h-5 animate-spin" />
                ) : (
                  <Search className="w-5 h-5" />
                )}
                <span>{isSearching ? 'Researching...' : 'Research'}</span>
              </button>
            </div>

            {progress && (
              <motion.div
                initial={{ opacity: 0, y: -10 }}
                animate={{ opacity: 1, y: 0 }}
                className="mt-6 space-y-4"
              >
                {/* Main Progress */}
                <div>
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-sm text-gray-300 font-medium">
                      {progress.current_operation || getStatusMessage(progress.status)}
                    </span>
                    <span className="text-sm text-gray-400">
                      {Math.round(progress.percentage)}%
                    </span>
                  </div>
                  <div className="w-full bg-dark-bg rounded-full h-2">
                    <motion.div
                      className="bg-primary h-2 rounded-full"
                      initial={{ width: 0 }}
                      animate={{ width: `${progress.percentage}%` }}
                      transition={{ duration: 0.3 }}
                    />
                  </div>
                  {progress.phase_details && (
                    <p className="text-xs text-gray-500 mt-1">{progress.phase_details.details}</p>
                  )}
                </div>

                {/* Subtasks Progress */}
                {progress.subtasks_progress.length > 0 && (
                  <div className="space-y-3">
                    <h4 className="text-sm font-medium text-gray-300">Research Tasks:</h4>
                    {progress.subtasks_progress.map((subtask) => (
                      <motion.div
                        key={subtask.id}
                        initial={{ opacity: 0, x: -20 }}
                        animate={{ opacity: 1, x: 0 }}
                        className="bg-dark-bg rounded-lg p-3 border border-gray-700"
                      >
                        <div className="flex items-center justify-between mb-2">
                          <span className="text-sm text-gray-300 truncate flex-1 mr-2">
                            {subtask.query}
                          </span>
                          <span className={`text-xs px-2 py-1 rounded-full ${getStatusColor(subtask.status)}`}>
                            {subtask.status}
                          </span>
                        </div>
                        {subtask.current_operation && (
                          <p className="text-xs text-gray-400 mb-2">{subtask.current_operation}</p>
                        )}
                        <div className="flex items-center gap-4 text-xs text-gray-500">
                          {subtask.search_results_count > 0 && (
                            <span>{subtask.search_results_count} search results</span>
                          )}
                          {subtask.scraped_pages_count > 0 && (
                            <span>{subtask.scraped_pages_count} pages scraped</span>
                          )}
                          {subtask.results.length > 0 && (
                            <span className="text-green-400">{subtask.results.length} findings</span>
                          )}
                        </div>
                      </motion.div>
                    ))}
                  </div>
                )}

                {/* Intermediate Results */}
                {progress.intermediate_results.length > 0 && (
                  <div className="space-y-2">
                    <h4 className="text-sm font-medium text-gray-300">Latest Findings:</h4>
                    <div className="max-h-40 overflow-y-auto space-y-2">
                      {progress.intermediate_results.slice(-3).map((result) => (
                        <motion.div
                          key={result.id}
                          initial={{ opacity: 0, scale: 0.95 }}
                          animate={{ opacity: 1, scale: 1 }}
                          className="bg-dark-bg rounded-lg p-2 border border-gray-700"
                        >
                          <div className="flex items-start justify-between">
                            <div className="flex-1">
                              <h5 className="text-sm font-medium text-gray-200 truncate">{result.title}</h5>
                              <p className="text-xs text-gray-400 truncate">{result.url}</p>
                              <p className="text-xs text-gray-300 mt-1 line-clamp-2">{result.content.slice(0, 120)}...</p>
                            </div>
                            <span className="text-xs text-green-400 ml-2">
                              {Math.round(result.relevance_score * 100)}%
                            </span>
                          </div>
                        </motion.div>
                      ))}
                    </div>
                  </div>
                )}
              </motion.div>
            )}
          </div>

          {currentTaskId && !isSearching && (
            <ResearchResults taskId={currentTaskId} />
          )}
        </>
      )}
    </div>
  )
}

function getStatusMessage(status: string): string {
  switch (status) {
    case 'SplittingTasks':
      return 'Creating research plan...'
    case 'Searching':
      return 'Searching the web...'
    case 'Scraping':
      return 'Extracting content...'
    case 'Analyzing':
      return 'Analyzing findings...'
    case 'Completed':
      return 'Research completed!'
    default:
      return 'Preparing research...'
  }
}

function getStatusColor(status: string): string {
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