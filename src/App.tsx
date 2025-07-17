import { useState } from 'react'
import { motion } from 'framer-motion'
import Sidebar from './components/layout/Sidebar'
import Dashboard from './components/Dashboard'
import GoalsManager from './components/GoalsManager'
import ActivityMonitor from './components/ActivityMonitor'
import AudioRecorder from './components/AudioRecorder'
import DocumentManager from './components/DocumentManager'
import UnifiedChat from './components/UnifiedChat'
import Settings from './components/Settings'

type View = 'dashboard' | 'goals' | 'activity' | 'audio' | 'documents' | 'chat' | 'settings'

function App() {
  const [currentView, setCurrentView] = useState<View>('dashboard')

  const renderView = () => {
    switch (currentView) {
      case 'dashboard':
        return <Dashboard />
      case 'goals':
        return <GoalsManager />
      case 'activity':
        return <ActivityMonitor />
      case 'audio':
        return <AudioRecorder />
      case 'documents':
        return <DocumentManager />
      case 'chat':
        return <UnifiedChat />
      case 'settings':
        return <Settings />
      default:
        return <Dashboard />
    }
  }

  return (
    <div className="flex h-screen bg-dark-bg">
      <Sidebar currentView={currentView} onViewChange={(view) => setCurrentView(view as View)} />
      
      <main className="flex-1 overflow-y-auto">
        <motion.div
          key={currentView}
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -20 }}
          transition={{ duration: 0.3 }}
          className="p-8"
        >
          {renderView()}
        </motion.div>
      </main>
    </div>
  )
}

export default App