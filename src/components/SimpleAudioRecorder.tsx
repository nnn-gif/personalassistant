import { useState, useEffect } from 'react'
import { Mic, Square } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { AudioDevice, RecordingInfo } from '../types'

export default function SimpleAudioRecorder() {
  const [devices, setDevices] = useState<AudioDevice[]>([])
  const [selectedDevice, setSelectedDevice] = useState<string>('')
  const [isRecording, setIsRecording] = useState(false)
  const [recordingInfo, setRecordingInfo] = useState<RecordingInfo | null>(null)
  const [error, setError] = useState<string>('')

  useEffect(() => {
    loadDevices()
  }, [])

  const loadDevices = async () => {
    try {
      console.log('Loading audio devices...')
      const deviceList = await invoke<AudioDevice[]>('list_audio_devices')
      console.log('Devices loaded:', deviceList)
      setDevices(deviceList)
      
      // Auto-select first device
      if (deviceList.length > 0) {
        setSelectedDevice(deviceList[0].name)
      }
    } catch (error) {
      console.error('Failed to load devices:', error)
      const errorMsg = String(error)
      if (errorMsg.includes('No audio input devices found')) {
        setError('No microphone detected. Please check your audio settings and ensure a microphone is connected.')
      } else if (navigator.userAgent.includes('Windows')) {
        setError('Failed to load audio devices. On Windows, please ensure microphone permissions are granted in Settings > Privacy > Microphone.')
      } else {
        setError('Failed to load audio devices: ' + error)
      }
    }
  }

  const startRecording = async () => {
    setError('')
    console.log('Starting recording with device:', selectedDevice)
    
    try {
      const result = await invoke<RecordingInfo>('start_audio_recording', {
        devices: selectedDevice ? [selectedDevice] : [],
        title: `Recording ${new Date().toLocaleString()}`
      })
      
      console.log('Recording started:', result)
      setRecordingInfo(result)
      setIsRecording(true)
    } catch (error) {
      console.error('Failed to start recording:', error)
      const errorMsg = String(error)
      if (errorMsg.includes('No supported input configurations')) {
        setError('Audio device not supported. Please try a different microphone.')
      } else if (errorMsg.includes('Failed to build stream')) {
        setError('Failed to access microphone. Please ensure no other application is using the microphone.')
      } else if (navigator.userAgent.includes('Windows')) {
        setError(`Failed to start recording. Windows error: ${error}\n\nTroubleshooting:\n1. Check microphone permissions in Windows Settings\n2. Ensure no other app is using the microphone\n3. Try restarting the application`)
      } else {
        setError('Failed to start recording: ' + error)
      }
    }
  }

  const stopRecording = async () => {
    console.log('Stopping recording...')
    
    try {
      const result = await invoke('stop_audio_recording')
      console.log('Recording stopped:', result)
      setIsRecording(false)
      setRecordingInfo(null)
    } catch (error) {
      console.error('Failed to stop recording:', error)
      setError('Failed to stop recording: ' + error)
    }
  }

  return (
    <div className="space-y-6">
      <header>
        <h2 className="text-3xl font-bold">Simple Audio Recorder</h2>
        <p className="text-gray-400 mt-2">Basic audio recording test</p>
      </header>

      {error && (
        <div className="p-4 bg-red-500/20 border border-red-500 rounded-lg text-red-300">
          {error}
        </div>
      )}

      <div className="card">
        <h3 className="text-xl font-semibold mb-4">Audio Devices</h3>
        
        {devices.length === 0 ? (
          <div className="text-gray-400">
            <p>No audio devices found</p>
            {navigator.userAgent.includes('Windows') && (
              <p className="text-sm mt-2">On Windows, please check:
                <br />• Microphone is connected
                <br />• Microphone permissions are enabled in Settings
                <br />• Audio drivers are up to date
              </p>
            )}
          </div>
        ) : (
          <select
            value={selectedDevice}
            onChange={(e) => setSelectedDevice(e.target.value)}
            className="input w-full mb-4"
            disabled={isRecording}
          >
            {devices.map((device) => (
              <option key={device.name} value={device.name}>
                {device.name} ({device.device_type})
              </option>
            ))}
          </select>
        )}

        <div className="flex items-center space-x-4">
          {!isRecording ? (
            <button
              onClick={startRecording}
              disabled={!selectedDevice}
              className="btn-primary flex items-center space-x-2"
            >
              <Mic className="w-5 h-5" />
              <span>Start Recording</span>
            </button>
          ) : (
            <button
              onClick={stopRecording}
              className="btn-danger flex items-center space-x-2"
            >
              <Square className="w-5 h-5" />
              <span>Stop Recording</span>
            </button>
          )}
        </div>

        {isRecording && recordingInfo && (
          <div className="mt-4 p-4 bg-primary/10 rounded-lg">
            <p className="text-sm">Recording in progress...</p>
            <pre className="text-xs mt-2 text-gray-400">
              {JSON.stringify(recordingInfo, null, 2)}
            </pre>
          </div>
        )}
      </div>

      <div className="card">
        <h3 className="text-xl font-semibold mb-4">Debug Info</h3>
        <div className="text-sm text-gray-400 space-y-1">
          <p>Devices: {devices.length}</p>
          <p>Selected: {selectedDevice || 'None'}</p>
          <p>Recording: {isRecording ? 'Yes' : 'No'}</p>
          <p>Button disabled: {!selectedDevice ? 'Yes' : 'No'}</p>
        </div>
      </div>
    </div>
  )
}