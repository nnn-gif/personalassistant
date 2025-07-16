import { MessageSquare, BookOpen, Search } from 'lucide-react'

export type ChatMode = 'general' | 'knowledge' | 'research'

interface ModeSelectorProps {
  currentMode: ChatMode
  onModeChange: (mode: ChatMode) => void
}

export const modeConfig = {
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

export default function ModeSelector({ currentMode, onModeChange }: ModeSelectorProps) {
  return (
    <div className="flex space-x-2 p-4 border-b border-dark-border">
      {(Object.keys(modeConfig) as ChatMode[]).map((mode) => {
        const IconComponent = modeConfig[mode].icon
        const isActive = currentMode === mode
        return (
          <button
            key={mode}
            onClick={() => onModeChange(mode)}
            className={`flex items-center space-x-2 px-3 py-2 rounded-md text-sm font-medium transition-all ${
              isActive
                ? 'bg-primary text-white'
                : 'text-gray-400 hover:text-white hover:bg-dark-border'
            }`}
          >
            <IconComponent className="w-4 h-4" />
            <span>{modeConfig[mode].title}</span>
          </button>
        )
      })}
    </div>
  )
}