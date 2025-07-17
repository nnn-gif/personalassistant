import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { TrendingUp, Clock, Target, Brain } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import ProductivityChart from './charts/ProductivityChart'
import ActivityTimeline from './charts/ActivityTimeline'
import { formatDecimalHours } from '../lib/timeUtils'
import { Goal, ProductivityStats, ProductivityInsights } from '../types'

interface ProductivityScore {
  overall: number
  focus: number
  efficiency: number
  breaks: number
}

interface StatCardProps {
  title: string
  value: string | number
  icon: React.ElementType
  color: string
  isLoading?: boolean
}

function StatCard({ title, value, icon: Icon, color, isLoading = false }: StatCardProps) {
  return (
    <motion.div
      whileHover={{ scale: 1.02 }}
      className="card"
    >
      <div className="flex items-center justify-between">
        <div>
          <p className="text-gray-400 text-sm">{title}</p>
          {isLoading ? (
            <div className="mt-1 flex items-center">
              <motion.div
                className="w-2 h-2 bg-primary rounded-full"
                animate={{ opacity: [0.3, 1, 0.3] }}
                transition={{ duration: 1.5, repeat: Infinity }}
              />
              <motion.div
                className="w-2 h-2 bg-primary rounded-full ml-1"
                animate={{ opacity: [0.3, 1, 0.3] }}
                transition={{ duration: 1.5, repeat: Infinity, delay: 0.3 }}
              />
              <motion.div
                className="w-2 h-2 bg-primary rounded-full ml-1"
                animate={{ opacity: [0.3, 1, 0.3] }}
                transition={{ duration: 1.5, repeat: Infinity, delay: 0.6 }}
              />
            </div>
          ) : (
            <p className="text-2xl font-bold mt-1">{value}</p>
          )}
        </div>
        <div className={`p-3 rounded-lg ${color}`}>
          <Icon className="w-6 h-6" />
        </div>
      </div>
    </motion.div>
  )
}

