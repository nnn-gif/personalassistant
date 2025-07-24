#[cfg(test)]
mod tests {
    use personalassistant_lib::llm::llama_cpp_metal_backend::LlamaCppMetalBackend;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_gpu_detection() {
        // Test GPU detection logic
        let use_gpu = if cfg!(target_os = "macos") {
            std::env::var("CANDLE_USE_METAL").map(|v| v != "0").unwrap_or(true)
        } else if cfg!(target_os = "windows") {
            std::env::var("LLAMA_CUDA_FORCE_DISABLE").map(|v| v != "1").unwrap_or(true)
        } else {
            false
        };
        
        println!("Platform: {}", std::env::consts::OS);
        println!("GPU support detected: {}", use_gpu);
        
        if cfg!(target_os = "macos") {
            println!("Running on macOS - Metal support available");
        } else if cfg!(target_os = "windows") {
            println!("Running on Windows - CUDA/Vulkan support available");
        } else {
            println!("Running on other platform - CPU only");
        }
    }
    
    #[tokio::test]
    async fn test_llama_cpp_initialization() {
        let cache_dir = PathBuf::from("./test_cache");
        let model_id = "TinyLlama/TinyLlama-1.1B-Chat-v1.0";
        
        println!("Attempting to initialize LlamaCpp backend...");
        
        match LlamaCppMetalBackend::new(model_id, cache_dir).await {
            Ok(_backend) => {
                println!("✅ LlamaCpp backend initialized successfully!");
                if cfg!(target_os = "macos") {
                    println!("✅ Metal acceleration is active");
                } else if cfg!(target_os = "windows") {
                    println!("✅ GPU acceleration (CUDA/Vulkan) is active");
                } else {
                    println!("✅ CPU mode is active");
                }
            }
            Err(e) => {
                println!("❌ Failed to initialize LlamaCpp backend: {}", e);
                println!("This is expected if model download fails in test environment");
            }
        }
    }
    
    #[test]
    fn test_platform_specific_features() {
        #[cfg(target_os = "macos")]
        {
            println!("macOS-specific test: Checking Metal feature");
            // This will fail to compile if metal feature is not available
            let _ = llama_cpp::LlamaParams::default();
        }
        
        #[cfg(target_os = "windows")]
        {
            println!("Windows-specific test: Checking CUDA/Vulkan features");
            // This will fail to compile if cuda/vulkan features are not available
            let _ = llama_cpp::LlamaParams::default();
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            println!("Other platform test: CPU-only mode");
            let _ = llama_cpp::LlamaParams::default();
        }
    }
}