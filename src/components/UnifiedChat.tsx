import React, { useState, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { motion, AnimatePresence } from 'framer-motion'
import { Send, Bot, User, FileText, Loader2, MessageSquare, Search, BookOpen } from 'lucide-react'
import ResearchResults from './research/ResearchResults'

type ChatMode = 'general' | 'knowledge' | 'research'

interface ChatMessage {
  id: string
  content: string
  isUser: boolean
  timestamp: Date
  mode: ChatMode
  sources?: DocumentSource[]
  contextUsed?: boolean
  researchTaskId?: string
}

interface DocumentSource {
  document_id: string
  content: string
  score: number
}

interface ChatResponse {
  message: string
  sources: DocumentSource[]
  context_used: boolean
}

interface Goal {
  id: string
  name: string
}

interface ResearchProgress {
  task_id: string
  status: string
  current_subtask?: string
  completed_subtasks: number
  total_subtasks: number
  percentage: number
  current_operation?: string
}

const modeConfig = {
  general: {
    icon: MessageSquare,
    title: 'General Assistant',
    placeholder: 'Ask me anything...',
    description: 'Open-ended conversation and general assistance'
  },
  knowledge: {
    icon: BookOpen,
    title: 'Knowledge Assistant',
    placeholder: 'Ask about your documents...',
    description: 'Search and chat with your indexed documents'
  },
  research: {
    icon: Search,
    title: 'Research Assistant',
    placeholder: 'What would you like to research?',
    description: 'AI-powered web research and analysis'
  }
}

export default function UnifiedChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [inputMessage, setInputMessage] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [currentMode, setCurrentMode] = useState<ChatMode>('general')
  const [goals, setGoals] = useState<Goal[]>([])
  const [selectedGoal, setSelectedGoal] = useState<string>('')
  const [selectedModel, setSelectedModel] = useState<string>('llama3.2:1b')
  const [availableModels, setAvailableModels] = useState<string[]>([])
  const [researchProgress, setResearchProgress] = useState<ResearchProgress | null>(null)
  const [currentResearchTaskId, setCurrentResearchTaskId] = useState<string | null>(null)
  const [researchUnlisten, setResearchUnlisten] = useState<(() => void) | null>(null)
  const messagesEndRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    loadGoals()
    loadAvailableModels()
    setMessages([{
      id: '1',
      content: "Hello! I'm your AI assistant. Choose a mode above to get started, or just start chatting!",
      isUser: false,
      timestamp: new Date(),
      mode: 'general',
      contextUsed: false
    }])
  }, [])

  useEffect(() => {
    scrollToBottom()
  }, [messages])

  useEffect(() => {
    if (currentMode === 'research' && !researchUnlisten) {
      listen<ResearchProgress>('browser-ai-progress', (event) => {
        console.log('Research progress:', event.payload)
        setResearchProgress(event.payload)
        
        // Check if research is completed
        if (event.payload.status === 'Completed') {
          setIsLoading(false)
        }
      }).then(fn => {
        setResearchUnlisten(() => fn)
      })
    }

    return () => {
      // Cleanup will be handled by mode change
    }
  }, [currentMode])

  const loadGoals = async () => {
    try {
      const goalData = await invoke<Goal[]>('get_goals')
      setGoals(goalData)
    } catch (error) {
      console.error('Failed to load goals:', error)
    }
  }

  const loadAvailableModels = async () => {
    try {
      const models = await invoke<string[]>('get_available_models')
      setAvailableModels(models)
      if (models.length > 0 && !models.includes(selectedModel)) {
        setSelectedModel(models[0])
      }
    } catch (error) {
      console.error('Failed to load available models:', error)
      setAvailableModels([
        'llama3.2:1b', 'llama3.2:3b', 'llama3.1:8b', 'llama3.1:70b',
        'qwen2.5:1.5b', 'qwen2.5:3b', 'qwen2.5:7b',
        'gemma2:2b', 'gemma2:9b', 'phi3.5:3.8b',
        'codellama:7b', 'codellama:13b', 'mistral:7b', 'mixtral:8x7b'
      ])
    }
  }

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  const sendMessage = async () => {
    if (!inputMessage.trim() || isLoading) return

    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      content: inputMessage,
      isUser: true,
      timestamp: new Date(),
      mode: currentMode
    }

    setMessages(prev => [...prev, userMessage])
    const query = inputMessage
    setInputMessage('')
    setIsLoading(true)

    try {
      let assistantMessage: ChatMessage

      switch (currentMode) {
        case 'knowledge':
          const knowledgeResponse = await invoke<ChatResponse>('chat_with_documents', {
            query,
            goalId: selectedGoal || null,
            limit: 5,
            model: selectedModel || null
          })

          assistantMessage = {
            id: (Date.now() + 1).toString(),
            content: knowledgeResponse.message,
            isUser: false,
            timestamp: new Date(),
            mode: currentMode,
            sources: knowledgeResponse.sources,
            contextUsed: knowledgeResponse.context_used
          }
          break

        case 'research':
          const taskId = await invoke<string>('start_research', { query })
          setCurrentResearchTaskId(taskId)
          
          // Set up timeout to stop loading after 2 minutes if not completed
          setTimeout(() => {
            setIsLoading(false)
            if (researchUnlisten) {
              researchUnlisten()
              setResearchUnlisten(null)
            }
          }, 120000) // 2 minute timeout
          
          assistantMessage = {
            id: (Date.now() + 1).toString(),
            content: `Starting research on: "${query}". I'll gather information from multiple sources and provide you with a comprehensive analysis.`,
            isUser: false,
            timestamp: new Date(),
            mode: currentMode,
            researchTaskId: taskId
          }
          break

        case 'general':
        default:
          // For general chat, we'll use a simple LLM call without RAG
          const generalResponse = await invoke<string>('general_chat', {
            message: query,
            model: selectedModel || null
          })

          assistantMessage = {
            id: (Date.now() + 1).toString(),
            content: generalResponse,
            isUser: false,
            timestamp: new Date(),
            mode: currentMode
          }
          break
      }

      setMessages(prev => [...prev, assistantMessage])
    } catch (error) {
      console.error('Chat failed:', error)
      const errorMessage: ChatMessage = {
        id: (Date.now() + 1).toString(),
        content: "I'm sorry, I encountered an error while processing your request. Please try again.",
        isUser: false,
        timestamp: new Date(),
        mode: currentMode,
        contextUsed: false
      }
      setMessages(prev => [...prev, errorMessage])
    } finally {
      setIsLoading(false)
      setResearchProgress(null)
    }
  }

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      sendMessage()
    }
  }

  const handleModeChange = (mode: ChatMode) => {
    // Clean up research state when switching away from research mode
    if (currentMode === 'research' && mode !== 'research') {
      setCurrentResearchTaskId(null)
      setResearchProgress(null)
      if (researchUnlisten) {
        researchUnlisten()
        setResearchUnlisten(null)
      }
    }
    
    setCurrentMode(mode)
    // Add a system message about mode change
    const modeMessage: ChatMessage = {
      id: Date.now().toString(),
      content: `Switched to ${modeConfig[mode].title}. ${modeConfig[mode].description}`,
      isUser: false,
      timestamp: new Date(),
      mode,
      contextUsed: false
    }
    setMessages(prev => [...prev, modeMessage])
  }

  const getCurrentIcon = () => {
    const IconComponent = modeConfig[currentMode].icon
    return <IconComponent className="w-6 h-6 text-primary" />
  }

  return (
    <div className="flex flex-col h-full max-h-[80vh] bg-dark-card rounded-lg border border-dark-border">
      {/* Header with Mode Selector */}
      <div className="p-4 border-b border-dark-border">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center space-x-2">
            {getCurrentIcon()}
            <h2 className="text-xl font-bold text-white">Assistant Chat</h2>
          </div>
          
          <div className="flex items-center space-x-4">
            <select
              value={selectedModel}
              onChange={(e) => setSelectedModel(e.target.value)}
              className="px-3 py-1 bg-dark-bg border border-dark-border rounded text-white text-sm"
            >
              {availableModels.map(model => (
                <option key={model} value={model}>{model}</option>
              ))}
            </select>
            
            {currentMode === 'knowledge' && (
              <select
                value={selectedGoal}
                onChange={(e) => setSelectedGoal(e.target.value)}
                className="px-3 py-1 bg-dark-bg border border-dark-border rounded text-white text-sm"
              >
                <option value="">All Documents</option>
                {goals.map(goal => (
                  <option key={goal.id} value={goal.id}>{goal.name}</option>
                ))}
              </select>
            )}
          </div>
        </div>

        {/* Mode Selector */}
        <div className="flex space-x-1 bg-dark-bg rounded-lg p-1">
          {(Object.keys(modeConfig) as ChatMode[]).map((mode) => {
            const config = modeConfig[mode]
            const IconComponent = config.icon
            const isActive = currentMode === mode
            
            return (
              <button
                key={mode}
                onClick={() => handleModeChange(mode)}
                className={`flex items-center space-x-2 px-3 py-2 rounded-md text-sm font-medium transition-all ${
                  isActive
                    ? 'bg-primary text-white'
                    : 'text-gray-400 hover:text-white hover:bg-dark-border'
                }`}
              >
                <IconComponent className="w-4 h-4" />
                <span>{config.title}</span>
              </button>
            )
          })}
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        <AnimatePresence>
          {messages.map((message) => (
            <motion.div
              key={message.id}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -20 }}
              className={`flex ${message.isUser ? 'justify-end' : 'justify-start'}`}
            >
              <div className={`max-w-[80%] ${message.isUser ? 'order-2' : 'order-1'}`}>
                <div
                  className={`p-3 rounded-lg ${
                    message.isUser
                      ? 'bg-primary text-white'
                      : 'bg-dark-bg text-gray-100'
                  }`}
                >
                  <div className="flex items-start space-x-2">
                    {!message.isUser && (
                      <Bot className="w-4 h-4 mt-0.5 text-primary flex-shrink-0" />
                    )}
                    {message.isUser && (
                      <User className="w-4 h-4 mt-0.5 text-white flex-shrink-0" />
                    )}
                    <div className="flex-1">
                      <p className="whitespace-pre-wrap">{message.content}</p>
                      
                      {/* Mode indicator */}
                      {!message.isUser && (
                        <div className="mt-1 flex items-center text-xs text-gray-400">
                          {React.createElement(modeConfig[message.mode].icon, { className: "w-3 h-3 mr-1" })}
                          {modeConfig[message.mode].title}
                        </div>
                      )}
                      
                      {/* Context indicator */}
                      {!message.isUser && message.contextUsed && (
                        <div className="mt-2 flex items-center text-xs text-green-400">
                          <FileText className="w-3 h-3 mr-1" />
                          Answer based on your documents
                        </div>
                      )}
                      
                      {/* Sources */}
                      {message.sources && message.sources.length > 0 && (
                        <div className="mt-3 space-y-2">
                          <div className="text-xs text-gray-400 font-medium">Sources:</div>
                          {message.sources.slice(0, 3).map((source, index) => (
                            <div
                              key={index}
                              className="text-xs bg-gray-800 p-2 rounded border-l-2 border-primary"
                            >
                              <div className="flex items-center justify-between mb-1">
                                <span className="text-gray-300">Document {index + 1}</span>
                                <span className="text-green-400">
                                  {Math.round(source.score * 100)}% relevance
                                </span>
                              </div>
                              <p className="text-gray-400 line-clamp-2">
                                {source.content.substring(0, 100)}...
                              </p>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
                
                <div className="text-xs text-gray-500 mt-1 px-2">
                  {message.timestamp.toLocaleTimeString()}
                </div>
              </div>
            </motion.div>
          ))}
        </AnimatePresence>
        
        {/* Research Progress */}
        {researchProgress && currentMode === 'research' && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-dark-bg p-4 rounded-lg border border-dark-border"
          >
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm text-gray-300 font-medium">
                {researchProgress.current_operation || 'Researching...'}
              </span>
              <span className="text-sm text-gray-400">
                {Math.round(researchProgress.percentage)}%
              </span>
            </div>
            <div className="w-full bg-gray-700 rounded-full h-2">
              <motion.div
                className="bg-primary h-2 rounded-full"
                initial={{ width: 0 }}
                animate={{ width: `${researchProgress.percentage}%` }}
                transition={{ duration: 0.3 }}
              />
            </div>
          </motion.div>
        )}
        
        {isLoading && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="flex justify-start"
          >
            <div className="bg-dark-bg text-gray-100 p-3 rounded-lg max-w-[80%]">
              <div className="flex items-center space-x-2">
                <Bot className="w-4 h-4 text-primary" />
                <Loader2 className="w-4 h-4 animate-spin text-primary" />
                <span>Thinking...</span>
              </div>
            </div>
          </motion.div>
        )}
        
        <div ref={messagesEndRef} />
      </div>

      {/* Research Results */}
      {currentMode === 'research' && currentResearchTaskId && !isLoading && (
        <div className="p-4 border-t border-dark-border">
          <ResearchResults taskId={currentResearchTaskId} />
        </div>
      )}

      {/* Input */}
      <div className="p-4 border-t border-dark-border">
        <div className="flex space-x-2">
          <textarea
            value={inputMessage}
            onChange={(e) => setInputMessage(e.target.value)}
            onKeyPress={handleKeyPress}
            placeholder={modeConfig[currentMode].placeholder}
            className="flex-1 px-3 py-2 bg-dark-bg border border-dark-border rounded-lg text-white placeholder-gray-400 resize-none"
            rows={1}
            disabled={isLoading}
          />
          <motion.button
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
            onClick={sendMessage}
            disabled={!inputMessage.trim() || isLoading}
            className="px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary-hover disabled:opacity-50 disabled:cursor-not-allowed flex items-center space-x-2"
          >
            <Send className="w-4 h-4" />
          </motion.button>
        </div>
        
        <div className="text-xs text-gray-500 mt-2 flex items-center justify-between">
          <span>
            {currentMode === 'knowledge' && selectedGoal 
              ? `Searching in goal: ${goals.find(g => g.id === selectedGoal)?.name}` 
              : modeConfig[currentMode].description}
          </span>
          <span>
            Model: {selectedModel}
          </span>
        </div>
      </div>
    </div>
  )
}