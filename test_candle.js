// Test script to verify Candle inference
// Run with: node test_candle.js

const { invoke } = require('@tauri-apps/api/core');

async function testCandleInference() {
  console.log('Testing Candle inference...\n');
  
  try {
    // Get current inference info
    const info = await invoke('get_inference_info');
    console.log('Current inference provider:', info.provider);
    console.log('Model:', info.model_name);
    
    if (info.candle_info) {
      console.log('\nCandle Info:');
      console.log('- Model Type:', info.candle_info.model_type);
      console.log('- Device:', info.candle_info.device);
      console.log('- Cache Dir:', info.candle_info.cache_dir);
      console.log('- Model Loaded:', info.candle_info.loaded);
      console.log('- Tokenizer Loaded:', info.candle_info.tokenizer_loaded);
    }
    
    // Test actual inference
    console.log('\nTesting inference...');
    const prompt = 'Analyze my productivity for today';
    
    // This would call the actual inference endpoint
    // const response = await invoke('generate_text', { prompt });
    // console.log('Response:', response);
    
  } catch (error) {
    console.error('Error:', error);
  }
}

// Note: This would need to be run within the Tauri app context
console.log('This test script demonstrates how to test Candle inference.');
console.log('To test within the app, use the developer console or add a test button.');