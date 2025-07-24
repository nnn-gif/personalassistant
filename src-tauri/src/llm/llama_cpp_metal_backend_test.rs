#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_llama_cpp_backend_initialization() {
        println!("Testing LlamaCpp backend initialization...");
        
        let cache_dir = PathBuf::from("/tmp/test_llama_cache");
        let result = LlamaCppMetalBackend::new(
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0",
            cache_dir,
        ).await;
        
        // Should succeed in downloading the model
        assert!(result.is_ok(), "Failed to initialize backend: {:?}", result.err());
        
        let backend = result.unwrap();
        assert!(backend.use_metal == cfg!(target_os = "macos"));
    }

    #[tokio::test]
    async fn test_llama_cpp_generation() {
        let cache_dir = dirs::cache_dir()
            .unwrap()
            .join("huggingface")
            .join("hub");
            
        let backend = LlamaCppMetalBackend::new(
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0",
            cache_dir,
        ).await.expect("Failed to create backend");
        
        let result = backend.generate("Hello", 10).await;
        assert!(result.is_ok(), "Generation failed: {:?}", result.err());
        
        let response = result.unwrap();
        assert!(!response.is_empty());
        assert!(response.contains("LlamaCpp")); // Our placeholder response
    }

    #[test]
    fn test_chat_template() {
        let backend = LlamaCppMetalBackend {
            model_id: "TinyLlama/TinyLlama-1.1B-Chat-v1.0".to_string(),
            cache_dir: PathBuf::new(),
            model_path: None,
            use_metal: false,
        };
        
        let prompt = "Hello, how are you?";
        let formatted = backend.apply_chat_template(prompt);
        
        assert!(formatted.contains("<|system|>"));
        assert!(formatted.contains("<|user|>"));
        assert!(formatted.contains(prompt));
        assert!(formatted.contains("<|assistant|>"));
    }
    
    #[test]
    fn test_qwen_chat_template() {
        let backend = LlamaCppMetalBackend {
            model_id: "Qwen/Qwen2.5-0.5B-Instruct".to_string(),
            cache_dir: PathBuf::new(),
            model_path: None,
            use_metal: false,
        };
        
        let prompt = "What is 2+2?";
        let formatted = backend.apply_chat_template(prompt);
        
        assert!(formatted.contains("<|im_start|>"));
        assert!(formatted.contains("<|im_end|>"));
        assert!(formatted.contains(prompt));
    }
}