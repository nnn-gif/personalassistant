# PowerShell script to verify Windows build configuration
Write-Host "Verifying Windows build configuration..." -ForegroundColor Green

# Check if running on Windows
if ($env:OS -eq "Windows_NT") {
    Write-Host "Running on Windows" -ForegroundColor Cyan
    
    # Check for CUDA/Vulkan environment
    Write-Host "`nChecking GPU environment variables:" -ForegroundColor Yellow
    Write-Host "LLAMA_CUDA_FORCE_DISABLE: $env:LLAMA_CUDA_FORCE_DISABLE"
    
    # Check for CUDA installation
    Write-Host "`nChecking for CUDA installation:" -ForegroundColor Yellow
    if (Test-Path "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA") {
        Write-Host "CUDA installation found" -ForegroundColor Green
    } else {
        Write-Host "CUDA not found - GPU acceleration may not work" -ForegroundColor Yellow
    }
    
    # Run cargo check to verify dependencies
    Write-Host "`nRunning cargo check..." -ForegroundColor Yellow
    cargo check --target x86_64-pc-windows-msvc 2>&1 | Out-String
    
    # Check if objc dependencies are being pulled in
    Write-Host "`nChecking for macOS dependencies in cargo tree:" -ForegroundColor Yellow
    $objcDeps = cargo tree | Select-String "objc"
    if ($objcDeps) {
        Write-Host "WARNING: Found objc dependencies that shouldn't be on Windows:" -ForegroundColor Red
        $objcDeps
    } else {
        Write-Host "No objc dependencies found - Good!" -ForegroundColor Green
    }
} else {
    Write-Host "This script is intended for Windows only" -ForegroundColor Red
}