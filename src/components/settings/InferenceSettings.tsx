import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { motion } from 'framer-motion'
import { Cpu, Cloud, Download, CheckCircle, AlertCircle } from 'lucide-react'

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
}

interface InferenceInfo {
  provider: string
  model_name: string
  candle_info?: {
    model_type: string
    device: string
    cache_dir: string
  }
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

  useEffect(() => {
    loadConfig()
  }, [])

  const loadConfig = async () => {
    try {
      const [configData, models, info, path] = await Promise.all([
        invoke<InferenceConfig>('get_inference_config'),
        invoke<CandleModel[]>('get_candle_models'),
        invoke<InferenceInfo>('get_inference_info'),
        invoke<string>('get_config_path')
      ])

      setCandleModels(models)
      setInferenceInfo(info)
      setSelectedProvider(configData.provider)
      setSelectedCandleModel(configData.candle_model_id)
      setOllamaModel(configData.ollama_model)
      setConfigPath(path)
    } catch (error) {
      console.error('Failed to load inference config:', error)
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
      const modelId = selectedProvider === 'Candle' ? selectedCandleModel : ollamaModel
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
            </div>
          )}
        </div>
      )}

      {/* Provider Selection */}
      <div className="mb-6">
        <label className="block text-sm font-medium mb-3">Inference Provider</label>
        <div className="grid grid-cols-2 gap-4">
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
        </div>
      </div>

      {/* Model Selection */}
      {selectedProvider === 'Candle' ? (
        <div className="mb-6">
          <label className="block text-sm font-medium mb-3">Candle Model</label>
          <div className="space-y-3">
            {candleModels.map((model) => (
              <motion.div
                key={model.id}
                whileTap={{ scale: 0.98 }}
                onClick={() => setSelectedCandleModel(model.id)}
                className={`p-4 rounded-lg border cursor-pointer transition-all ${
                  selectedCandleModel === model.id
                    ? 'border-primary bg-primary/5'
                    : 'border-gray-700 hover:border-gray-600'
                }`}
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <p className="font-medium">{model.name}</p>
                    <p className="text-sm text-gray-400 mt-1">{model.description}</p>
                    <p className="text-xs text-gray-500 mt-2">Model ID: {model.id}</p>
                  </div>
                  <div className="text-right ml-4">
                    <p className="text-sm text-gray-400">{model.size}</p>
                    {selectedCandleModel === model.id && (
                      <CheckCircle className="w-5 h-5 text-primary mt-2" />
                    )}
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
                ? 'Candle runs models locally on your device. First run will download the model (~2-14GB).'
                : 'Ollama requires the Ollama service to be running on your system.'
              }
            </p>
          </div>
        </div>
      </div>

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
                  Restart the app for changes to take effect.
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