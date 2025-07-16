import { useEffect, useState } from 'react'
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Cell } from 'recharts'
import { invoke } from '@tauri-apps/api/core'

interface AppUsageData {
  app_name: string
  total_minutes: number
  is_productive: boolean
}

interface ChartData {
  app: string
  minutes: number
  productive: boolean
}

export default function ActivityTimeline() {
  const [data, setData] = useState<ChartData[]>([])
  
  useEffect(() => {
    loadAppUsageStats()
    const interval = setInterval(loadAppUsageStats, 60000) // Update every minute
    return () => clearInterval(interval)
  }, [])
  
  const loadAppUsageStats = async () => {
    try {
      const stats = await invoke<AppUsageData[]>('get_app_usage_stats', { hours: 8 })
      const chartData = stats
        .slice(0, 6) // Top 6 apps
        .map(item => ({
          app: item.app_name,
          minutes: item.total_minutes,
          productive: item.is_productive
        }))
      setData(chartData)
    } catch (error) {
      console.error('Failed to load app usage stats:', error)
    }
  }
  return (
    <div className="h-64">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={data} layout="horizontal">
          <CartesianGrid strokeDasharray="3 3" stroke="#3a3a3a" />
          <XAxis 
            type="number"
            stroke="#6b7280"
            style={{ fontSize: '12px' }}
          />
          <YAxis 
            type="category"
            dataKey="app"
            stroke="#6b7280"
            style={{ fontSize: '12px' }}
            width={80}
          />
          <Tooltip 
            contentStyle={{ 
              backgroundColor: '#2a2a2a', 
              border: '1px solid #3a3a3a',
              borderRadius: '8px'
            }}
          />
          <Bar 
            dataKey="minutes" 
            radius={[0, 4, 4, 0]}
          >
            {data.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={entry.productive ? '#3b82f6' : '#6b7280'} />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}