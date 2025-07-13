import { useState, useEffect, useRef } from 'react'
import { motion } from 'framer-motion'
import { Mic, Pause, Play, Square, Download, Loader2, FileAudio, Trash2, PlayCircle, PauseCircle } from 'lucide-react'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'

interface AudioDevice {
  name: string
  device_type: string
}

interface Recording {
  id: string
  title: string
  started_at: string
  ended_at: string
  duration_seconds: number
  file_path: string
  transcription?: string
  meeting_info?: {
    app_name: string
    meeting_title?: string
    participants: string[]
    meeting_url?: string
  }
  file_size_bytes: number
  sample_rate: number
  channels: number
}

interface RecordingStatus {
  status: 'Idle' | 'Recording' | 'Paused' | 'Processing' | 'Failed'
  recording_info?: {
    id: string
    started_at: string
    duration_seconds: number
    method: string
    devices: string[]
  }
  error?: string
}

export default function AudioRecorder() {
  const [devices, setDevices] = useState<AudioDevice[]>([])
  const [selectedDevices, setSelectedDevices] = useState<string[]>([])
  const [isRecording, setIsRecording] = useState(false)
  const [isPaused, setIsPaused] = useState(false)
  const [recordings, setRecordings] = useState<Recording[]>([])
  const [status, setStatus] = useState<RecordingStatus>({ status: 'Idle' })
  const [duration, setDuration] = useState(0)
  const [transcribing, setTranscribing] = useState<string | null>(null)
  const [playingId, setPlayingId] = useState<string | null>(null)
  const [deletingId, setDeletingId] = useState<string | null>(null)
  const audioRef = useRef<HTMLAudioElement | null>(null)
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  useEffect(() => {
    loadDevices()
    loadRecordings()
    
    // Cleanup audio on unmount
    return () => {
      if (audioRef.current) {
        audioRef.current.pause()
        audioRef.current = null
      }
    }
  }, [])

  useEffect(() => {
    if (isRecording && !isPaused) {
      intervalRef.current = setInterval(() => {
        setDuration(d => d + 1)
      }, 1000)
    } else {
      if (intervalRef.current) {
        clearInterval(intervalRef.current)
        intervalRef.current = null
      }
    }

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current)
      }
    }
  }, [isRecording, isPaused])

  const loadDevices = async () => {
    try {
      const deviceList = await invoke<AudioDevice[]>('list_audio_devices')
      setDevices(deviceList)
          // Auto-select the first device if available
      if (deviceList.length > 0 && selectedDevices.length === 0) {
        setSelectedDevices([deviceList[0].name])
      }
    } catch (error) {
      console.error('Failed to load audio devices:', error)
    }
  }

  const loadRecordings = async () => {
    try {
      const recs = await invoke<Recording[]>('get_recordings')
      setRecordings(recs)
    } catch (error) {
      console.error('Failed to load recordings:', error)
    }
  }

  const startRecording = async () => {
    console.log('startRecording called');
    console.log('Selected devices:', selectedDevices);
    console.log('Is recording:', isRecording);
    
    try {
      const title = `Meeting Recording ${new Date().toLocaleString()}`
      const result = await invoke('start_audio_recording', {
        devices: selectedDevices,
        title
      })
      console.log('Recording started:', result)
      setIsRecording(true)
      setDuration(0)
      
      // Start polling for status
      pollStatus()
    } catch (error) {
      console.error('Failed to start recording:', error)
      alert('Failed to start recording: ' + error)
    }
  }

  const stopRecording = async () => {
    try {
      const recording = await invoke<Recording>('stop_audio_recording')
      setIsRecording(false)
      setIsPaused(false)
      setDuration(0)
      
      await loadRecordings()
      
      // Auto-transcribe
      if (recording.id) {
        transcribeRecording(recording)
      }
    } catch (error) {
      console.error('Failed to stop recording:', error)
      alert('Failed to stop recording: ' + error)
    }
  }

  const pauseRecording = async () => {
    try {
      if (isPaused) {
        await invoke('resume_audio_recording')
        setIsPaused(false)
      } else {
        await invoke('pause_audio_recording')
        setIsPaused(true)
      }
    } catch (error) {
      console.error('Failed to pause/resume recording:', error)
    }
  }

  const pollStatus = async () => {
    try {
      const statusJson = await invoke<string>('get_recording_status')
      const newStatus = JSON.parse(statusJson) as RecordingStatus
      setStatus(newStatus)
      
      if (isRecording) {
        setTimeout(pollStatus, 1000)
      }
    } catch (error) {
      console.error('Failed to get status:', error)
    }
  }

  const transcribeRecording = async (recording: Recording) => {
    setTranscribing(recording.id)
    try {
      const result = await invoke('transcribe_recording', {
        recordingId: recording.id,
        recordingPath: recording.file_path
      })
      
      // Generate summary
      const transcriptionResult = result as any
      if (transcriptionResult?.text) {
        const summary = await invoke('generate_meeting_summary', {
          transcription: transcriptionResult.text
        })
        console.log('Meeting summary:', summary)
      }
      
      await loadRecordings()
    } catch (error) {
      console.error('Failed to transcribe:', error)
    } finally {
      setTranscribing(null)
    }
  }

  const deleteRecording = async (id: string) => {
    console.log('Delete button clicked for recording:', id)
    setDeletingId(id)
  }

  const confirmDelete = async () => {
    if (!deletingId) return
    
    try {
      console.log('Invoking delete_recording command...')
      await invoke('delete_recording', { recordingId: deletingId })
      console.log('Delete successful, reloading recordings...')
      await loadRecordings()
      setDeletingId(null)
    } catch (error) {
      console.error('Failed to delete recording:', error)
      alert('Failed to delete recording: ' + error)
      setDeletingId(null)
    }
  }

  const playRecording = async (recording: Recording) => {
    if (playingId === recording.id) {
      // Stop playing
      if (audioRef.current) {
        audioRef.current.pause()
        audioRef.current = null
      }
      setPlayingId(null)
    } else {
      // Start playing
      if (audioRef.current) {
        audioRef.current.pause()
      }
      
      try {
        // Convert the file path to a URL that can be played
        const audioUrl = convertFileSrc(recording.file_path)
        console.log('Audio URL:', audioUrl)
        
        // Try to fetch the audio file and create a blob URL
        const response = await fetch(audioUrl)
        if (!response.ok) {
          throw new Error(`Failed to fetch audio: ${response.status}`)
        }
        
        const blob = await response.blob()
        const blobUrl = URL.createObjectURL(blob)
        
        const audio = new Audio(blobUrl)
        audio.onended = () => {
          setPlayingId(null)
          audioRef.current = null
          URL.revokeObjectURL(blobUrl)
        }
        audio.onerror = (e) => {
          console.error('Failed to play audio:', e)
          alert('Failed to play recording')
          setPlayingId(null)
          audioRef.current = null
          URL.revokeObjectURL(blobUrl)
        }
        
        audioRef.current = audio
        await audio.play()
        setPlayingId(recording.id)
      } catch (error) {
        console.error('Failed to play audio:', error)
        alert(`Failed to play recording: ${error instanceof Error ? error.message : String(error)}`)
      }
    }
  }

  const formatDuration = (seconds: number) => {
    const hrs = Math.floor(seconds / 3600)
    const mins = Math.floor((seconds % 3600) / 60)
    const secs = seconds % 60
    
    if (hrs > 0) {
      return `${hrs}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`
    }
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return bytes + ' B'
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
  }

  return (
    <div className="space-y-6">
      <header>
        <h2 className="text-3xl font-bold">Audio Recorder</h2>
        <p className="text-gray-400 mt-2">Record and transcribe meeting calls</p>
        {/* Debug info */}
        <p className="text-xs text-gray-500 mt-1">
          Devices: {devices.length}, Selected: {selectedDevices.length}, 
          Recording: {isRecording ? 'Yes' : 'No'}
        </p>
      </header>

      {/* Recording Controls */}
      <div className="card">
        <h3 className="text-xl font-semibold mb-4">Recording Controls</h3>
        
        {/* Device Selection */}
        <div className="mb-6">
          <label className="block text-sm font-medium mb-2">Audio Devices</label>
          <div className="space-y-2">
            {devices.map((device) => (
              <label key={device.name} className="flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={selectedDevices.includes(device.name)}
                  onChange={(e) => {
                    if (e.target.checked) {
                      setSelectedDevices([...selectedDevices, device.name])
                    } else {
                      setSelectedDevices(selectedDevices.filter(d => d !== device.name))
                    }
                  }}
                  className="checkbox"
                  disabled={isRecording}
                />
                <span>{device.name} ({device.device_type})</span>
              </label>
            ))}
          </div>
        </div>

        {/* Recording Status */}
        {isRecording && (
          <div className="mb-6 p-4 bg-primary/10 rounded-lg">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-3">
                <div className={`w-3 h-3 rounded-full ${isPaused ? 'bg-yellow-500' : 'bg-red-500 animate-pulse'}`} />
                <span className="font-medium">
                  {isPaused ? 'Paused' : 'Recording'}
                </span>
                <span className="text-gray-400">
                  {formatDuration(duration)}
                </span>
              </div>
              <div className="text-sm text-gray-400">
                Method: {status.recording_info?.method || 'Unknown'}
              </div>
            </div>
          </div>
        )}

        {/* Control Buttons */}
        <div className="flex items-center space-x-4">
          {!isRecording ? (
            <button
              onClick={() => {
                console.log('Button clicked!');
                startRecording();
              }}
              disabled={selectedDevices.length === 0}
              className="btn-primary flex items-center space-x-2"
            >
              <Mic className="w-5 h-5" />
              <span>Start Recording</span>
            </button>
          ) : (
            <>
              <button
                onClick={stopRecording}
                className="btn-danger flex items-center space-x-2"
              >
                <Square className="w-5 h-5" />
                <span>Stop</span>
              </button>
              <button
                onClick={pauseRecording}
                className="btn-secondary flex items-center space-x-2"
              >
                {isPaused ? (
                  <Play className="w-5 h-5" />
                ) : (
                  <Pause className="w-5 h-5" />
                )}
                <span>{isPaused ? 'Resume' : 'Pause'}</span>
              </button>
            </>
          )}
        </div>
      </div>

      {/* Recordings List */}
      <div className="card">
        <h3 className="text-xl font-semibold mb-4">Recordings</h3>
        
        {recordings.length === 0 ? (
          <p className="text-gray-400 text-center py-8">No recordings yet</p>
        ) : (
          <div className="space-y-4">
            {recordings.map((recording) => (
              <motion.div
                key={recording.id}
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                className="p-4 bg-dark-bg rounded-lg"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <h4 className="font-medium flex items-center space-x-2">
                      <FileAudio className="w-4 h-4 text-primary" />
                      <span>{recording.title}</span>
                    </h4>
                    <div className="mt-2 text-sm text-gray-400 space-y-1">
                      <p>Duration: {formatDuration(Math.round(recording.duration_seconds))}</p>
                      <p>Size: {formatFileSize(recording.file_size_bytes)}</p>
                      <p>Quality: {recording.sample_rate}Hz, {recording.channels}ch</p>
                      <p>Date: {new Date(recording.started_at).toLocaleString()}</p>
                    </div>
                    
                    {recording.transcription && (
                      <div className="mt-3 p-3 bg-dark-card rounded">
                        <p className="text-sm font-medium mb-1">Transcription:</p>
                        <p className="text-sm text-gray-300 line-clamp-3">
                          {recording.transcription}
                        </p>
                      </div>
                    )}
                    
                    {playingId === recording.id && (
                      <div className="mt-3">
                        <div className="flex items-center space-x-2 mb-1">
                          <span className="text-xs text-primary font-medium">Playing</span>
                        </div>
                        <div className="h-1 bg-dark-bg rounded-full overflow-hidden">
                          <div className="h-full bg-primary rounded-full animate-pulse" />
                        </div>
                      </div>
                    )}
                  </div>
                  
                  <div className="flex items-center space-x-2 ml-4">
                    <button
                      onClick={() => void playRecording(recording)}
                      className={`btn-sm flex items-center space-x-1 ${
                        playingId === recording.id 
                          ? 'btn-primary' 
                          : 'btn-secondary'
                      }`}
                      title={playingId === recording.id ? 'Pause' : 'Play'}
                    >
                      {playingId === recording.id ? (
                        <PauseCircle className="w-5 h-5" />
                      ) : (
                        <PlayCircle className="w-5 h-5" />
                      )}
                    </button>
                    
                    {transcribing === recording.id ? (
                      <Loader2 className="w-5 h-5 animate-spin text-primary" />
                    ) : !recording.transcription ? (
                      <button
                        onClick={() => transcribeRecording(recording)}
                        className="btn-secondary btn-sm"
                      >
                        Transcribe
                      </button>
                    ) : null}
                    
                    <button
                      onClick={async () => {
                        try {
                          console.log('Downloading recording:', recording.title)
                          // Convert file path to a URL that can be accessed
                          const audioUrl = convertFileSrc(recording.file_path)
                          
                          // Fetch the audio file
                          const response = await fetch(audioUrl)
                          if (!response.ok) {
                            throw new Error('Failed to fetch audio file')
                          }
                          
                          // Get the blob
                          const blob = await response.blob()
                          
                          // Create a download link
                          const url = URL.createObjectURL(blob)
                          const link = document.createElement('a')
                          link.href = url
                          link.download = `${recording.title}.wav`
                          document.body.appendChild(link)
                          link.click()
                          document.body.removeChild(link)
                          
                          // Clean up
                          setTimeout(() => URL.revokeObjectURL(url), 100)
                        } catch (error) {
                          console.error('Failed to download recording:', error)
                          alert('Failed to download recording. Please try again.')
                        }
                      }}
                      className="btn-secondary btn-sm flex items-center space-x-1"
                      title="Download"
                    >
                      <Download className="w-4 h-4" />
                    </button>
                    
                    <button
                      onClick={() => {
                        console.log('Delete button onClick triggered for:', recording.id)
                        deleteRecording(recording.id)
                      }}
                      className="btn-secondary btn-sm text-danger hover:bg-danger/20"
                      style={{ pointerEvents: 'auto', cursor: 'pointer' }}
                      title="Delete recording"
                      type="button"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
        )}
      </div>

      {/* Delete Confirmation Dialog */}
      {deletingId && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <motion.div
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            className="bg-dark-surface rounded-lg p-6 max-w-md w-full mx-4"
          >
            <h3 className="text-xl font-semibold mb-4">Delete Recording?</h3>
            <p className="text-gray-400 mb-6">
              Are you sure you want to delete this recording? This action cannot be undone.
            </p>
            <div className="flex justify-end space-x-4">
              <button
                onClick={() => setDeletingId(null)}
                className="btn-secondary"
              >
                Cancel
              </button>
              <button
                onClick={confirmDelete}
                className="btn-danger"
              >
                Delete
              </button>
            </div>
          </motion.div>
        </div>
      )}
    </div>
  )
}