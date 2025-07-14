import React, { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { motion } from 'framer-motion'

interface IndexedDocument {
  id: string
  title: string
  file_path: string
  goal_id: string | null
  chunks_count: number
  created_at: string
}

interface IndexingProgress {
  isIndexing: boolean
  currentFile: string
  progress: number
  total: number
  phase?: string
  taskId?: string
  error?: string
}

interface FolderStats {
  folder_path: string
  total_files: number
  total_size: number
  file_types: Record<string, number>
  sample_files: string[]
}

interface IndexingResult {
  successful: Array<{
    id: string
    path: string
    title: string
    chunks_count: number
  }>
  failed: Array<{
    path: string
    error: string
  }>
  total_processed: number
}

interface Goal {
  id: string
  name: string
}

export default function DocumentManager() {
  const [indexedDocuments, setIndexedDocuments] = useState<IndexedDocument[]>([])
  const [goals, setGoals] = useState<Goal[]>([])
  const [selectedGoal, setSelectedGoal] = useState<string>('')
  const [supportedTypes, setSupportedTypes] = useState<string[]>([])
  const [indexingProgress, setIndexingProgress] = useState<IndexingProgress>({
    isIndexing: false,
    currentFile: '',
    progress: 0,
    total: 0
  })
  const [searchQuery, setSearchQuery] = useState('')
  const [folderStats, setFolderStats] = useState<FolderStats | null>(null)
  const [showFolderPreview, setShowFolderPreview] = useState(false)
  const [indexingResult, setIndexingResult] = useState<IndexingResult | null>(null)

  useEffect(() => {
    loadIndexedDocuments()
    loadGoals()
    loadSupportedTypes()
    
    // Set up event listeners for async indexing
    const setupEventListeners = async () => {
      const unlistenProgress = await listen('indexing-progress', (event: any) => {
        const progress = event.payload
        setIndexingProgress({
          isIndexing: progress.status === 'processing' || progress.status === 'starting',
          currentFile: progress.current_file,
          progress: progress.progress,
          total: progress.total,
          phase: progress.phase,
          taskId: progress.task_id,
          error: progress.error
        })
        
        if (progress.status === 'completed' || progress.status === 'error') {
          setTimeout(() => {
            setIndexingProgress({
              isIndexing: false,
              currentFile: '',
              progress: 0,
              total: 0
            })
          }, 2000) // Show completion for 2 seconds
        }
      })
      
      const unlistenIndexed = await listen('document-indexed', (event: any) => {
        console.log('Document indexed:', event.payload)
        loadIndexedDocuments() // Refresh the list
      })
      
      return () => {
        unlistenProgress()
        unlistenIndexed()
      }
    }
    
    // Set up keyboard event listeners for modal handling
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        if (showFolderPreview) {
          setShowFolderPreview(false)
          setFolderStats(null)
        }
        if (indexingResult) {
          setIndexingResult(null)
        }
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    setupEventListeners()

    return () => {
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [showFolderPreview, indexingResult])

  const loadIndexedDocuments = async () => {
    try {
      const docs = await invoke<IndexedDocument[]>('list_indexed_documents', {
        goalId: selectedGoal || null
      })
      setIndexedDocuments(docs)
    } catch (error) {
      console.error('Failed to load indexed documents:', error)
    }
  }

  const loadGoals = async () => {
    try {
      const goalData = await invoke<Goal[]>('get_goals')
      setGoals(goalData)
    } catch (error) {
      console.error('Failed to load goals:', error)
    }
  }

  const loadSupportedTypes = async () => {
    try {
      const types = await invoke<string[]>('get_supported_file_types')
      setSupportedTypes(types)
    } catch (error) {
      console.error('Failed to load supported types:', error)
    }
  }

  const handleSelectFiles = async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [{
          name: 'Documents',
          extensions: supportedTypes
        }]
      })

      if (selected && Array.isArray(selected)) {
        await indexMultipleFiles(selected)
      } else if (selected) {
        await indexSingleFile(selected as string)
      }
    } catch (error) {
      console.error('Error selecting files:', error)
    }
  }

  const handleSelectFolder = async () => {
    try {
      const selected = await open({
        directory: true
      })

      if (selected) {
        await previewFolder(selected as string)
      }
    } catch (error) {
      console.error('Error selecting folder:', error)
    }
  }

  const previewFolder = async (folderPath: string) => {
    try {
      const stats = await invoke<FolderStats>('get_folder_stats', {
        folderPath,
        includeSubdirs: true
      })
      setFolderStats(stats)
      setShowFolderPreview(true)
    } catch (error) {
      console.error('Failed to get folder stats:', error)
    }
  }

  const confirmFolderIndexing = async () => {
    if (!folderStats) return

    // Close the modal immediately to prevent UI issues
    setShowFolderPreview(false)
    setFolderStats(null)

    try {
      const files = await invoke<string[]>('scan_folder_for_documents', {
        folderPath: folderStats.folder_path,
        includeSubdirs: true,
        maxDepth: 10
      })

      setIndexingProgress({ 
        isIndexing: true, 
        currentFile: '', 
        progress: 0, 
        total: files.length 
      })

      const result = await invoke<IndexingResult>('index_multiple_documents', {
        filePaths: files,
        goalId: selectedGoal || null
      })

      setIndexingResult(result)
      await loadIndexedDocuments()
    } catch (error) {
      console.error('Failed to index folder:', error)
    } finally {
      setIndexingProgress({ isIndexing: false, currentFile: '', progress: 0, total: 0 })
    }
  }

  const indexSingleFile = async (filePath: string) => {
    const taskId = `index_${Date.now()}`
    
    try {
      await invoke('index_document_async', {
        filePath,
        goalId: selectedGoal || null,
        taskId
      })
      console.log('Async indexing started for:', filePath)
    } catch (error) {
      console.error('Failed to start indexing:', error)
      setIndexingProgress({ isIndexing: false, currentFile: '', progress: 0, total: 0 })
    }
  }

  const indexMultipleFiles = async (filePaths: string[]) => {
    const taskId = `batch_${Date.now()}`
    
    try {
      // For now, index files one by one with async calls
      for (const filePath of filePaths) {
        await indexSingleFile(filePath)
        // Small delay to prevent overwhelming the system
        await new Promise(resolve => setTimeout(resolve, 100))
      }
    } catch (error) {
      console.error('Failed to start batch indexing:', error)
      setIndexingProgress({ isIndexing: false, currentFile: '', progress: 0, total: 0 })
    }
  }

  const formatFileSize = (bytes: number) => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  const removeDocument = async (documentId: string) => {
    try {
      await invoke('remove_document', { documentId })
      await loadIndexedDocuments()
    } catch (error) {
      console.error('Failed to remove document:', error)
    }
  }

  const searchDocuments = async () => {
    if (!searchQuery.trim()) {
      return
    }

    try {
      const results = await invoke('search_documents', {
        query: searchQuery,
        goalId: selectedGoal || null,
        limit: 10
      })
      console.log('Search results:', results)
      // TODO: Display search results in a modal or separate view
    } catch (error) {
      console.error('Search failed:', error)
    }
  }

  const inspectDatabase = async () => {
    try {
      const result = await invoke('inspect_rag_database') as any
      console.log('=== RAG DATABASE INSPECTION ===')
      console.log(`Total Documents: ${result.total_documents}`)
      console.log(`Total Chunks: ${result.total_chunks}`)
      console.log(`Corrupted Documents: ${result.corrupted_documents}`)
      console.log('\n=== DOCUMENT DETAILS ===')
      
      result.documents.forEach((doc: any, index: number) => {
        console.log(`\n--- Document ${index + 1} ---`)
        console.log(`Title: ${doc.title}`)
        console.log(`File Path: ${doc.file_path}`)
        console.log(`Chunks: ${doc.chunks_count}`)
        console.log(`Content Length: ${doc.content_length} characters`)
        console.log(`Is Corrupted: ${doc.is_corrupted}`)
        console.log(`Content Preview: "${doc.content_preview}"`)
      })
      
      alert(`Database contains ${result.total_documents} documents, ${result.corrupted_documents} corrupted. Check console for details.`)
    } catch (error) {
      console.error('Database inspection failed:', error)
      alert('Failed to inspect database')
    }
  }

  const cleanupCorrupted = async () => {
    try {
      const result = await invoke('cleanup_corrupted_documents') as any
      console.log(`Cleaned up ${result.removed_count} corrupted documents`)
      console.log('Removed IDs:', result.removed_ids)
      alert(`Cleaned up ${result.removed_count} corrupted documents`)
      await loadIndexedDocuments() // Refresh the list
    } catch (error) {
      console.error('Cleanup failed:', error)
      alert('Failed to cleanup corrupted documents')
    }
  }

  const filteredDocuments = indexedDocuments.filter(doc =>
    doc.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
    doc.file_path.toLowerCase().includes(searchQuery.toLowerCase())
  )

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Document Manager</h2>
        <div className="flex items-center gap-4">
          <select
            value={selectedGoal}
            onChange={(e) => setSelectedGoal(e.target.value)}
            className="px-3 py-2 bg-dark-card border border-dark-border rounded-lg text-white"
          >
            <option value="">All Goals</option>
            {goals.map(goal => (
              <option key={goal.id} value={goal.id}>{goal.name}</option>
            ))}
          </select>
        </div>
      </div>

      {/* Action Buttons */}
      <div className="flex gap-4">
        <motion.button
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.95 }}
          onClick={handleSelectFiles}
          disabled={indexingProgress.isIndexing}
          className="px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary-hover disabled:opacity-50"
        >
          Add Files
        </motion.button>
        
        <motion.button
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.95 }}
          onClick={handleSelectFolder}
          disabled={indexingProgress.isIndexing}
          className="px-4 py-2 bg-secondary text-white rounded-lg hover:bg-secondary-hover disabled:opacity-50"
        >
          Add Folder
        </motion.button>

        <button
          onClick={loadIndexedDocuments}
          className="px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700"
        >
          Refresh
        </button>

        <button
          onClick={inspectDatabase}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
        >
          Inspect DB
        </button>

        <button
          onClick={cleanupCorrupted}
          className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700"
        >
          Clean Corrupted
        </button>
      </div>

      {/* Indexing Progress */}
      {indexingProgress.isIndexing && (
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-dark-card p-4 rounded-lg border border-dark-border"
        >
          <div className="flex items-center justify-between mb-2">
            <span className="text-white">
              {indexingProgress.phase || 'Indexing Documents...'}
            </span>
            <span className="text-sm text-gray-400">
              {indexingProgress.progress} / {indexingProgress.total}
            </span>
          </div>
          
          <div className="w-full bg-gray-700 rounded-full h-2 mb-2">
            <div
              className="bg-primary h-2 rounded-full transition-all duration-300"
              style={{ width: `${(indexingProgress.progress / Math.max(indexingProgress.total, 1)) * 100}%` }}
            />
          </div>
          
          {indexingProgress.currentFile && (
            <p className="text-sm text-gray-400 truncate">
              Current: {indexingProgress.currentFile}
            </p>
          )}
          
          {indexingProgress.error && (
            <p className="text-sm text-red-400 mt-2">
              Error: {indexingProgress.error}
            </p>
          )}
          
          {indexingProgress.taskId && (
            <p className="text-xs text-gray-500 mt-1">
              Task: {indexingProgress.taskId}
            </p>
          )}
        </motion.div>
      )}

      {/* Search Bar */}
      <div className="flex gap-2">
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search documents..."
          className="flex-1 px-3 py-2 bg-dark-card border border-dark-border rounded-lg text-white placeholder-gray-400"
        />
        <button
          onClick={searchDocuments}
          className="px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary-hover"
        >
          Search
        </button>
      </div>

      {/* Supported File Types */}
      <div className="bg-dark-card p-4 rounded-lg border border-dark-border">
        <h3 className="text-lg font-semibold text-white mb-2">Supported File Types</h3>
        <div className="flex flex-wrap gap-2">
          {supportedTypes.map(type => (
            <span
              key={type}
              className="px-2 py-1 bg-gray-700 text-gray-300 rounded text-sm"
            >
              .{type}
            </span>
          ))}
        </div>
      </div>

      {/* Documents List */}
      <div className="bg-dark-card rounded-lg border border-dark-border">
        <div className="p-4 border-b border-dark-border">
          <h3 className="text-lg font-semibold text-white">
            Indexed Documents ({filteredDocuments.length})
          </h3>
        </div>
        
        <div className="max-h-96 overflow-y-auto">
          {filteredDocuments.length === 0 ? (
            <div className="p-8 text-center text-gray-400">
              No documents indexed yet. Add some files to get started!
            </div>
          ) : (
            <div className="divide-y divide-dark-border">
              {filteredDocuments.map(doc => (
                <motion.div
                  key={doc.id}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  className="p-4 hover:bg-gray-800 transition-colors"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex-1 min-w-0">
                      <h4 className="text-white font-medium truncate">{doc.title}</h4>
                      <p className="text-sm text-gray-400 truncate">{doc.file_path}</p>
                      <div className="flex items-center gap-4 mt-1">
                        <span className="text-xs text-gray-500">
                          {doc.chunks_count} chunks
                        </span>
                        <span className="text-xs text-gray-500">
                          {new Date(doc.created_at).toLocaleDateString()}
                        </span>
                        {doc.goal_id && (
                          <span className="text-xs bg-primary px-2 py-1 rounded">
                            {goals.find(g => g.id === doc.goal_id)?.name || 'Unknown Goal'}
                          </span>
                        )}
                      </div>
                    </div>
                    
                    <button
                      onClick={() => removeDocument(doc.id)}
                      className="ml-4 px-3 py-1 bg-red-600 text-white rounded hover:bg-red-700 text-sm"
                    >
                      Remove
                    </button>
                  </div>
                </motion.div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Folder Preview Modal */}
      {showFolderPreview && folderStats && (
        <div 
          className="fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-[9999] backdrop-blur-sm"
          style={{ 
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            backgroundColor: 'rgba(0, 0, 0, 0.75)',
            backdropFilter: 'blur(2px)'
          }}
          onClick={(e) => {
            // Close modal when clicking on backdrop
            if (e.target === e.currentTarget) {
              setShowFolderPreview(false)
              setFolderStats(null)
            }
          }}
        >
          <motion.div
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.9 }}
            className="bg-gray-900 border border-gray-700 p-6 rounded-lg shadow-2xl max-w-2xl w-full mx-4 max-h-[80vh] overflow-y-auto"
            style={{
              backgroundColor: '#111827',
              border: '1px solid #374151',
              boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.5)'
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-xl font-bold text-white mb-4">Folder Preview</h3>
            
            <div className="space-y-4">
              <div>
                <p className="text-gray-400">Folder: {folderStats.folder_path}</p>
                <p className="text-white">Found {folderStats.total_files} indexable documents</p>
                <p className="text-gray-400">Total size: {formatFileSize(folderStats.total_size)}</p>
              </div>

              <div>
                <h4 className="text-lg font-semibold text-white mb-2">File Types</h4>
                <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
                  {Object.entries(folderStats.file_types).map(([type, count]) => (
                    <div key={type} className="bg-gray-700 px-3 py-2 rounded">
                      <span className="text-white">.{type}</span>
                      <span className="text-gray-400 ml-2">({count})</span>
                    </div>
                  ))}
                </div>
              </div>

              <div>
                <h4 className="text-lg font-semibold text-white mb-2">Sample Files</h4>
                <div className="max-h-40 overflow-y-auto space-y-1">
                  {folderStats.sample_files.map((file, index) => (
                    <p key={index} className="text-sm text-gray-400 truncate">{file}</p>
                  ))}
                  {folderStats.total_files > folderStats.sample_files.length && (
                    <p className="text-sm text-gray-500">
                      ... and {folderStats.total_files - folderStats.sample_files.length} more files
                    </p>
                  )}
                </div>
              </div>
            </div>

            <div className="flex gap-4 mt-6">
              <button
                onClick={confirmFolderIndexing}
                disabled={indexingProgress.isIndexing}
                className="px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary-hover disabled:opacity-50"
              >
                Index All Documents
              </button>
              <button
                onClick={() => {
                  setShowFolderPreview(false)
                  setFolderStats(null)
                }}
                className="px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700"
              >
                Cancel
              </button>
            </div>
          </motion.div>
        </div>
      )}

      {/* Indexing Result Modal */}
      {indexingResult && (
        <div 
          className="fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-[9999] backdrop-blur-sm"
          style={{ 
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            backgroundColor: 'rgba(0, 0, 0, 0.75)',
            backdropFilter: 'blur(2px)'
          }}
          onClick={(e) => {
            // Close modal when clicking on backdrop
            if (e.target === e.currentTarget) {
              setIndexingResult(null)
            }
          }}
        >
          <motion.div
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.9 }}
            className="bg-gray-900 border border-gray-700 p-6 rounded-lg shadow-2xl max-w-2xl w-full mx-4 max-h-[80vh] overflow-y-auto"
            style={{
              backgroundColor: '#111827',
              border: '1px solid #374151',
              boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.5)'
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-xl font-bold text-white mb-4">Indexing Complete</h3>
            
            <div className="space-y-4">
              <div className="grid grid-cols-3 gap-4 text-center">
                <div className="bg-green-900 p-3 rounded">
                  <p className="text-green-400 text-2xl font-bold">{indexingResult.successful.length}</p>
                  <p className="text-green-300 text-sm">Successful</p>
                </div>
                <div className="bg-red-900 p-3 rounded">
                  <p className="text-red-400 text-2xl font-bold">{indexingResult.failed.length}</p>
                  <p className="text-red-300 text-sm">Failed</p>
                </div>
                <div className="bg-blue-900 p-3 rounded">
                  <p className="text-blue-400 text-2xl font-bold">{indexingResult.total_processed}</p>
                  <p className="text-blue-300 text-sm">Total</p>
                </div>
              </div>

              {indexingResult.failed.length > 0 && (
                <div>
                  <h4 className="text-lg font-semibold text-red-400 mb-2">Failed Files</h4>
                  <div className="max-h-40 overflow-y-auto space-y-2">
                    {indexingResult.failed.map((fail, index) => (
                      <div key={index} className="bg-red-900 bg-opacity-30 p-2 rounded">
                        <p className="text-sm text-white truncate">{fail.path}</p>
                        <p className="text-xs text-red-300">{fail.error}</p>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>

            <button
              onClick={() => setIndexingResult(null)}
              className="mt-6 px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary-hover w-full"
            >
              Close
            </button>
          </motion.div>
        </div>
      )}
    </div>
  )
}