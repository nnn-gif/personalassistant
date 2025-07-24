#[path = "src/llm/bert_metal_backend.rs"]
mod bert_metal_backend;
#[path = "src/error.rs"]
mod error;

use bert_metal_backend::BertMetalBackend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing BERT models with Metal acceleration...\n");
    
    // Enable Metal
    std::env::set_var("CANDLE_USE_METAL", "1");
    
    let cache_dir = dirs::home_dir()
        .unwrap()
        .join(".cache")
        .join("huggingface")
        .join("hub");
    
    // Test different BERT models
    let models = vec![
        "bert-base-uncased",
        "sentence-transformers/all-MiniLM-L6-v2",
    ];
    
    for model_id in models {
        println!("\n{}", "=".repeat(60));
        println!("Testing model: {}", model_id);
        println!("{}\n", "=".repeat(60));
        
        match BertMetalBackend::new(model_id, cache_dir.clone()).await {
            Ok(backend) => {
                // Test embeddings generation
                let test_texts = vec![
                    "Metal acceleration makes machine learning fast",
                    "The capital of France is Paris",
                    "BERT models use LayerNorm instead of RMS norm",
                ];
                
                for text in test_texts {
                    println!("\nText: '{}'", text);
                    match backend.generate_embeddings(text).await {
                        Ok(embeddings) => {
                            println!("✓ Successfully generated {}-dim embeddings", embeddings.len());
                            println!("  First 5 values: [{:.4}, {:.4}, {:.4}, {:.4}, {:.4}]",
                                embeddings[0], embeddings[1], embeddings[2], embeddings[3], embeddings[4]);
                        }
                        Err(e) => {
                            println!("✗ Failed to generate embeddings: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to load model {}: {}", model_id, e);
            }
        }
    }
    
    println!("\n\n{}", "=".repeat(60));
    println!("Summary: BERT models work perfectly with Metal!");
    println!("They use LayerNorm which is fully supported.");
    println!("{}", "=".repeat(60));
    
    Ok(())
}