export default function Dashboard() {
  const [productivityScore, setProductivityScore] = useState<ProductivityScore | null>(null)
  const [activeGoalName, setActiveGoalName] = useState<string>('')
  const [todayHours, setTodayHours] = useState(0)
  const [hoursLoading, setHoursLoading] = useState(true)
  const [insights, setInsights] = useState<string[]>([])

  useEffect(() => {
    loadDashboardData()
    
    // Fallback to prevent infinite loading
    const fallbackTimer = setTimeout(() => {
      if (hoursLoading) {
        console.error('[Dashboard] Fallback triggered - setting loading to false after 10 seconds')
        setHoursLoading(false)
        setTodayHours(0)
      }
    }, 10000)
    
    return () => clearTimeout(fallbackTimer)
  }, [])

  const loadDashboardData = async () => {
    try {
      console.log('Loading dashboard data...')
      
      // Debug: Check tracking stats first
      const trackingStats = await invoke<ProductivityStats>('get_tracking_stats')
      console.log('Tracking stats:', trackingStats)

      // Get productivity score
      try {
        const score = await invoke<ProductivityScore>('get_productivity_score', { hours: 8 })
        console.log('Productivity score:', score)
        setProductivityScore(score)
      } catch (error) {
        console.error('Failed to get productivity score:', error)
        // Fallback to basic calculation
        const currentScore = await invoke<number>('get_current_productivity_score')
        setProductivityScore({
          overall: currentScore,
          focus: currentScore * 0.9,
          efficiency: currentScore * 0.8,
          breaks: currentScore * 0.7
        })
      }

      // Get goals
      try {
        const goals = await invoke<Goal[]>('get_goals')
        console.log('Goals:', goals)
        const activeGoal = goals.find(g => g.is_active)
        if (activeGoal) {
          setActiveGoalName(activeGoal.name)
        } else {
          setActiveGoalName('No active goal')
        }
      } catch (error) {
        console.error('Failed to get goals:', error)
        setActiveGoalName('No goals')
      }

      // Get productivity insights
      try {
        const insightsData = await invoke<ProductivityInsights>('get_productivity_insights', { hours: 8 })
        console.log('Insights:', insightsData)
        setInsights(insightsData.insights || [])
      } catch (error) {
        console.error('Failed to get insights:', error)
        setInsights(['Start tracking to see insights'])
      }

      // Get today's hours from database
      try {
        console.log('[Dashboard] Fetching today stats...')
        setHoursLoading(true)
        
        // Add timeout to prevent infinite loading
        const timeoutPromise = new Promise<never>((_, reject) => 
          setTimeout(() => reject(new Error('Timeout fetching today stats')), 5000)
        )
        
        const todayStats = await Promise.race([
          invoke<ProductivityStats>('get_today_stats'),
          timeoutPromise
        ])
        console.log('[Dashboard] Today stats received:', JSON.stringify(todayStats, null, 2))
        
        if (!todayStats) {
          console.error('[Dashboard] todayStats is null or undefined')
          setTodayHours(0)
        } else if (typeof todayStats.total_tracked_seconds !== 'number') {
          console.error('[Dashboard] Invalid total_tracked_seconds:', todayStats.total_tracked_seconds, 'type:', typeof todayStats.total_tracked_seconds)
          console.error('[Dashboard] Full todayStats object:', todayStats)
          setTodayHours(0)
        } else {
          const hours = todayStats.total_tracked_seconds / 3600
          setTodayHours(hours)
          console.log(`[Dashboard] Successfully calculated hours: ${hours.toFixed(2)} hours (${todayStats.total_tracked_seconds} seconds)`)
          
          // Log additional stats
          console.log(`[Dashboard] Productivity score: ${todayStats.productivity_score}%`)
          console.log(`[Dashboard] Active time: ${todayStats.active_time_seconds} seconds`)
          console.log(`[Dashboard] Top apps:`, todayStats.top_apps)
          
          if (hours === 0 || isNaN(hours)) {
            console.warn(`[Dashboard] Today's Hours has ${hours === 0 ? '0' : 'nan'}`)
          }
        }
      } catch (error) {
        console.error('[Dashboard] Failed to get today stats:', error)
        console.error('[Dashboard] Error details:', error instanceof Error ? error.message : 'Unknown error')
        
        // Show error message to user
        if (error instanceof Error && error.message.includes('Timeout')) {
          console.error('[Dashboard] Request timed out after 5 seconds')
        }
        
        setTodayHours(0)
      } finally {
        setHoursLoading(false)
        console.log('[Dashboard] Hours loading completed, loading state set to false')
      }
    } catch (error) {
      console.error('Failed to load dashboard data:', error)
    }
  }

  return (
    <div className="space-y-6">
      <header>
        <h2 className="text-3xl font-bold">Dashboard</h2>
        <p className="text-gray-400 mt-2">Your productivity at a glance</p>
      </header>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <StatCard
          title="Productivity Score"
          value={productivityScore?.overall.toFixed(0) || 0}
          icon={TrendingUp}
          color="bg-primary/20 text-primary"
        />
        <StatCard
          title="Focus Score"
          value={productivityScore?.focus.toFixed(0) || 0}
          icon={Brain}
          color="bg-secondary/20 text-secondary"
        />
        <StatCard
          title="Current Goal"
          value={activeGoalName}
          icon={Target}
          color="bg-success/20 text-success"
        />
        <StatCard
          title="Today's Hours"
          value={hoursLoading ? '' : formatDecimalHours(todayHours)}
          icon={Clock}
          color="bg-warning/20 text-warning"
          isLoading={hoursLoading}
        />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="card">
          <h3 className="text-xl font-semibold mb-4">Productivity Trend</h3>
          <ProductivityChart />
        </div>
        
        <div className="card">
          <h3 className="text-xl font-semibold mb-4">Activity Timeline</h3>
          <ActivityTimeline />
        </div>
      </div>

      {insights.length > 0 && (
        <div className="card">
          <h3 className="text-xl font-semibold mb-4">AI Insights</h3>
          <ul className="space-y-2">
            {insights.map((insight, index) => (
              <motion.li
                key={index}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: index * 0.1 }}
                className="flex items-start space-x-2"
              >
                <span className="text-primary mt-1">•</span>
                <span className="text-gray-300">{insight}</span>
              </motion.li>
            ))}
          </ul>
        </div>
      )}
    </div>
  )
}