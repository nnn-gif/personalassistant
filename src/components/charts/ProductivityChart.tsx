import { useEffect, useState } from 'react'
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts'
import { invoke } from '@tauri-apps/api/core'

interface ProductivityTrend {
  hour: number
  productive_minutes: number
  total_minutes: number
  productivity_percentage: number
}

export default function ProductivityChart() {
  const [data, setData] = useState<any[]>([])
  
  useEffect(() => {
    loadProductivityTrend()
    const interval = setInterval(loadProductivityTrend, 60000) // Update every minute
    return () => clearInterval(interval)
  }, [])
  
  const loadProductivityTrend = async () => {
    try {
      const trend = await invoke<ProductivityTrend[]>('get_productivity_trend', { hours: 8 })
      const chartData = trend.map(item => ({
        time: `${24 - item.hour}h ago`,
        score: Math.round(item.productivity_percentage)
      }))
      setData(chartData)
    } catch (error) {
      console.error('Failed to load productivity trend:', error)
    }
  }
  return (
    <div className="h-64">
      {data.length === 0 ? (
        <div className="flex items-center justify-center h-full text-gray-400">
          Start tracking to see productivity trends
        </div>
      ) : (
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#3a3a3a" />
            <XAxis 
              dataKey="time" 
              stroke="#6b7280"
              style={{ fontSize: '12px' }}
            />
            <YAxis 
              stroke="#6b7280"
              style={{ fontSize: '12px' }}
            />
            <Tooltip 
              contentStyle={{ 
                backgroundColor: '#2a2a2a', 
                border: '1px solid #3a3a3a',
                borderRadius: '8px'
              }}
            />
            <Line 
              type="monotone" 
              dataKey="score" 
              stroke="#3b82f6" 
              strokeWidth={2}
              dot={{ fill: '#3b82f6', r: 4 }}
              activeDot={{ r: 6 }}
            />
          </LineChart>
        </ResponsiveContainer>
      )}
    </div>
  )
}