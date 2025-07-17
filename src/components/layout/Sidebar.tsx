import { motion } from 'framer-motion'
import { 
  LayoutDashboard, 
  Target, 
  Activity,
  Brain,
  Mic,
  FileText,
  MessageCircle,
  Settings
} from 'lucide-react'

interface SidebarProps {
  currentView: string
  onViewChange: (view: string) => void
}

const menuItems = [
  { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
  { id: 'chat', label: 'Assistant Chat', icon: MessageCircle },
  { id: 'goals', label: 'Goals', icon: Target },
  { id: 'activity', label: 'Activity Monitor', icon: Activity },
  { id: 'audio', label: 'Audio Recorder', icon: Mic },
  { id: 'documents', label: 'Document Manager', icon: FileText },
  { id: 'settings', label: 'Settings', icon: Settings },
]

export default function Sidebar({ currentView, onViewChange }: SidebarProps) {
  return (
    <aside className="w-64 bg-dark-surface border-r border-dark-border">
      <div className="p-6">
        <div className="flex items-center space-x-3">
          <Brain className="w-8 h-8 text-primary" />
          <h1 className="text-xl font-bold">Personal Assistant</h1>
        </div>
      </div>
      
      <nav className="px-4 pb-4">
        {menuItems.map((item) => {
          const Icon = item.icon
          const isActive = currentView === item.id
          
          return (
            <motion.button
              key={item.id}
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              onClick={() => onViewChange(item.id)}
              className={`
                w-full flex items-center space-x-3 px-4 py-3 rounded-lg mb-2
                transition-colors duration-200
                ${isActive 
                  ? 'bg-primary/20 text-primary' 
                  : 'text-gray-400 hover:bg-dark-bg hover:text-white'
                }
              `}
            >
              <Icon className="w-5 h-5" />
              <span className="font-medium">{item.label}</span>
            </motion.button>
          )
        })}
      </nav>
    </aside>
  )
}