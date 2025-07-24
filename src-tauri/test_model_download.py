#!/usr/bin/env python3

from huggingface_hub import hf_hub_download, snapshot_download
import os

model_id = "TinyLlama/TinyLlama-1.1B-Chat-v1.0"
cache_dir = os.path.expanduser("~/.cache/huggingface/hub")

print(f"Testing download for {model_id}")
print(f"Cache directory: {cache_dir}")

# Check if standard model files exist
try:
    # Download model files
    print("\nDownloading model.safetensors...")
    model_path = hf_hub_download(
        repo_id=model_id,
        filename="model.safetensors",
        cache_dir=cache_dir
    )
    print(f"Downloaded to: {model_path}")
    
    # Download config
    print("\nDownloading config.json...")
    config_path = hf_hub_download(
        repo_id=model_id,
        filename="config.json",
        cache_dir=cache_dir
    )
    print(f"Downloaded to: {config_path}")
    
    # Download tokenizer
    print("\nDownloading tokenizer.json...")
    tokenizer_path = hf_hub_download(
        repo_id=model_id,
        filename="tokenizer.json",
        cache_dir=cache_dir
    )
    print(f"Downloaded to: {tokenizer_path}")
    
    print("\nAll files downloaded successfully!")
    
except Exception as e:
    print(f"Error downloading: {e}")

# Check GGUF version
print("\n" + "="*50)
print("Checking GGUF version...")
gguf_model = "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF"
gguf_file = "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf"

try:
    print(f"\nDownloading {gguf_file} from {gguf_model}...")
    gguf_path = hf_hub_download(
        repo_id=gguf_model,
        filename=gguf_file,
        cache_dir=cache_dir
    )
    print(f"Downloaded to: {gguf_path}")
    
except Exception as e:
    print(f"Error downloading GGUF: {e}")