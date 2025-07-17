import React, { useState, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { AnimatePresence } from 'framer-motion'
import { History } from 'lucide-react'
import ResearchResults from './research/ResearchResults'
import ChatHistory, { ChatConversationSummary } from './chat/ChatHistory'
import StreamingMessageList, { ChatMessage } from './chat/StreamingMessageList'
import ModeSelector, { ChatMode, modeConfig } from './chat/ModeSelector'
import ChatInput from './chat/ChatInput'
import ResearchProgressComponent, { ResearchProgress } from './chat/ResearchProgress'
import { useStreamingChat } from '../hooks/useStreamingChat'


interface Goal {
  id: string
  name: string
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
  
  // Streaming chat hook
  const { sendStreamingMessage, streamingMessages, clearStreamingMessage } = useStreamingChat()

  useEffect(() => {
    loadGoals()
    loadAvailableModels()
    loadConversations()
    // Set initial welcome message for default mode
    setMessages([{
      id: Date.now().toString(),
      content: `Welcome to ${modeConfig[currentMode].title}! ${modeConfig[currentMode].description}`,
      isUser: false,
      timestamp: new Date(),
      mode: currentMode
    }])
  }, [])

  useEffect(() => {
    scrollToBottom()
  }, [messages])

  useEffect(() => {
    // Cleanup previous research listener if exists
    if (researchUnlisten) {
      researchUnlisten()
      setResearchUnlisten(null)
    }

    // Setup research progress listener only for research mode
    if (currentMode === 'research') {
      const setupListener = async () => {
        const unlisten = await listen<ResearchProgress>('research-progress', (event) => {
          console.log('Research progress:', event.payload)
          setResearchProgress(event.payload)
          
          if (event.payload.status === 'completed' || event.payload.status === 'failed') {
            setTimeout(() => {
              setResearchProgress(null)
              setCurrentResearchTaskId(null)
            }, 3000)
          }
        })
        setResearchUnlisten(() => unlisten)
      }
      setupListener()
    } else {
      // Clear research state when switching away from research mode
      setResearchProgress(null)
      setCurrentResearchTaskId(null)
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
      // Set default model if available
      if (models.length > 0 && !models.includes(selectedModel)) {
        setSelectedModel(models[0])
      }
    } catch (error) {
      console.error('Failed to load models:', error)
      // Fallback to default model
      setAvailableModels(['llama3.2:1b'])
    }
  }

  const loadConversations = async () => {
    try {
      const convs = await invoke<ChatConversationSummary[]>('get_chat_conversations')
      setConversations(convs)
    } catch (error) {
      console.error('Failed to load conversations:', error)
    }
  }

  const loadConversation = async (conversationId: string) => {
    try {
      const messages = await invoke<ChatMessage[]>('get_chat_messages', { conversationId })
      const formattedMessages = messages.map(msg => ({
        ...msg,
        timestamp: new Date(msg.timestamp)
      }))
      setMessages(formattedMessages)
      setCurrentConversationId(conversationId)
      
      // Set mode based on conversation
      const conversation = conversations.find(c => c.id === conversationId)
      if (conversation) {
        setCurrentMode(conversation.mode)
      }
    } catch (error) {
      console.error('Failed to load conversation:', error)
    }
  }

  const deleteConversation = async (conversationId: string) => {
    try {
      await invoke('delete_chat_conversation', { conversationId })
      await loadConversations()
      
      // If deleting current conversation, clear messages
      if (conversationId === currentConversationId) {
        setCurrentConversationId(null)
        setMessages([{
          id: Date.now().toString(),
          content: `Welcome to ${modeConfig[currentMode].title}! ${modeConfig[currentMode].description}`,
          isUser: false,
          timestamp: new Date(),
          mode: currentMode
        }])
      }
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
      mode: currentMode
    }])
  }

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  const createNewConversation = async (mode: ChatMode, firstMessage: string) => {
    try {
      const conversationId = await invoke<string>('create_chat_conversation', {
        title: firstMessage.slice(0, 50) + (firstMessage.length > 50 ? '...' : ''),
        mode
      })
      setCurrentConversationId(conversationId)
      await loadConversations()
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
        mode: message.mode,
        sources: message.sources ? JSON.stringify(message.sources) : null,
        contextUsed: message.contextUsed,
        researchTaskId: message.researchTaskId
      })
    } catch (error) {
      console.error('Failed to save message:', error)
    }
  }

  const sendMessage = async () => {
    if (!inputMessage.trim() || isLoading) return

    const userMessageId = Date.now().toString()
    const userMessage: ChatMessage = {
      id: userMessageId,
      content: inputMessage,
      isUser: true,
      timestamp: new Date(),
      mode: currentMode
    }

    setMessages(prev => [...prev, userMessage])
    const messageToSend = inputMessage
    setInputMessage('')
    setIsLoading(true)

    try {
      // Create conversation if needed
      let conversationId = currentConversationId
      if (!conversationId) {
        conversationId = await createNewConversation(currentMode, messageToSend)
      }

      // Save user message
      if (conversationId) {
        await saveMessage(userMessage, conversationId)
      }

      // Create assistant message placeholder
      const assistantMessageId = Date.now().toString() + '-assistant'
      const assistantMessage: ChatMessage = {
        id: assistantMessageId,
        content: '',
        isUser: false,
        timestamp: new Date(),
        mode: currentMode
      }
      setMessages(prev => [...prev, assistantMessage])

      if (currentMode === 'research') {
        // Research mode - start research task
        try {
          const taskIdUuid = await invoke<string>('start_research', {
            query: messageToSend
          })
          setCurrentResearchTaskId(taskIdUuid)
          
          // Update message with research started notification
          setMessages(prev => prev.map(msg => 
            msg.id === assistantMessageId 
              ? { ...msg, content: "Research task started. I'll analyze your query and gather information from the web...", researchTaskId: taskIdUuid }
              : msg
          ))
        } catch (error) {
          console.error('Failed to start research:', error)
          setMessages(prev => prev.map(msg => 
            msg.id === assistantMessageId 
              ? { ...msg, content: `Failed to start research: ${error}` }
              : msg
          ))
        }
      } else {
        // Use streaming for general and knowledge modes
        await sendStreamingMessage(
          conversationId!,
          assistantMessageId,
          messageToSend,
          currentMode === 'knowledge' ? 'knowledge' : 'general',
          {
            model: selectedModel,
            goalId: selectedGoal || undefined,
            limit: 5
          }
        )

        // Wait for streaming to complete
        const checkComplete = setInterval(() => {
          const streamingMsg = streamingMessages.get(assistantMessageId)
          if (streamingMsg && streamingMsg.isComplete) {
            clearInterval(checkComplete)
            
            // Update message with final content
            setMessages(prev => prev.map(msg => 
              msg.id === assistantMessageId 
                ? { 
                    ...msg, 
                    content: streamingMsg.content || msg.content,
                    sources: streamingMsg.sources?.map(s => ({
                      document_id: s.document_id,
                      content: s.preview,
                      score: s.score
                    })),
                    contextUsed: !!streamingMsg.sources?.length
                  }
                : msg
            ))
            
            // Save assistant message
            if (conversationId && streamingMsg.content) {
              saveMessage({
                ...assistantMessage,
                content: streamingMsg.content,
                sources: streamingMsg.sources?.map(s => ({
                  document_id: s.document_id,
                  content: s.preview,
                  score: s.score
                })),
                contextUsed: !!streamingMsg.sources?.length
              }, conversationId)
            }
            
            // Clear streaming message
            clearStreamingMessage(assistantMessageId)
            setIsLoading(false)
          }
        }, 100)
        
        // Timeout after 2 minutes
        setTimeout(() => {
          clearInterval(checkComplete)
          setIsLoading(false)
        }, 120000)
        
        return // Exit early for streaming
      }
    } catch (error) {
      console.error('Failed to send message:', error)
      const errorMessage: ChatMessage = {
        id: (Date.now() + 1).toString(),
        content: `Error: ${error}`,
        isUser: false,
        timestamp: new Date(),
        mode: currentMode
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

  const handleModeChange = (mode: ChatMode) => {
    setCurrentMode(mode)
    // Clear current conversation when switching modes
    setCurrentConversationId(null)
    setMessages([{
      id: Date.now().toString(),
      content: `Switched to ${modeConfig[mode].title}. ${modeConfig[mode].description}`,
      isUser: false,
      timestamp: new Date(),
      mode
    }])
    
    // Clear research progress if switching away from research
    if (mode !== 'research') {
      setResearchProgress(null)
      setCurrentResearchTaskId(null)
    }
  }

  const getCurrentIcon = () => {
    const IconComponent = modeConfig[currentMode].icon
    return <IconComponent className="w-5 h-5" />
  }

  return (
    <div className="flex h-full max-h-[80vh] bg-dark-card rounded-lg border border-dark-border">
      {/* Chat History Sidebar */}
      {showHistory && (
        <ChatHistory
          conversations={conversations}
          currentConversationId={currentConversationId}
          onLoadConversation={loadConversation}
          onDeleteConversation={deleteConversation}
          onNewConversation={startNewConversation}
        />
      )}

      {/* Main Chat Area */}
      <div className="flex-1 flex flex-col">
        {/* Header */}
        <div className="p-4 border-b border-dark-border flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <button
              onClick={() => setShowHistory(!showHistory)}
              className="p-2 rounded-lg hover:bg-dark-border transition-colors"
              title="Toggle chat history"
            >
              <History className="w-5 h-5 text-gray-400" />
            </button>
            <div className="flex items-center space-x-2">
              {getCurrentIcon()}
              <h2 className="text-lg font-semibold text-white">{modeConfig[currentMode].title}</h2>
            </div>
          </div>
          <p className="text-sm text-gray-500">{modeConfig[currentMode].description}</p>
        </div>

        {/* Mode Selector */}
        <ModeSelector currentMode={currentMode} onModeChange={handleModeChange} />

        {/* Research Progress */}
        {researchProgress && currentMode === 'research' && (
          <ResearchProgressComponent progress={researchProgress} />
        )}

        {/* Messages */}
        <StreamingMessageList 
          messages={messages} 
          streamingMessages={streamingMessages}
          messagesEndRef={messagesEndRef} 
        />

        {/* Research Results */}
        <AnimatePresence>
          {currentResearchTaskId && currentMode === 'research' && (
            <div className="border-t border-dark-border">
              <ResearchResults taskId={currentResearchTaskId} />
            </div>
          )}
        </AnimatePresence>

        {/* Input Area */}
        <ChatInput
          inputMessage={inputMessage}
          isLoading={isLoading}
          currentMode={currentMode}
          selectedGoal={selectedGoal}
          selectedModel={selectedModel}
          goals={goals}
          availableModels={availableModels}
          onInputChange={setInputMessage}
          onSendMessage={sendMessage}
          onKeyPress={handleKeyPress}
          onGoalChange={setSelectedGoal}
          onModelChange={setSelectedModel}
        />
      </div>
    </div>
  )
}