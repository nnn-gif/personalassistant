import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { motion } from 'framer-motion'
import { Cpu, Cloud, Download, CheckCircle, AlertCircle, Zap, HardDrive } from 'lucide-react'

interface InferenceConfig {
  provider: string
  ollama_model: string
  candle_model_id: string
  candle_model_revision: string
  available_providers: string[]
}

interface CandleModel {
  id: string
  name: string
  description: string
  size: string
  downloaded: boolean
  download_path?: string
}

interface InferenceInfo {
  provider: string
  model_name: string
  candle_info?: {
    model_type: string
    device: string
    cache_dir: string
    loaded: boolean
    tokenizer_loaded: boolean
  }
}

interface DownloadProgress {
  model_id: string
  status: string
  progress: number
  message: string
}

export default function InferenceSettings() {
  const [candleModels, setCandleModels] = useState<CandleModel[]>([])
  const [inferenceInfo, setInferenceInfo] = useState<InferenceInfo | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [selectedProvider, setSelectedProvider] = useState<string>('Ollama')
  const [selectedCandleModel, setSelectedCandleModel] = useState<string>('')
  const [ollamaModel, setOllamaModel] = useState<string>('')
  const [configPath, setConfigPath] = useState<string>('')
  const [saveSuccess, setSaveSuccess] = useState(false)
  const [downloadingModel, setDownloadingModel] = useState<string | null>(null)
  const [downloadProgress, setDownloadProgress] = useState<Record<string, DownloadProgress>>({})

  useEffect(() => {
    loadConfig()
  }, [])

  useEffect(() => {
    // Listen for download progress events
    const unlisten = listen<DownloadProgress>('download-progress', (event) => {
      const { model_id, status, progress, message } = event.payload
      
      console.log(`[Download Progress] ${model_id}: ${status} - ${progress}% - ${message}`)
      
      // Update progress state
      setDownloadProgress(prev => ({
        ...prev,
        [model_id]: { model_id, status, progress, message }
      }))
      
      // Update the downloading state based on status
      if (status === 'completed' || status === 'error') {
        setDownloadingModel(null)
        // Clear progress after a delay
        setTimeout(() => {
          setDownloadProgress(prev => {
            const newProgress = { ...prev }
            delete newProgress[model_id]
            return newProgress
          })
        }, 2000)
        // Reload models to update their download status
        loadConfig()
      }
    })

    // Cleanup listener on unmount
    return () => {
      unlisten.then(fn => fn())
    }
  }, [])

  const loadConfig = async () => {
    try {
      // Load basic config first
      const configData = await invoke<InferenceConfig>('get_inference_config')
      setSelectedProvider(configData.provider)
      setSelectedCandleModel(configData.candle_model_id)
      setOllamaModel(configData.ollama_model)
      
      // Load models list
      try {
        const models = await invoke<CandleModel[]>('get_candle_models')
        setCandleModels(models)
      } catch (error) {
        console.error('Failed to load candle models:', error)
        setCandleModels([])
      }
      
      // Load config path
      try {
        const path = await invoke<string>('get_config_path')
        setConfigPath(path)
      } catch (error) {
        console.error('Failed to load config path:', error)
      }
      
      // Load inference info with timeout
      try {
        const infoPromise = invoke<InferenceInfo>('get_inference_info')
        const timeoutPromise = new Promise((_, reject) => 
          setTimeout(() => reject(new Error('Timeout loading inference info')), 5000)
        )
        
        const info = await Promise.race([infoPromise, timeoutPromise]) as InferenceInfo
        setInferenceInfo(info)
      } catch (error) {
        console.error('Failed to load inference info:', error)
        // Set a default/fallback inference info
        setInferenceInfo({
          provider: configData.provider,
          model_name: configData.provider === 'Ollama' ? configData.ollama_model : configData.candle_model_id,
          candle_info: undefined
        })
      }
    } catch (error) {
      console.error('Failed to load inference config:', error)
      alert('Failed to load settings: ' + error)
    } finally {
      setLoading(false)
    }
  }

  const handleProviderChange = async (provider: string) => {
    setSelectedProvider(provider)
  }

  const saveConfiguration = async () => {
    setSaving(true)
    setSaveSuccess(false)
    try {
      const modelId = selectedProvider === 'Candle' || selectedProvider === 'Crane' || selectedProvider === 'Callm' || selectedProvider === 'LlamaCpp' ? selectedCandleModel : ollamaModel
      await invoke('set_inference_provider', {
        provider: selectedProvider,
        modelId
      })

      // Reload config to confirm changes
      await loadConfig()
      
      // Show success message
      setSaveSuccess(true)
      setTimeout(() => setSaveSuccess(false), 5000) // Hide after 5 seconds
    } catch (error) {
      console.error('Failed to save configuration:', error)
      alert('Failed to save configuration: ' + error)
    } finally {
      setSaving(false)
    }
  }
  
  const downloadModel = async (modelId: string) => {
    setDownloadingModel(modelId)
    try {
      const result = await invoke<string>('download_model', { modelId })
      console.log('Model download result:', result)
      
      // Reload models to update download status
      const models = await invoke<CandleModel[]>('get_candle_models')
      setCandleModels(models)
      
      // Find the downloaded model
      const downloadedModel = models.find(m => m.id === modelId)
      if (downloadedModel?.downloaded) {
        // Auto-select the downloaded model if it's now available
        if (selectedProvider === 'Candle' || selectedProvider === 'Crane' || selectedProvider === 'Callm' || selectedProvider === 'LlamaCpp') {
          setSelectedCandleModel(modelId)
        }
      }
    } catch (error) {
      console.error('Failed to download model:', error)
      alert(`Failed to download model: ${error}`)
    } finally {
      setDownloadingModel(null)
    }
  }

  if (loading) {
    return (
      <div className="card">
        <p className="text-gray-400 text-center py-8">Loading inference settings...</p>
      </div>
    )
  }

  return (
    <div className="card">
      <h3 className="text-xl font-semibold mb-6">Inference Settings</h3>
      
      {/* Current Status */}
      {inferenceInfo && (
        <div className="mb-6 p-4 bg-dark-bg rounded-lg">
          <p className="text-sm text-gray-400 mb-2">Current Configuration</p>
          <div className="flex items-center space-x-2">
            {inferenceInfo.provider === 'Candle' ? (
              <Cpu className="w-4 h-4 text-primary" />
            ) : inferenceInfo.provider === 'Crane' ? (
              <Zap className="w-4 h-4 text-primary" />
            ) : inferenceInfo.provider === 'Callm' ? (
              <Zap className="w-4 h-4 text-primary" />
            ) : inferenceInfo.provider === 'LlamaCpp' ? (
              <HardDrive className="w-4 h-4 text-primary" />
            ) : (
              <Cloud className="w-4 h-4 text-primary" />
            )}
            <span className="font-medium">{inferenceInfo.provider}</span>
            <span className="text-sm text-gray-400">• {inferenceInfo.model_name}</span>
          </div>
          {inferenceInfo.candle_info && (
            <div className="mt-2 text-xs text-gray-500">
              <p>Device: {inferenceInfo.candle_info.device}</p>
              <p>Cache: {inferenceInfo.candle_info.cache_dir}</p>
              <p className="mt-1 flex items-center space-x-2">
                <span>Model Status:</span>
                {inferenceInfo.candle_info.loaded ? (
                  <span className="text-green-400 flex items-center">
                    <CheckCircle className="w-3 h-3 mr-1" />
                    Loaded
                  </span>
                ) : (
                  <span className="text-yellow-400">Not loaded</span>
                )}
              </p>
              <p className="flex items-center space-x-2">
                <span>Tokenizer:</span>
                {inferenceInfo.candle_info.tokenizer_loaded ? (
                  <span className="text-green-400 flex items-center">
                    <CheckCircle className="w-3 h-3 mr-1" />
                    Ready
                  </span>
                ) : (
                  <span className="text-yellow-400">Not loaded</span>
                )}
              </p>
            </div>
          )}
        </div>
      )}

      {/* Provider Selection */}
      <div className="mb-6">
        <label className="block text-sm font-medium mb-3">Inference Provider</label>
        <div className="grid grid-cols-5 gap-4">
          <motion.button
            whileTap={{ scale: 0.95 }}
            onClick={() => handleProviderChange('Ollama')}
            className={`p-4 rounded-lg border-2 transition-all ${
              selectedProvider === 'Ollama'
                ? 'border-primary bg-primary/10'
                : 'border-gray-700 hover:border-gray-600'
            }`}
          >
            <Cloud className="w-6 h-6 mb-2 mx-auto" />
            <p className="font-medium">Ollama</p>
            <p className="text-xs text-gray-400 mt-1">Cloud-based inference</p>
          </motion.button>
          
          <motion.button
            whileTap={{ scale: 0.95 }}
            onClick={() => handleProviderChange('Candle')}
            className={`p-4 rounded-lg border-2 transition-all ${
              selectedProvider === 'Candle'
                ? 'border-primary bg-primary/10'
                : 'border-gray-700 hover:border-gray-600'
            }`}
          >
            <Cpu className="w-6 h-6 mb-2 mx-auto" />
            <p className="font-medium">Candle</p>
            <p className="text-xs text-gray-400 mt-1">Local inference</p>
          </motion.button>
          
          <motion.button
            whileTap={{ scale: 0.95 }}
            onClick={() => handleProviderChange('Crane')}
            className={`p-4 rounded-lg border-2 transition-all ${
              selectedProvider === 'Crane'
                ? 'border-primary bg-primary/10'
                : 'border-gray-700 hover:border-gray-600'
            }`}
          >
            <Zap className="w-6 h-6 mb-2 mx-auto" />
            <p className="font-medium">Crane</p>
            <p className="text-xs text-gray-400 mt-1">Fast CPU inference</p>
          </motion.button>
          
          <motion.button
            whileTap={{ scale: 0.95 }}
            onClick={() => handleProviderChange('Callm')}
            className={`p-4 rounded-lg border-2 transition-all ${
              selectedProvider === 'Callm'
                ? 'border-primary bg-primary/10'
                : 'border-gray-700 hover:border-gray-600'
            }`}
          >
            <Zap className="w-6 h-6 mb-2 mx-auto" />
            <p className="font-medium">Callm</p>
            <p className="text-xs text-gray-400 mt-1">Hardware accelerated</p>
          </motion.button>
          
          <motion.button
            whileTap={{ scale: 0.95 }}
            onClick={() => handleProviderChange('LlamaCpp')}
            className={`p-4 rounded-lg border-2 transition-all ${
              selectedProvider === 'LlamaCpp'
                ? 'border-primary bg-primary/10'
                : 'border-gray-700 hover:border-gray-600'
            }`}
          >
            <HardDrive className="w-6 h-6 mb-2 mx-auto" />
            <p className="font-medium">LlamaCpp</p>
            <p className="text-xs text-gray-400 mt-1">Metal optimized</p>
          </motion.button>
        </div>
      </div>

      {/* Model Status Summary */}
      {(selectedProvider === 'Candle' || selectedProvider === 'Crane' || selectedProvider === 'Callm' || selectedProvider === 'LlamaCpp') && (
        <div className="mb-4 p-3 bg-dark-bg rounded-lg">
          <div className="flex items-center justify-between">
            <p className="text-sm text-gray-400">
              Models: {candleModels.filter(m => m.downloaded).length} available, {candleModels.filter(m => !m.downloaded).length} not downloaded
            </p>
            {downloadingModel && (
              <p className="text-sm text-blue-400 flex items-center space-x-2">
                <div className="w-3 h-3 border-2 border-blue-400/30 border-t-blue-400 rounded-full animate-spin" />
                <span>Downloading model...</span>
              </p>
            )}
          </div>
        </div>
      )}

      {/* Model Selection */}
      {(selectedProvider === 'Candle' || selectedProvider === 'Crane' || selectedProvider === 'Callm' || selectedProvider === 'LlamaCpp') ? (
        <div className="mb-6">
          <label className="block text-sm font-medium mb-3">{selectedProvider} Model</label>
          <div className="space-y-3">
            {candleModels.map((model) => (
              <motion.div
                key={model.id}
                whileTap={{ scale: 0.98 }}
                onClick={() => setSelectedCandleModel(model.id)}
                className={`p-4 rounded-lg border cursor-pointer transition-all ${
                  selectedCandleModel === model.id
                    ? 'border-primary bg-primary/5'
                    : model.downloaded
                    ? 'border-gray-700 hover:border-gray-600 bg-gray-900/50'
                    : 'border-gray-800 hover:border-gray-700 bg-gray-950/50 opacity-90'
                }`}
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center space-x-2">
                      <p className="font-medium">{model.name}</p>
                      {/* Model Status Badge */}
                      {downloadProgress[model.id] ? (
                        <span className="text-xs bg-blue-500/20 text-blue-400 px-2 py-0.5 rounded flex items-center space-x-1">
                          <div className="w-2 h-2 border border-blue-400/50 border-t-blue-400 rounded-full animate-spin" />
                          <span>{Math.round(downloadProgress[model.id].progress)}%</span>
                        </span>
                      ) : model.downloaded ? (
                        <span className="text-xs bg-green-500/20 text-green-400 px-2 py-0.5 rounded flex items-center space-x-1">
                          <CheckCircle className="w-3 h-3" />
                          <span>Available</span>
                        </span>
                      ) : (
                        <span className="text-xs bg-gray-500/20 text-gray-400 px-2 py-0.5 rounded">
                          Not downloaded
                        </span>
                      )}
                    </div>
                    <p className="text-sm text-gray-400 mt-1">{model.description}</p>
                    <p className="text-xs text-gray-500 mt-2">Model ID: {model.id}</p>
                    {model.downloaded && model.download_path && (
                      <p className="text-xs text-gray-600 mt-1" title={model.download_path}>
                        📁 Cached locally
                      </p>
                    )}
                    {/* Download Progress Bar */}
                    {downloadProgress[model.id] && (
                      <div className="mt-3">
                        <div className="flex items-center justify-between text-xs text-gray-400 mb-1">
                          <span>{downloadProgress[model.id].message}</span>
                          <span>{Math.round(downloadProgress[model.id].progress)}%</span>
                        </div>
                        <div className="w-full h-2 bg-gray-800 rounded-full overflow-hidden">
                          <motion.div
                            className="h-full bg-blue-500"
                            initial={{ width: 0 }}
                            animate={{ width: `${downloadProgress[model.id].progress}%` }}
                            transition={{ duration: 0.3 }}
                          />
                        </div>
                      </div>
                    )}
                  </div>
                  <div className="text-right ml-4">
                    <p className="text-sm text-gray-400">{model.size}</p>
                    {/* Action Buttons */}
                    <div className="mt-2">
                      {!model.downloaded ? (
                        <motion.button
                          whileTap={{ scale: 0.95 }}
                          onClick={(e) => {
                            e.stopPropagation()
                            downloadModel(model.id)
                          }}
                          disabled={!!downloadProgress[model.id]}
                          className="px-3 py-1 text-xs bg-primary/20 text-primary rounded hover:bg-primary/30 disabled:opacity-50 disabled:cursor-not-allowed flex items-center space-x-1"
                        >
                          {downloadProgress[model.id] ? (
                            <>
                              <div className="w-3 h-3 border-2 border-primary/30 border-t-primary rounded-full animate-spin" />
                              <span>Downloading...</span>
                            </>
                          ) : (
                            <>
                              <Download className="w-3 h-3" />
                              <span>Download</span>
                            </>
                          )}
                        </motion.button>
                      ) : (
                        <div className="flex items-center justify-end space-x-2">
                          {selectedCandleModel === model.id ? (
                            <div className="flex items-center text-primary space-x-1">
                              <CheckCircle className="w-4 h-4" />
                              <span className="text-xs">Selected</span>
                            </div>
                          ) : (
                            <span className="text-xs text-green-400">Ready to use</span>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      ) : (
        <div className="mb-6">
          <label className="block text-sm font-medium mb-3">Ollama Model</label>
          <input
            type="text"
            value={ollamaModel}
            onChange={(e) => setOllamaModel(e.target.value)}
            className="w-full px-4 py-2 bg-dark-bg rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
            placeholder="e.g., llama3.2:1b"
          />
          <p className="text-xs text-gray-400 mt-2">
            Enter the Ollama model name (e.g., llama3.2:1b, phi:latest)
          </p>
        </div>
      )}

      {/* Info Box */}
      <div className="mb-6 p-4 bg-blue-500/10 border border-blue-500/20 rounded-lg">
        <div className="flex items-start space-x-2">
          <AlertCircle className="w-5 h-5 text-blue-400 mt-0.5" />
          <div className="text-sm text-blue-300">
            <p className="font-medium mb-1">Important Note</p>
            <p className="text-xs">
              {selectedProvider === 'Candle' 
                ? 'Candle runs models locally on your device. Download models before use.'
                : selectedProvider === 'Crane'
                ? 'Crane provides optimized inference. Download models before use.'
                : selectedProvider === 'Callm'
                ? 'Callm provides hardware-accelerated inference with automatic device selection.'
                : selectedProvider === 'LlamaCpp'
                ? 'LlamaCpp provides full Metal support for Apple Silicon with optimized performance.'
                : 'Ollama requires the Ollama service to be running on your system.'
              }
            </p>
          </div>
        </div>
      </div>
      
      {/* Model Download Warning */}
      {(selectedProvider === 'Candle' || selectedProvider === 'Crane' || selectedProvider === 'Callm' || selectedProvider === 'LlamaCpp') && selectedCandleModel && (() => {
        const selectedModel = candleModels.find(m => m.id === selectedCandleModel)
        return selectedModel && !selectedModel.downloaded ? (
          <div className="mb-6 p-4 bg-yellow-500/10 border border-yellow-500/20 rounded-lg">
            <div className="flex items-start space-x-2">
              <AlertCircle className="w-5 h-5 text-yellow-400 mt-0.5" />
              <div className="text-sm text-yellow-300">
                <p className="font-medium mb-1">Model Not Downloaded</p>
                <p className="text-xs">
                  The selected model "{selectedModel.name}" is not downloaded yet. 
                  Please download it before using {selectedProvider}.
                </p>
              </div>
            </div>
          </div>
        ) : null
      })()}

      {/* Save Button and Status */}
      <div className="space-y-4">
        {/* Success Message */}
        {saveSuccess && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            className="p-4 bg-green-500/10 border border-green-500/20 rounded-lg"
          >
            <div className="flex items-center space-x-2">
              <CheckCircle className="w-5 h-5 text-green-400" />
              <div>
                <p className="font-medium text-green-300">Configuration Saved!</p>
                <p className="text-xs text-green-400 mt-1">
                  Changes will take effect immediately on your next chat message.
                </p>
              </div>
            </div>
          </motion.div>
        )}
        
        {/* Config Path */}
        {configPath && (
          <div className="text-xs text-gray-500">
            <p>Configuration saved to:</p>
            <p className="font-mono text-gray-400 mt-1 break-all">{configPath}</p>
          </div>
        )}
        
        <div className="flex justify-end">
          <motion.button
            whileTap={{ scale: 0.95 }}
            onClick={saveConfiguration}
            disabled={saving}
            className="px-6 py-2 bg-primary text-white rounded-lg hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center space-x-2"
          >
            {saving ? (
              <>
                <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                <span>Saving...</span>
              </>
            ) : (
              <>
                <Download className="w-4 h-4" />
                <span>Save Configuration</span>
              </>
            )}
          </motion.button>
        </div>
      </div>
    </div>
  )
}