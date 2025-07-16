import React from 'react'
import { motion } from 'framer-motion'
import { Bot, User, FileText } from 'lucide-react'
import { formatDistanceToNow } from 'date-fns'

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

interface MessageListProps {
  messages: ChatMessage[]
  messagesEndRef: React.RefObject<HTMLDivElement>
}

export default function MessageList({ messages, messagesEndRef }: MessageListProps) {
  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-4">
      {messages.map((message, index) => (
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
              {message.isUser ? <User className="w-4 h-4 text-white" /> : <Bot className="w-4 h-4 text-primary" />}
            </div>
            <div className={`flex-1 ${message.isUser ? 'text-right' : ''}`}>
              <div className={`inline-block px-4 py-2 rounded-lg ${
                message.isUser ? 'bg-primary text-white' : 'bg-dark-border text-gray-100'
              }`}>
                <p className="whitespace-pre-wrap">{message.content}</p>
                {message.sources && message.sources.length > 0 && (
                  <div className="mt-3 pt-3 border-t border-gray-700">
                    <p className="text-xs text-gray-400 mb-2 flex items-center">
                      <FileText className="w-3 h-3 mr-1" />
                      Sources ({message.sources.length})
                    </p>
                    <div className="space-y-2 max-h-32 overflow-y-auto">
                      {message.sources.map((source, idx) => (
                        <div key={idx} className="text-xs bg-dark-bg/50 p-2 rounded">
                          <p className="text-gray-300 line-clamp-2">{source.content}</p>
                          <p className="text-gray-500 mt-1">Score: {(source.score * 100).toFixed(1)}%</p>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
              <p className="text-xs text-gray-500 mt-1">
                {formatDistanceToNow(message.timestamp, { addSuffix: true })}
              </p>
            </div>
          </div>
        </motion.div>
      ))}
      <div ref={messagesEndRef} />
    </div>
  )
}