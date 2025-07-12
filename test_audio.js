// Test script to check if audio recording is working
const testAudioRecording = async () => {
  const { invoke } = window.__TAURI__.core;
  
  console.log('Testing audio recording...');
  
  try {
    // List audio devices
    console.log('Listing audio devices...');
    const devices = await invoke('list_audio_devices');
    console.log('Available devices:', devices);
    
    // Start recording
    console.log('Starting recording...');
    const recordingInfo = await invoke('start_audio_recording', {
      devices: devices.length > 0 ? [devices[0].name] : ['default'],
      title: 'Test Recording'
    });
    console.log('Recording started:', recordingInfo);
    
    // Wait 5 seconds
    console.log('Recording for 5 seconds...');
    await new Promise(resolve => setTimeout(resolve, 5000));
    
    // Stop recording
    console.log('Stopping recording...');
    const result = await invoke('stop_audio_recording');
    console.log('Recording stopped:', result);
    
    console.log('Test completed successfully!');
  } catch (error) {
    console.error('Test failed:', error);
  }
};

// Run the test
testAudioRecording();