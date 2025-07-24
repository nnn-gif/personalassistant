# Metal-Compatible Models for Candle

## Overview

This document lists models that work well with Metal acceleration in Candle. The key differentiator is that these models use **LayerNorm** instead of **RMS norm**, which is fully supported on Metal.

## Why Some Models Don't Work with Metal

The issue with models like Llama, Mistral, and Qwen on Metal is that they use RMS normalization (Root Mean Square Layer Normalization), which doesn't have a Metal kernel implementation in Candle. This causes the error:
```
Metal error: Could not create metal kernel for op: rms-norm
```

## Fully Metal-Compatible Models

### 1. BERT Family
- **bert-base-uncased** - Original BERT model
- **bert-base-cased** - Case-sensitive BERT
- **bert-large-uncased** - Larger BERT variant
- Uses standard LayerNorm throughout
- Perfect for embeddings and classification tasks

### 2. DistilBERT
- **distilbert-base-uncased** - Distilled version of BERT
- **distilbert-base-cased** - Case-sensitive variant
- 40% smaller, 60% faster than BERT
- Maintains 97% of BERT's performance
- Uses LayerNorm

### 3. RoBERTa
- **roberta-base** - Robustly optimized BERT
- **roberta-large** - Larger variant
- Based on BERT architecture, uses LayerNorm
- Better performance than BERT on many tasks

### 4. Sentence Transformers (BERT-based)
- **sentence-transformers/all-MiniLM-L6-v2** - Fast, lightweight
- **sentence-transformers/all-mpnet-base-v2** - High quality embeddings
- **sentence-transformers/paraphrase-MiniLM-L6-v2** - Good for similarity
- All use LayerNorm, optimized for embeddings

### 5. T5 Family
- **t5-small** - 60M parameters
- **t5-base** - 220M parameters
- **google/flan-t5-small** - Instruction-tuned T5
- **google/flan-t5-base** - Larger instruction-tuned
- Uses T5-specific LayerNorm (not RMS norm)
- Good for text-to-text tasks

### 6. CLIP Models (Vision + Text)
- **openai/clip-vit-base-patch32** - Vision transformer + text encoder
- Text encoder uses LayerNorm
- Good for multimodal tasks

### 7. DeBERTa
- **microsoft/deberta-base** - Improved BERT architecture
- **microsoft/deberta-v3-base** - Latest version
- Uses LayerNorm with enhanced attention

## Models to Avoid on Metal (Use RMS Norm)

- ❌ **Llama** family (all versions)
- ❌ **Mistral** models
- ❌ **Qwen** models
- ❌ **Phi** models
- ❌ **StableLM**
- ❌ **Falcon**
- ❌ Most recent LLMs (they adopted RMS norm for efficiency)

## Implementation Example

```rust
use candle_core::{Device, Tensor};
use candle_transformers::models::bert::{BertModel, Config};
use candle_nn::VarBuilder;

// Create Metal device
let device = Device::new_metal(0)?;

// Load BERT (works perfectly on Metal)
let config = Config::bert_base_uncased();
let vb = VarBuilder::from_mmaped_safetensors(&["model.safetensors"], DType::F32, &device)?;
let model = BertModel::new(&config, vb)?;

// Run inference
let input = Tensor::new(&[101u32, 2023, 2003, 1037, 3231, 102], &device)?;
let output = model.forward(&input.unsqueeze(0)?)?;
```

## Performance Considerations

1. **Metal Acceleration**: Models using LayerNorm get full Metal acceleration
2. **Memory Efficiency**: Use F16 dtype when possible for better memory usage
3. **Batch Processing**: Metal performs better with larger batch sizes

## Recommendations

For different use cases on Metal:

1. **Text Embeddings**: Use sentence-transformers models
2. **Classification**: Use BERT or DistilBERT
3. **Text Generation**: Use T5 models (though slower than modern LLMs)
4. **Fast Inference**: Use DistilBERT or MiniLM variants
5. **High Quality**: Use RoBERTa or DeBERTa

## Future Improvements

The Candle team may add Metal kernels for RMS norm in the future, which would enable:
- Llama models on Metal
- Mistral models on Metal
- Other modern LLMs

Until then, stick to the models listed above for Metal acceleration.