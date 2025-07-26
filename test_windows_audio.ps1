# Windows Audio Recording Test Script for Personal Assistant

Write-Host "Personal Assistant - Windows Audio Recording Test" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan

# Check if running as administrator
$currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
$isAdmin = $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "WARNING: Not running as administrator. Some features may not work." -ForegroundColor Yellow
}

# Check Windows Audio Service
Write-Host "`nChecking Windows Audio Service..." -ForegroundColor Green
$audioService = Get-Service -Name "AudioSrv" -ErrorAction SilentlyContinue
if ($audioService) {
    Write-Host "  Status: $($audioService.Status)" -ForegroundColor $(if ($audioService.Status -eq 'Running') { 'Green' } else { 'Red' })
    if ($audioService.Status -ne 'Running') {
        Write-Host "  Attempting to start Windows Audio Service..." -ForegroundColor Yellow
        try {
            Start-Service -Name "AudioSrv"
            Write-Host "  Service started successfully!" -ForegroundColor Green
        } catch {
            Write-Host "  Failed to start service: $_" -ForegroundColor Red
        }
    }
} else {
    Write-Host "  Windows Audio Service not found!" -ForegroundColor Red
}

# Check audio devices
Write-Host "`nChecking Audio Input Devices..." -ForegroundColor Green
try {
    # Use WMI to get audio devices
    $audioDevices = Get-WmiObject Win32_SoundDevice | Where-Object { $_.StatusInfo -eq 3 }
    if ($audioDevices) {
        $audioDevices | ForEach-Object {
            Write-Host "  - $($_.Name)" -ForegroundColor Cyan
        }
    } else {
        Write-Host "  No active audio devices found via WMI" -ForegroundColor Yellow
    }
} catch {
    Write-Host "  Failed to enumerate devices via WMI: $_" -ForegroundColor Red
}

# Check microphone permissions
Write-Host "`nChecking Microphone Permissions..." -ForegroundColor Green
try {
    $micPermission = Get-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone" -Name "Value" -ErrorAction SilentlyContinue
    if ($micPermission.Value -eq "Allow") {
        Write-Host "  Microphone access is ALLOWED" -ForegroundColor Green
    } else {
        Write-Host "  Microphone access is DENIED or not set" -ForegroundColor Red
        Write-Host "  Please enable microphone access in Windows Settings > Privacy > Microphone" -ForegroundColor Yellow
    }
} catch {
    Write-Host "  Could not check microphone permissions" -ForegroundColor Yellow
}

# Check for exclusive mode
Write-Host "`nChecking Audio Exclusive Mode Settings..." -ForegroundColor Green
Write-Host "  To disable exclusive mode:" -ForegroundColor Yellow
Write-Host "  1. Right-click speaker icon in system tray" -ForegroundColor Yellow
Write-Host "  2. Select 'Sounds' > 'Recording' tab" -ForegroundColor Yellow
Write-Host "  3. Select your microphone > Properties > Advanced" -ForegroundColor Yellow
Write-Host "  4. Uncheck 'Allow applications to take exclusive control'" -ForegroundColor Yellow

# Environment setup
Write-Host "`nSetting up environment variables..." -ForegroundColor Green
$env:RUST_LOG = "personalassistant=debug,cpal=debug,personalassistant::audio=trace"
Write-Host "  RUST_LOG set to: $env:RUST_LOG" -ForegroundColor Cyan

# Build instructions
Write-Host "`nBuild and Run Instructions:" -ForegroundColor Green
Write-Host "  1. Build for Windows:" -ForegroundColor Yellow
Write-Host "     npm run tauri:build -- --target x86_64-pc-windows-msvc" -ForegroundColor Cyan
Write-Host ""
Write-Host "  2. Run in development mode with debug logging:" -ForegroundColor Yellow
Write-Host "     npm run tauri:dev" -ForegroundColor Cyan
Write-Host ""
Write-Host "  3. Test commands in the app console:" -ForegroundColor Yellow
Write-Host @"
     // List devices
     const devices = await invoke('list_audio_devices');
     console.log('Devices:', devices);
     
     // Start recording
     const info = await invoke('start_audio_recording', {
       devices: ['Default Input'],
       title: 'Test Recording'
     });
     console.log('Started:', info);
     
     // Stop recording
     const recording = await invoke('stop_audio_recording');
     console.log('Stopped:', recording);
"@ -ForegroundColor Cyan

# Test with built-in recorder
Write-Host "`n`nQuick Test with Windows Voice Recorder:" -ForegroundColor Green
Write-Host "  Opening Windows Voice Recorder for comparison test..." -ForegroundColor Yellow
try {
    Start-Process "ms-callrecording:"
} catch {
    Write-Host "  Could not open Voice Recorder. Try manually from Start Menu." -ForegroundColor Yellow
}

Write-Host "`n`nTest Complete!" -ForegroundColor Green
Write-Host "If Windows Voice Recorder works but Personal Assistant doesn't," -ForegroundColor Yellow
Write-Host "check the debug logs for specific error messages." -ForegroundColor Yellow