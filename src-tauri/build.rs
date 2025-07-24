fn main() {
    // Set platform-specific configurations
    #[cfg(target_os = "windows")]
    {
        // Windows-specific build configurations
        println!("cargo:rustc-env=CANDLE_USE_METAL=0");
        println!("cargo:rustc-env=CANDLE_USE_CUDA=1");
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS-specific build configurations
        println!("cargo:rustc-env=CANDLE_USE_METAL=1");
        println!("cargo:rustc-env=CANDLE_USE_CUDA=0");
    }
    
    #[cfg(target_os = "linux")]
    {
        // Linux-specific build configurations
        println!("cargo:rustc-env=CANDLE_USE_METAL=0");
        println!("cargo:rustc-env=CANDLE_USE_CUDA=1");
    }
    
    tauri_build::build()
}
