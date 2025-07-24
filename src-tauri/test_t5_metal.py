#!/usr/bin/env python3
"""
Quick test to verify T5 model structure and requirements
"""

import json
import requests

model_id = "t5-small"
config_url = f"https://huggingface.co/{model_id}/raw/main/config.json"

# Download and check config
response = requests.get(config_url)
config = response.json()

print(f"Model: {model_id}")
print(f"Architecture: {config.get('model_type', 'unknown')}")
print(f"Hidden size: {config.get('d_model', 'unknown')}")
print(f"Num layers: {config.get('num_layers', 'unknown')}")
print(f"Vocab size: {config.get('vocab_size', 'unknown')}")

# Check normalization type
if 'layer_norm_epsilon' in config:
    print("✅ Uses standard LayerNorm (Metal compatible)")
else:
    print("❌ May use different normalization")

# Check for RMS norm indicators
config_str = json.dumps(config, indent=2)
if 'rms' in config_str.lower():
    print("⚠️  Found 'rms' in config - may not be Metal compatible")
else:
    print("✅ No RMS norm found in config")