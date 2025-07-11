import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts'

// Mock data - in real app would fetch from backend
const data = [
  { app: 'VS Code', minutes: 120, productive: true },
  { app: 'Chrome', minutes: 85, productive: true },
  { app: 'Slack', minutes: 45, productive: true },
  { app: 'Terminal', minutes: 38, productive: true },
  { app: 'Spotify', minutes: 25, productive: false },
  { app: 'Twitter', minutes: 15, productive: false },
]

export default function ActivityTimeline() {
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
            fill={(entry: any) => entry.productive ? '#3b82f6' : '#6b7280'}
            radius={[0, 4, 4, 0]}
          />
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}