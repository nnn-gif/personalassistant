export interface Activity {
  id: string
  app_usage: AppUsage
  project_context?: ProjectContext
  timestamp: string
  duration_seconds: number
  goal_id?: string
  goal_name?: string
  system_state?: SystemState
  input_metrics?: InputMetrics
}

export interface AppUsage {
  app_name: string
  window_title: string
  category?: string
  is_productive?: boolean
}

export interface ProjectContext {
  project_name: string
  project_path: string
  language: string
  framework: string
  git_branch?: string
}

export interface AudioDevice {
  id: string
  name: string
  is_default: boolean
  device_type: 'input' | 'output'
}

export interface RecordingInfo {
  recording_id: string
  device_id: string
  device_name: string
  start_time: string
  duration_seconds: number
  status: 'recording' | 'paused' | 'stopped'
}

export interface Recording {
  id: string
  file_path: string
  device_name: string
  duration_seconds: number
  created_at: string
  transcription?: string
  summary?: string
}

export interface Goal {
  id: string
  name: string
  description: string
  target_hours: number
  target_apps?: string[]
  deadline?: string
  created_at: string
  updated_at: string
  is_active: boolean
  total_seconds_spent: number
  progress_percentage: number
}

export interface GoalProgress {
  goal_id: string
  goal_name: string
  target_hours: number
  total_seconds_spent: number
  progress_percentage: number
  app_breakdown: Record<string, number>
  daily_progress: DailyProgress[]
}

export interface DailyProgress {
  date: string
  seconds_spent: number
}

export interface Document {
  id: string
  file_path: string
  file_name: string
  file_type: string
  file_size: number
  indexed_at: string
  chunk_count: number
  metadata?: Record<string, unknown>
}

export interface SearchResult {
  document_id: string
  file_path: string
  score: number
  content: string
  metadata?: Record<string, unknown>
}

export interface ChatConversation {
  id: string
  title: string
  mode: 'general' | 'documents' | 'research'
  created_at: string
  updated_at: string
  message_count: number
}

export interface ChatMessage {
  id: string
  conversation_id: string
  role: 'user' | 'assistant'
  content: string
  created_at: string
  metadata?: Record<string, unknown>
}

export interface ResearchTask {
  id: string
  query: string
  status: 'pending' | 'running' | 'completed' | 'failed'
  progress: number
  current_step?: string
  subtasks: ResearchSubtask[]
  results: ResearchResult[]
  created_at: string
  updated_at: string
}

export interface ResearchSubtask {
  id: string
  description: string
  status: 'pending' | 'running' | 'completed' | 'failed'
  result?: string
}

export interface ResearchResult {
  source: string
  title: string
  content: string
  relevance_score: number
  url?: string
}

export interface ProductivityStats {
  total_tracked_seconds: number
  active_time_seconds: number
  productivity_score: number
  top_apps: Array<{
    app_name: string
    total_seconds: number
    percentage: number
  }>
  hourly_breakdown: Array<{
    hour: number
    seconds: number
  }>
}

export interface ProductivityInsights {
  insights: string[]
  productivity_score: number
  recommendations: string[]
  peak_hours: number[]
  distraction_patterns: string[]
}

export interface IndexingProgress {
  current: number
  total: number
  file_name: string
  status: string
}

export interface RAGDatabaseInfo {
  status: string
  total_documents: number
  total_chunks: number
  documents: Array<{
    id: string
    file_path: string
    chunk_count: number
    indexed_at: string
  }>
}

export interface CleanupResult {
  removed_count: number
  remaining_count: number
  errors: string[]
}

export interface ClearDatabaseResult {
  success: boolean
  message: string
}

export interface SystemState {
  cpu_usage_percent: number
  idle_time_seconds: number
}

export interface InputMetrics {
  keystrokes: number
  mouse_clicks: number
}