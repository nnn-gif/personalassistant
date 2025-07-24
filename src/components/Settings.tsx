import { useState } from 'react'
import { motion } from 'framer-motion'
import { Settings as SettingsIcon, Cpu, Database, Activity, ChevronRight } from 'lucide-react'
import InferenceSettings from './settings/InferenceSettings'

type SettingsTab = 'inference' | 'tracking' | 'database' | 'general'

interface TabConfig {
  id: SettingsTab
  label: string
  icon: React.ComponentType<{ className?: string }>
  description: string
}

const tabs: TabConfig[] = [
  {
    id: 'inference',
    label: 'Inference',
    icon: Cpu,
    description: 'Configure AI model providers and settings'
  },
  {
    id: 'tracking',
    label: 'Activity Tracking',
    icon: Activity,
    description: 'Manage activity tracking preferences'
  },
  {
    id: 'database',
    label: 'Database',
    icon: Database,
    description: 'Database and storage settings'
  },
  {
    id: 'general',
    label: 'General',
    icon: SettingsIcon,
    description: 'General application settings'
  }
]

export default function Settings() {
  const [activeTab, setActiveTab] = useState<SettingsTab>('inference')

  const renderContent = () => {
    switch (activeTab) {
      case 'inference':
        return <InferenceSettings />
      case 'tracking':
        return (
          <div className="card">
            <h3 className="text-xl font-semibold mb-4">Activity Tracking Settings</h3>
            <p className="text-gray-400">Activity tracking settings coming soon...</p>
          </div>
        )
      case 'database':
        return (
          <div className="card">
            <h3 className="text-xl font-semibold mb-4">Database Settings</h3>
            <p className="text-gray-400">Database settings coming soon...</p>
          </div>
        )
      case 'general':
        return (
          <div className="card">
            <h3 className="text-xl font-semibold mb-4">General Settings</h3>
            <p className="text-gray-400">General settings coming soon...</p>
          </div>
        )
      default:
        return null
    }
  }

  return (
    <div className="container mx-auto p-6">
      <div className="flex items-center space-x-3 mb-8">
        <SettingsIcon className="w-8 h-8 text-primary" />
        <h1 className="text-3xl font-bold">Settings</h1>
      </div>

      <div className="grid grid-cols-4 gap-6">
        {/* Sidebar */}
        <div className="col-span-1">
          <div className="space-y-2">
            {tabs.map((tab) => {
              const Icon = tab.icon
              return (
                <motion.button
                  key={tab.id}
                  whileTap={{ scale: 0.98 }}
                  onClick={() => setActiveTab(tab.id)}
                  className={`w-full text-left p-4 rounded-lg transition-all ${
                    activeTab === tab.id
                      ? 'bg-primary/10 border-l-4 border-primary'
                      : 'hover:bg-dark-bg'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-3">
                      <Icon className={`w-5 h-5 ${
                        activeTab === tab.id ? 'text-primary' : 'text-gray-400'
                      }`} />
                      <div>
                        <p className={`font-medium ${
                          activeTab === tab.id ? 'text-white' : 'text-gray-300'
                        }`}>
                          {tab.label}
                        </p>
                        <p className="text-xs text-gray-500 mt-1">
                          {tab.description}
                        </p>
                      </div>
                    </div>
                    {activeTab === tab.id && (
                      <ChevronRight className="w-4 h-4 text-primary" />
                    )}
                  </div>
                </motion.button>
              )
            })}
          </div>
        </div>

        {/* Content Area */}
        <div className="col-span-3">
          {renderContent()}
        </div>
      </div>
    </div>
  )
}