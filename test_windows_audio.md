# Windows Audio Recording Test Guide

## Common Windows Audio Issues and Solutions

### 1. WASAPI Exclusive Mode Issues
**Problem**: Windows audio devices may be locked in exclusive mode by other applications.
**Solution**: 
- Ensure no other recording apps are running
- Check Windows Sound Settings → Recording → Device Properties → Advanced
- Uncheck "Allow applications to take exclusive control"

### 2. Sample Format Mismatch
**Problem**: Windows devices often support different sample formats than expected.
**Solution**: The WindowsAudioRecorder now handles F32, I16, and U16 formats automatically.

### 3. Permission Issues
**Problem**: Windows may block microphone access.
**Solution**:
- Go to Windows Settings → Privacy → Microphone
- Ensure "Allow apps to access your microphone" is ON
- Add the Personal Assistant app to allowed apps

### 4. Device Enumeration
**Problem**: Some USB devices may not appear immediately.
**Solution**: The recorder now properly enumerates all available devices and provides a "Default Input" option.

## Testing Steps

### 1. List Available Devices
```javascript
// In the app console
const devices = await invoke('list_audio_devices');
console.log('Available devices:', devices);
```

### 2. Start Recording with Default Device
```javascript
const recordingInfo = await invoke('start_audio_recording', {
  devices: ['Default Input'],
  title: 'Test Recording'
});
console.log('Recording started:', recordingInfo);
```

### 3. Start Recording with Specific Device
```javascript
const recordingInfo = await invoke('start_audio_recording', {
  devices: ['Microphone (USB Audio Device)'],
  title: 'USB Mic Test'
});
```

### 4. Stop Recording
```javascript
const recording = await invoke('stop_audio_recording');
console.log('Recording saved:', recording);
```

### 5. Check Recording Status
```javascript
const status = await invoke('get_recording_status');
console.log('Status:', status);
```

## Debugging Audio Issues

### Enable Debug Logging
Set environment variable before running:
```powershell
$env:RUST_LOG="personalassistant=debug,cpal=debug"
npm run tauri:dev
```

### Check Windows Audio Service
```powershell
# Check if Windows Audio service is running
Get-Service -Name "AudioSrv"

# Restart if needed
Restart-Service -Name "AudioSrv"
```

### Test with Windows Voice Recorder
If the built-in Windows Voice Recorder app works, the Personal Assistant should work too.

## Error Messages and Solutions

### "No default input device found"
- Check if any microphone is connected
- Check Device Manager for disabled audio devices
- Try updating audio drivers

### "Failed to build stream"
- The device might be in use by another application
- Try closing Zoom, Teams, Discord, etc.
- Restart the Windows Audio service

### "Failed to get supported configs"
- This usually means the device doesn't support standard formats
- The recorder will automatically try alternative configurations

## Performance Optimization

### Buffer Size
The recorder uses default buffer sizes for low latency. If you experience dropouts:
- Close unnecessary applications
- Increase Windows audio buffer size in device properties

### Sample Rate
The recorder automatically selects the best available sample rate. Common rates:
- 48000 Hz (recommended)
- 44100 Hz (CD quality)
- 16000 Hz (voice quality)

## Known Limitations

1. **Pause/Resume**: Not yet implemented on Windows
2. **Multiple Device Recording**: Currently supports single device only
3. **Real-time Monitoring**: Audio monitoring during recording not implemented

## Testing Checklist

- [ ] Device enumeration works
- [ ] Can start recording with default device
- [ ] Can start recording with USB microphone
- [ ] Can stop recording and save file
- [ ] WAV file is playable in Windows Media Player
- [ ] File size and duration are correct
- [ ] No audio dropouts or glitches
- [ ] Proper error messages for common issues