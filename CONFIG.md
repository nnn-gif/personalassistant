# Personal Assistant Configuration Guide

The Personal Assistant now has a comprehensive configuration system that supports environment variables, configuration files, and user preferences.

## Configuration Hierarchy

Configuration is loaded in the following order (later sources override earlier ones):

1. **Default values** (hardcoded in the application)
2. **Configuration file** (`config.toml`)
3. **Environment variables** (`.env` file or system environment)

## Configuration Files

### 1. Environment Variables (`.env`)

Create a `.env` file in the project root:

```bash
cp .env.example .env
```

Available environment variables:

```env
# Service URLs
OLLAMA_URL=http://localhost:11434
QDRANT_URL=http://localhost:6333

# LLM Models
OLLAMA_MODEL=llama3.2:1b
OLLAMA_EMBEDDING_MODEL=nomic-embed-text:latest

# Activity Tracking
TRACKING_ENABLED=true
TRACKING_INTERVAL_MS=5000
IDLE_THRESHOLD_MS=300000

# RAG Configuration
RAG_CHUNK_SIZE=1000
RAG_CHUNK_OVERLAP=200
USE_QDRANT=false
QDRANT_COLLECTION_NAME=documents

# Audio Configuration
AUDIO_SAMPLE_RATE=44100
AUDIO_CHANNELS=1
ENABLE_TRANSCRIPTION=true

# Database Configuration
DATABASE_NAME=personal_assistant.db
ENABLE_DB_MIGRATIONS=true
```

### 2. Configuration File (`config.toml`)

The app looks for `config.toml` in the platform-specific config directory:

- **macOS**: `~/Library/Application Support/com.personalassistant.app/config.toml`
- **Windows**: `%APPDATA%\com.personalassistant.app\config.toml`
- **Linux**: `~/.config/com.personalassistant.app/config.toml`

Copy the example configuration:

```bash
cp config.toml.example ~/Library/Application Support/com.personalassistant.app/config.toml
```

### 3. User Preferences

User preferences are stored separately from configuration and can be managed through the app:

```json
{
  "theme": "system",
  "language": "en",
  "notifications_enabled": true,
  "auto_start_tracking": false,
  "window_opacity": 1.0,
  "default_view": "dashboard"
}
```

## API Commands

The configuration system exposes several Tauri commands:

### Get Current Configuration
```typescript
import { invoke } from '@tauri-apps/api/tauri';

const config = await invoke('get_config');
```

### Update Configuration
```typescript
await invoke('update_config', { config: newConfig });
```

### Get User Preferences
```typescript
const preferences = await invoke('get_user_preferences');
```

### Update User Preferences
```typescript
await invoke('update_user_preferences', { preferences: newPreferences });
```

### Reset Preferences to Defaults
```typescript
const defaultPreferences = await invoke('reset_preferences');
```

## Configuration Structure

The configuration is organized into the following sections:

### Services
- `ollama_url`: URL for the Ollama LLM service
- `qdrant_url`: URL for the Qdrant vector database
- `ollama_model`: Default LLM model to use
- `ollama_embedding_model`: Model for generating embeddings

### Tracking
- `enabled`: Whether activity tracking is enabled
- `tracking_interval_ms`: How often to track activities (milliseconds)
- `idle_threshold_ms`: Time before marking user as idle (milliseconds)

### RAG (Retrieval-Augmented Generation)
- `chunk_size`: Size of text chunks for processing
- `chunk_overlap`: Overlap between chunks
- `use_qdrant`: Whether to use Qdrant or local vector store
- `collection_name`: Name of the Qdrant collection

### Audio
- `sample_rate`: Audio sample rate (Hz)
- `channels`: Number of audio channels
- `enable_transcription`: Whether to enable audio transcription

### Database
- `db_name`: SQLite database filename
- `enable_migrations`: Whether to run database migrations on startup

## Best Practices

1. **Development**: Use `.env` files for easy configuration during development
2. **Production**: Use environment variables for sensitive values (API keys, URLs)
3. **User Settings**: Use the preferences system for UI/UX settings
4. **Deployment**: Create a `config.toml` for production deployments

## Troubleshooting

### Configuration Not Loading

1. Check that the `.env` file is in the project root
2. Verify environment variable names match exactly
3. Check logs for configuration validation errors

### Invalid Configuration

The app validates configuration on startup. Check the console for validation errors:

```
Configuration validation errors: ["Invalid Ollama URL format", "RAG chunk overlap must be less than chunk size"]
```

### Service Connection Issues

If services like Ollama or Qdrant aren't connecting:

1. Verify the service is running on the configured URL
2. Check that the URLs in your configuration are correct
3. Look for connection warnings in the console logs