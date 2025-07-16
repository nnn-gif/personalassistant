import React from 'react'
import { Send, Loader2 } from 'lucide-react'
import { ChatMode, modeConfig } from './ModeSelector'

interface ChatInputProps {
  inputMessage: string
  isLoading: boolean
  currentMode: ChatMode
  selectedGoal: string
  selectedModel: string
  goals: Goal[]
  availableModels: string[]
  onInputChange: (value: string) => void
  onSendMessage: () => void
  onKeyPress: (e: React.KeyboardEvent) => void
  onGoalChange: (goalId: string) => void
  onModelChange: (model: string) => void
}

interface Goal {
  id: string
  name: string
}

export default function ChatInput({
  inputMessage,
  isLoading,
  currentMode,
  selectedGoal,
  selectedModel,
  goals,
  availableModels,
  onInputChange,
  onSendMessage,
  onKeyPress,
  onGoalChange,
  onModelChange
}: ChatInputProps) {
  return (
    <div className="p-4 border-t border-dark-border">
      <div className="flex items-center space-x-2 mb-2">
        <select
          value={selectedGoal}
          onChange={(e) => onGoalChange(e.target.value)}
          className="px-2 py-1 text-sm bg-dark-bg border border-dark-border rounded text-gray-300 focus:outline-none focus:border-primary"
        >
          <option value="">Select Goal</option>
          {goals.map(goal => (
            <option key={goal.id} value={goal.id}>{goal.name}</option>
          ))}
        </select>
        
        <select
          value={selectedModel}
          onChange={(e) => onModelChange(e.target.value)}
          className="px-2 py-1 text-sm bg-dark-bg border border-dark-border rounded text-gray-300 focus:outline-none focus:border-primary"
        >
          {availableModels.map(model => (
            <option key={model} value={model}>{model}</option>
          ))}
        </select>
      </div>

      <div className="flex items-end space-x-2">
        <textarea
          value={inputMessage}
          onChange={(e) => onInputChange(e.target.value)}
          onKeyPress={onKeyPress}
          placeholder={modeConfig[currentMode].placeholder}
          className="flex-1 px-4 py-2 bg-dark-bg border border-dark-border rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-primary resize-none"
          rows={3}
          disabled={isLoading}
        />
        <button
          onClick={onSendMessage}
          disabled={!inputMessage.trim() || isLoading}
          className="px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary-dark disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center space-x-2"
        >
          {isLoading ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              <span>Processing...</span>
            </>
          ) : (
            <>
              <Send className="w-4 h-4" />
              <span>Send</span>
            </>
          )}
        </button>
      </div>
    </div>
  )
}