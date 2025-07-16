import { History, Trash2, Plus, MessageSquare } from 'lucide-react'
import { formatDistanceToNow } from 'date-fns'
import { motion } from 'framer-motion'

export interface ChatConversationSummary {
  id: string
  title: string
  mode: 'general' | 'knowledge' | 'research'
  message_count: number
  last_message_at?: string
  created_at: string
}

interface ChatHistoryProps {
  conversations: ChatConversationSummary[]
  currentConversationId: string | null
  onLoadConversation: (conversationId: string) => void
  onDeleteConversation: (conversationId: string) => void
  onNewConversation: () => void
}

const modeIcons = {
  general: '💬',
  knowledge: '📚',
  research: '🔍'
}

export default function ChatHistory({
  conversations,
  currentConversationId,
  onLoadConversation,
  onDeleteConversation,
  onNewConversation
}: ChatHistoryProps) {
  return (
    <div className="w-80 border-r border-dark-border bg-dark-surface">
      <div className="p-4 border-b border-dark-border">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-white flex items-center">
            <History className="w-5 h-5 mr-2" />
            Chat History
          </h3>
          <button
            onClick={onNewConversation}
            className="p-2 rounded-lg bg-primary/20 hover:bg-primary/30 text-primary transition-colors"
            title="New Chat"
          >
            <Plus className="w-4 h-4" />
          </button>
        </div>
      </div>

      <div className="overflow-y-auto max-h-[calc(100%-5rem)]">
        <div className="p-4 space-y-2">
          {conversations.length === 0 ? (
            <div className="text-center text-gray-500 py-8">
              <MessageSquare className="w-12 h-12 mx-auto mb-2 opacity-50" />
              <p>No chat history yet</p>
            </div>
          ) : (
            conversations.map((conversation) => {
              const isActive = conversation.id === currentConversationId
              return (
                <motion.div
                  key={conversation.id}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  className={`group relative p-3 rounded-lg cursor-pointer transition-colors ${
                    isActive
                      ? 'bg-primary/20 border border-primary/30'
                      : 'bg-dark-bg hover:bg-dark-border'
                  }`}
                  onClick={() => onLoadConversation(conversation.id)}
                >
                  <div className="flex items-start space-x-2">
                    <span className="text-lg mt-0.5">{modeIcons[conversation.mode]}</span>
                    <div className="flex-1 min-w-0">
                      <h4 className="text-sm font-medium text-white truncate">
                        {conversation.title}
                      </h4>
                      <p className="text-xs text-gray-500 mt-0.5">
                        {conversation.message_count} messages • {' '}
                        {conversation.last_message_at
                          ? formatDistanceToNow(new Date(conversation.last_message_at), { addSuffix: true })
                          : formatDistanceToNow(new Date(conversation.created_at), { addSuffix: true })}
                      </p>
                    </div>
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        onDeleteConversation(conversation.id)
                      }}
                      className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-red-500/20 text-red-400 transition-opacity"
                      title="Delete conversation"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </motion.div>
              )
            })
          )}
        </div>
      </div>
    </div>
  )
}