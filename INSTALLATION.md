# Personal Assistant - Installation Guide

## System Requirements
- macOS 10.15+ (Catalina or newer)
- 4GB RAM minimum, 8GB recommended
- 500MB available storage space

## Prerequisites

### 1. Install Ollama
Personal Assistant requires Ollama for AI functionality:

```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Download required AI models
ollama pull nomic-embed-text
ollama pull llama3.2:1b
```

### 2. Install Qdrant (Optional)
For enhanced document search, install Qdrant:

```bash
# Using Docker (recommended)
docker run -d -p 6333:6333 --name qdrant qdrant/qdrant

# Or install directly from https://qdrant.tech/documentation/guides/installation/
```

## Installation

### macOS
1. Download the `.dmg` file from releases
2. Open the DMG file
3. Drag "Personal Assistant" to Applications folder
4. Launch from Applications or Spotlight

### First Launch
1. The app will create a local database for your data
2. Start tracking activities automatically
3. Index your first documents through the Document Manager
4. Create goals to organize your work

## Features
- **Activity Tracking**: Automatic monitoring of your computer usage
- **Document Management**: Index and search your personal documents
- **Goal Setting**: Create and track productivity goals
- **AI Chat**: Chat with your documents using local AI
- **Privacy-First**: All data stays on your device

## Troubleshooting

### "App can't be opened" (macOS)
If you see a security warning:
1. Go to System Preferences > Security & Privacy
2. Click "Open Anyway" next to the warning
3. Or run: `xattr -dr com.apple.quarantine /Applications/Personal\ Assistant.app`

### Ollama Connection Issues
- Ensure Ollama is running: `ollama serve`
- Check models are installed: `ollama list`
- Verify connection: `curl http://localhost:11434/api/tags`

### Performance Issues
- Close unused applications
- Ensure sufficient free disk space
- Check Activity Monitor for system resources

## Support
For issues or questions, visit: https://github.com/nnn-gif/personalassistant