import React, { useMemo } from 'react'
import { motion } from 'framer-motion'
import { Bot, User, FileText, AlertCircle } from 'lucide-react'
import { formatDistanceToNow } from 'date-fns'
import { ThinkingIndicator } from './ThinkingIndicator'
import { StreamingMessage } from '../../hooks/useStreamingChat'

export interface ChatMessage {
  id: string
  content: string
  isUser: boolean
  timestamp: Date
  mode: 'general' | 'knowledge' | 'research'
  sources?: DocumentSource[]
  contextUsed?: boolean
  researchTaskId?: string
}

export interface DocumentSource {
  document_id: string
  content: string
  score: number
}

interface StreamingMessageListProps {
  messages: ChatMessage[]
  streamingMessages: Map<string, StreamingMessage>
  messagesEndRef: React.RefObject<HTMLDivElement>
}

const StreamingMessageContent: React.FC<{
  message: ChatMessage
  streamingData?: StreamingMessage
}> = ({ message, streamingData }) => {
  const displayContent = streamingData?.content || message.content
  const isStreaming = streamingData && !streamingData.isComplete
  const hasError = streamingData?.error

  if (hasError) {
    return (
      <div className="flex items-center space-x-2 text-red-400">
        <AlertCircle className="w-4 h-4" />
        <span className="text-sm">{streamingData.error}</span>
      </div>
    )
  }

  return (
    <>
      {streamingData?.isThinking && streamingData.metadata?.thinkingContent && (
        <div className="mb-3">
          <ThinkingIndicator
            step={streamingData.thinkingStep}
            progress={streamingData.thinkingProgress}
            isVisible={true}
            thinkingContent={streamingData.metadata?.thinkingContent}
          />
        </div>
      )}
      
      {displayContent && (
        <div className="whitespace-pre-wrap">
          {displayContent}
          {isStreaming && !streamingData.isThinking && (
            <motion.span
              className="inline-block w-2 h-4 bg-current ml-1"
              animate={{ opacity: [0, 1, 0] }}
              transition={{ duration: 1, repeat: Infinity }}
            />
          )}
        </div>
      )}

      {/* Display sources */}
      {(streamingData?.sources || message.sources) && (streamingData?.sources || message.sources)!.length > 0 && (
        <div className="mt-3 pt-3 border-t border-gray-700">
          <p className="text-xs text-gray-400 mb-2 flex items-center">
            <FileText className="w-3 h-3 mr-1" />
            Sources ({(streamingData?.sources || message.sources)!.length})
          </p>
          <div className="space-y-2 max-h-32 overflow-y-auto">
            {(streamingData?.sources || message.sources)!.map((source: any, idx: number) => (
              <motion.div 
                key={idx} 
                className="text-xs bg-dark-bg/50 p-2 rounded"
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ delay: idx * 0.1 }}
              >
                <p className="text-gray-300 line-clamp-2">
                  {source.preview || source.content}
                </p>
                <p className="text-gray-500 mt-1">
                  Score: {(source.score * 100).toFixed(1)}%
                </p>
              </motion.div>
            ))}
          </div>
        </div>
      )}
    </>
  )
}

export default function StreamingMessageList({ 
  messages, 
  streamingMessages,
  messagesEndRef 
}: StreamingMessageListProps) {
  // Combine regular messages with their streaming data
  const messagesWithStreaming = useMemo(() => {
    return messages.map(message => ({
      message,
      streamingData: streamingMessages.get(message.id)
    }))
  }, [messages, streamingMessages])

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-4">
      {messagesWithStreaming.map(({ message, streamingData }, index) => (
        <motion.div
          key={message.id}
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: index * 0.05 }}
          className={`flex ${message.isUser ? 'justify-end' : 'justify-start'}`}
        >
          <div className={`flex items-start space-x-3 max-w-3xl ${message.isUser ? 'flex-row-reverse space-x-reverse' : ''}`}>
            <div className={`flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center ${
              message.isUser ? 'bg-primary' : 'bg-dark-border'
            }`}>
              {message.isUser ? (
                <User className="w-4 h-4 text-white" />
              ) : (
                <Bot className={`w-4 h-4 text-primary ${streamingData?.isThinking ? 'animate-pulse' : ''}`} />
              )}
            </div>
            <div className={`flex-1 ${message.isUser ? 'text-right' : ''}`}>
              <div className={`inline-block px-4 py-2 rounded-lg ${
                message.isUser ? 'bg-primary text-white' : 'bg-dark-border text-gray-100'
              } ${streamingData && !streamingData.isComplete ? 'min-w-[200px]' : ''}`}>
                <StreamingMessageContent 
                  message={message} 
                  streamingData={streamingData}
                />
              </div>
              <p className="text-xs text-gray-500 mt-1 flex items-center">
                {streamingData && !streamingData.isComplete ? (
                  streamingData.isThinking ? (
                    <>
                      <span className="text-primary">{streamingData.thinkingStep || 'Thinking'}</span>
                      <span className="ml-1 flex">
                        {[0, 1, 2].map((i) => (
                          <motion.span
                            key={i}
                            className="text-primary"
                            animate={{
                              opacity: [0.2, 1, 0.2],
                            }}
                            transition={{
                              duration: 1.5,
                              repeat: Infinity,
                              delay: i * 0.2,
                            }}
                          >
                            •
                          </motion.span>
                        ))}
                      </span>
                      {streamingData.thinkingProgress !== undefined && (
                        <span className="ml-2">({Math.round(streamingData.thinkingProgress * 100)}%)</span>
                      )}
                    </>
                  ) : (
                    <>
                      {formatDistanceToNow(message.timestamp, { addSuffix: true })}
                      <span className="ml-2 text-primary">• Streaming</span>
                    </>
                  )
                ) : (
                  formatDistanceToNow(message.timestamp, { addSuffix: true })
                )}
              </p>
            </div>
          </div>
        </motion.div>
      ))}
      <div ref={messagesEndRef} />
    </div>
  )
}