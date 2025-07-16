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
}

function StatCard({ title, value, icon: Icon, color }: StatCardProps) {
  return (
    <motion.div
      whileHover={{ scale: 1.02 }}
      className="card"
    >
      <div className="flex items-center justify-between">
        <div>
          <p className="text-gray-400 text-sm">{title}</p>
          <p className="text-2xl font-bold mt-1">{value}</p>
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
  const [activeGoals, setActiveGoals] = useState(0)
  const [todayHours, setTodayHours] = useState(0)
  const [insights, setInsights] = useState<string[]>([])

  useEffect(() => {
    loadDashboardData()
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
        setActiveGoals(goals.filter(g => g.is_active).length)
      } catch (error) {
        console.error('Failed to get goals:', error)
        setActiveGoals(0)
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
        const todayStats = await invoke<ProductivityStats>('get_today_stats')
        console.log('Today stats:', todayStats)
        const hours = todayStats.total_tracked_seconds / 3600
        setTodayHours(hours)
        console.log('Today\'s hours:', hours)
      } catch (error) {
        console.error('Failed to get today stats:', error)
        setTodayHours(0)
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
          title="Active Goals"
          value={activeGoals}
          icon={Target}
          color="bg-success/20 text-success"
        />
        <StatCard
          title="Today's Hours"
          value={formatDecimalHours(todayHours)}
          icon={Clock}
          color="bg-warning/20 text-warning"
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