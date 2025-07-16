import React, { useState, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { motion, AnimatePresence } from 'framer-motion'
import { Send, Bot, User, FileText, Loader2, MessageSquare, Search, BookOpen, History, Trash2, Plus } from 'lucide-react'
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

interface ChatConversationSummary {
  id: string
  title: string
  mode: ChatMode
  message_count: number
  last_message_at?: string
  created_at: string
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
  const [currentConversationId, setCurrentConversationId] = useState<string | null>(null)
  const [conversations, setConversations] = useState<ChatConversationSummary[]>([])
  const [showHistory, setShowHistory] = useState(false)
  const messagesEndRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    loadGoals()
    loadAvailableModels()
    loadConversations()
    // Set initial welcome message for default mode
    const welcomeMessage: ChatMessage = {
      id: '1',
      content: `Welcome to ${modeConfig[currentMode].title}! ${modeConfig[currentMode].description}`,
      isUser: false,
      timestamp: new Date(),
      mode: currentMode,
      contextUsed: false
    }
    setMessages([welcomeMessage])
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

  const loadConversations = async () => {
    try {
      const chatConversations = await invoke<ChatConversationSummary[]>('get_chat_conversations')
      setConversations(chatConversations)
      console.log('Loaded conversations:', chatConversations.length)
    } catch (error) {
      console.error('Failed to load conversations:', error)
    }
  }

  const loadConversation = async (conversationId: string) => {
    try {
      const chatMessages = await invoke<any[]>('get_chat_messages', { conversationId })
      
      // Convert stored messages to ChatMessage format
      const convertedMessages: ChatMessage[] = chatMessages.map(msg => ({
        id: msg.id,
        content: msg.content,
        isUser: msg.is_user,
        timestamp: new Date(msg.created_at),
        mode: msg.mode as ChatMode,
        sources: msg.sources ? JSON.parse(msg.sources) : undefined,
        contextUsed: msg.context_used,
        researchTaskId: msg.research_task_id
      }))

      setMessages(convertedMessages)
      setCurrentConversationId(conversationId)
      
      // Set mode to match the conversation
      if (convertedMessages.length > 0) {
        setCurrentMode(convertedMessages[0].mode)
      }
      
      console.log('Loaded conversation:', conversationId, 'with', convertedMessages.length, 'messages')
    } catch (error) {
      console.error('Failed to load conversation messages:', error)
    }
  }

  const deleteConversation = async (conversationId: string) => {
    try {
      await invoke('delete_chat_conversation', { conversationId })
      await loadConversations() // Refresh the list
      
      // If we're currently viewing the deleted conversation, start fresh
      if (currentConversationId === conversationId) {
        setCurrentConversationId(null)
        setMessages([{
          id: Date.now().toString(),
          content: `Welcome to ${modeConfig[currentMode].title}! ${modeConfig[currentMode].description}`,
          isUser: false,
          timestamp: new Date(),
          mode: currentMode,
          contextUsed: false
        }])
      }
      
      console.log('Deleted conversation:', conversationId)
    } catch (error) {
      console.error('Failed to delete conversation:', error)
    }
  }

  const startNewConversation = () => {
    setCurrentConversationId(null)
    setMessages([{
      id: Date.now().toString(),
      content: `Welcome to ${modeConfig[currentMode].title}! ${modeConfig[currentMode].description}`,
      isUser: false,
      timestamp: new Date(),
      mode: currentMode,
      contextUsed: false
    }])
  }

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  const createNewConversation = async (mode: ChatMode, firstMessage: string) => {
    try {
      const title = firstMessage.length > 50 
        ? firstMessage.substring(0, 50) + '...'
        : firstMessage
      
      const conversationId = await invoke<string>('create_chat_conversation', {
        title,
        mode: mode.toLowerCase()
      })
      
      setCurrentConversationId(conversationId)
      await loadConversations() // Refresh conversations list
      console.log('Created new conversation:', conversationId)
      return conversationId
    } catch (error) {
      console.error('Failed to create conversation:', error)
      return null
    }
  }

  const saveMessage = async (message: ChatMessage, conversationId: string) => {
    try {
      await invoke('save_chat_message', {
        conversationId,
        content: message.content,
        isUser: message.isUser,
        mode: message.mode.toLowerCase(),
        sources: message.sources ? JSON.stringify(message.sources) : null,
        contextUsed: message.contextUsed,
        researchTaskId: message.researchTaskId,
        metadata: null
      })
      console.log('Saved message:', message.id)
    } catch (error) {
      console.error('Failed to save message:', error)
    }
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

    // Create conversation if this is the first message
    let conversationId = currentConversationId
    if (!conversationId) {
      conversationId = await createNewConversation(currentMode, query)
      if (!conversationId) {
        setIsLoading(false)
        return
      }
    }

    // Save user message
    await saveMessage(userMessage, conversationId)

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
      
      // Save assistant message
      if (conversationId) {
        await saveMessage(assistantMessage, conversationId)
      }
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
      
      // Save error message
      if (conversationId) {
        await saveMessage(errorMessage, conversationId)
      }
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
    
    // Start a new conversation when switching modes
    setCurrentConversationId(null)
    setMessages([])
    
    setCurrentMode(mode)
    // Add a welcome message for the new mode
    const welcomeMessage: ChatMessage = {
      id: Date.now().toString(),
      content: `Welcome to ${modeConfig[mode].title}! ${modeConfig[mode].description}`,
      isUser: false,
      timestamp: new Date(),
      mode,
      contextUsed: false
    }
    setMessages([welcomeMessage])
  }

  const getCurrentIcon = () => {
    const IconComponent = modeConfig[currentMode].icon
    return <IconComponent className="w-6 h-6 text-primary" />
  }

  return (
    <div className="flex h-full max-h-[80vh] bg-dark-card rounded-lg border border-dark-border">
      {/* Chat History Sidebar */}
      {showHistory && (
        <div className="w-80 border-r border-dark-border bg-dark-surface">
          <div className="p-4 border-b border-dark-border">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-white flex items-center">
                <History className="w-5 h-5 mr-2" />
                Chat History
              </h3>
              <button
                onClick={() => setShowHistory(false)}
                className="text-gray-400 hover:text-white p-1"
              >
                ×
              </button>
            </div>
            <button
              onClick={startNewConversation}
              className="w-full flex items-center space-x-2 px-3 py-2 bg-primary text-white rounded-lg hover:bg-primary-hover transition-colors"
            >
              <Plus className="w-4 h-4" />
              <span>New Chat</span>
            </button>
          </div>
          
          <div className="flex-1 overflow-y-auto p-2 space-y-2">
            {conversations.map((conversation) => {
              const ModeIcon = modeConfig[conversation.mode].icon
              const isActive = currentConversationId === conversation.id
              
              return (
                <div
                  key={conversation.id}
                  className={`group relative p-3 rounded-lg cursor-pointer transition-colors ${
                    isActive
                      ? 'bg-primary/20 border border-primary/30'
                      : 'bg-dark-bg hover:bg-dark-border'
                  }`}
                  onClick={() => loadConversation(conversation.id)}
                >
                  <div className="flex items-start space-x-2">
                    <ModeIcon className="w-4 h-4 mt-0.5 text-primary flex-shrink-0" />
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium text-white truncate">
                        {conversation.title}
                      </p>
                      <p className="text-xs text-gray-400">
                        {conversation.message_count} messages
                      </p>
                      <p className="text-xs text-gray-500">
                        {conversation.last_message_at 
                          ? new Date(conversation.last_message_at).toLocaleDateString()
                          : new Date(conversation.created_at).toLocaleDateString()
                        }
                      </p>
                    </div>
                  </div>
                  
                  <button
                    onClick={(e) => {
                      e.stopPropagation()
                      deleteConversation(conversation.id)
                    }}
                    className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 p-1 text-gray-400 hover:text-red-400 transition-all"
                  >
                    <Trash2 className="w-3 h-3" />
                  </button>
                </div>
              )
            })}
            
            {conversations.length === 0 && (
              <div className="text-center text-gray-400 py-8">
                <History className="w-8 h-8 mx-auto mb-2 opacity-50" />
                <p className="text-sm">No chat history yet</p>
                <p className="text-xs">Start a conversation to see it here</p>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Main Chat Area */}
      <div className="flex flex-col flex-1">
      {/* Header with Mode Selector */}
      <div className="p-4 border-b border-dark-border">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center space-x-2">
            <button
              onClick={() => setShowHistory(!showHistory)}
              className="p-2 text-gray-400 hover:text-white hover:bg-dark-border rounded-lg transition-colors"
              title="Chat History"
            >
              <History className="w-5 h-5" />
            </button>
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
    </div>
  )
}