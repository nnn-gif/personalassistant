import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts'

// Mock data - in real app would fetch from backend
const data = [
  { time: '8 AM', score: 65 },
  { time: '9 AM', score: 78 },
  { time: '10 AM', score: 85 },
  { time: '11 AM', score: 82 },
  { time: '12 PM', score: 45 },
  { time: '1 PM', score: 52 },
  { time: '2 PM', score: 73 },
  { time: '3 PM', score: 80 },
  { time: '4 PM', score: 76 },
  { time: '5 PM', score: 68 },
]

export default function ProductivityChart() {
  return (
    <div className="h-64">
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
    </div>
  )
}