fn main() {
    // Platform-specific build configuration
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-env-changed=LLAMA_CUDA_FORCE_DISABLE");
        println!("cargo:rustc-cfg=feature=\"windows_gpu\"");
    }
    
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rerun-if-env-changed=CANDLE_USE_METAL");
        println!("cargo:rustc-cfg=feature=\"macos_metal\"");
    }
    
    // Run the standard Tauri build process
    tauri_build::build()
}
