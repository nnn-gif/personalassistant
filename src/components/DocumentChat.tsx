import React, { useState, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { motion, AnimatePresence } from 'framer-motion'
import { Send, Bot, User, FileText, Loader2 } from 'lucide-react'

interface ChatMessage {
  id: string
  content: string
  isUser: boolean
  timestamp: Date
  sources?: DocumentSource[]
  contextUsed?: boolean
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

export default function DocumentChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [inputMessage, setInputMessage] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [goals, setGoals] = useState<Goal[]>([])
  const [selectedGoal, setSelectedGoal] = useState<string>('')
  const [selectedModel, setSelectedModel] = useState<string>('llama3.2:1b')
  const [availableModels, setAvailableModels] = useState<string[]>([])
  const messagesEndRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    loadGoals()
    loadAvailableModels()
    // Add welcome message
    setMessages([{
      id: '1',
      content: "Hello! I'm your document assistant. I can help you find information from your indexed documents. Ask me anything!",
      isUser: false,
      timestamp: new Date(),
      contextUsed: false
    }])
  }, [])

  useEffect(() => {
    scrollToBottom()
  }, [messages])

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
      // Set first available model as default if current selection is not available
      if (models.length > 0 && !models.includes(selectedModel)) {
        setSelectedModel(models[0])
      }
    } catch (error) {
      console.error('Failed to load available models:', error)
      // Fallback to default models if API call fails
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
      timestamp: new Date()
    }

    setMessages(prev => [...prev, userMessage])
    setInputMessage('')
    setIsLoading(true)

    try {
      const response = await invoke<ChatResponse>('chat_with_documents', {
        query: inputMessage,
        goalId: selectedGoal || null,
        limit: 5,
        model: selectedModel || null
      })

      const assistantMessage: ChatMessage = {
        id: (Date.now() + 1).toString(),
        content: response.message,
        isUser: false,
        timestamp: new Date(),
        sources: response.sources,
        contextUsed: response.context_used
      }

      setMessages(prev => [...prev, assistantMessage])
    } catch (error) {
      console.error('Chat failed:', error)
      const errorMessage: ChatMessage = {
        id: (Date.now() + 1).toString(),
        content: "I'm sorry, I encountered an error while processing your request. Please try again.",
        isUser: false,
        timestamp: new Date(),
        contextUsed: false
      }
      setMessages(prev => [...prev, errorMessage])
    } finally {
      setIsLoading(false)
    }
  }

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      sendMessage()
    }
  }

  return (
    <div className="flex flex-col h-full max-h-[80vh] bg-dark-card rounded-lg border border-dark-border">
      {/* Header */}
      <div className="p-4 border-b border-dark-border">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <Bot className="w-6 h-6 text-primary" />
            <h2 className="text-xl font-bold text-white">Document Chat</h2>
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
          </div>
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

      {/* Input */}
      <div className="p-4 border-t border-dark-border">
        <div className="flex space-x-2">
          <textarea
            value={inputMessage}
            onChange={(e) => setInputMessage(e.target.value)}
            onKeyPress={handleKeyPress}
            placeholder="Ask me about your documents..."
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
            {selectedGoal ? `Searching in goal: ${goals.find(g => g.id === selectedGoal)?.name}` : 'Searching all documents'}
          </span>
          <span>
            Model: {selectedModel}
          </span>
        </div>
      </div>
    </div>
  )
}