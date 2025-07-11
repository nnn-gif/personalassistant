import { useState } from 'react'
import { motion } from 'framer-motion'
import { Search, Loader2, Save, Tag } from 'lucide-react'
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
                className="mt-6"
              >
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-gray-400">
                    {progress.current_subtask || 'Preparing research...'}
                  </span>
                  <span className="text-sm text-gray-400">
                    {progress.completed_subtasks}/{progress.total_subtasks} tasks
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