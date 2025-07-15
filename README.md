# Personal Assistant 🤖

[![Build Status](https://github.com/nnn-gif/personalassistant/actions/workflows/build.yml/badge.svg)](https://github.com/nnn-gif/personalassistant/actions/workflows/build.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)](https://github.com/nnn-gif/personalassistant/releases)

A privacy-first, AI-powered personal productivity assistant that runs entirely on your device. Track activities, manage goals, process documents, and chat with your data using local AI models.

## ✨ Features

### 🔍 **Intelligent Document Processing**
- Index and search your personal documents (PDF, DOCX, TXT, and more)
- Advanced PDF text extraction with multiple fallback methods
- Chat with your documents using local AI models
- Support for 50+ file formats with comprehensive metadata extraction

### 📊 **Activity Monitoring**
- Automatic tracking of computer usage and application activity
- Privacy-focused monitoring (all data stays local)
- Detailed analytics and productivity insights
- Integration with goal tracking for context-aware productivity

### 🎯 **Goal Management**
- Create and track productivity goals
- Monitor progress with detailed metrics
- Goal-based document organization and filtering
- Time tracking with automatic activity correlation

### 🧠 **Local AI Integration**
- Chat with your documents using Ollama models
- Multiple LLM support (Llama, Qwen, Gemma, Mistral, etc.)
- Vector search with Qdrant or built-in similarity search
- Context-aware responses using both documents and activity data

### 🔒 **Privacy-First Design**
- All data processing happens locally on your device
- No external API calls or data sharing
- Encrypted local storage
- Complete control over your personal information

## 🚀 Quick Start

### Prerequisites
1. **Install Ollama** (required for AI features):
   ```bash
   curl -fsSL https://ollama.ai/install.sh | sh
   ```

2. **Download AI models**:
   ```bash
   ollama pull nomic-embed-text
   ollama pull llama3.2:1b
   ```

3. **Install Qdrant** (optional, for enhanced search):
   ```bash
   docker run -d -p 6333:6333 --name qdrant qdrant/qdrant
   ```

### Installation

#### Download from Releases
1. Go to [Releases](https://github.com/nnn-gif/personalassistant/releases)
2. Download the appropriate package for your platform:
   - **macOS**: Download the `.dmg` file
   - **Windows**: Download the `.msi` or `.exe` file  
   - **Linux**: Download the `.deb` or `.AppImage` file
3. Follow the installation instructions in [INSTALLATION.md](INSTALLATION.md)

#### Build from Source
```bash
git clone https://github.com/nnn-gif/personalassistant.git
cd personalassistant
npm install
npm run tauri:build
```

## 📖 Usage

### Getting Started
1. **Launch the application** and let it create the local database
2. **Start activity tracking** - monitoring begins automatically
3. **Index your documents** through the Document Manager
4. **Create goals** to organize your productivity tracking
5. **Chat with your data** using the Document Chat feature

### Key Workflows

#### Document Management
- Drag and drop files into the Document Manager
- Organize documents by goals for better context
- Use the search functionality to find relevant information
- Clear the vector database to start fresh when needed

#### Activity Tracking
- Monitor real-time activity in the Activity Monitor
- View productivity trends and insights
- Correlate activities with your goals
- Export activity data for external analysis

#### AI Chat
- Ask questions about your indexed documents
- Switch between different AI models for varied responses
- Get context-aware answers that include recent activity
- View source documents for chat responses

## 🛠️ Development

### Tech Stack
- **Frontend**: React + TypeScript + Tailwind CSS
- **Backend**: Rust + Tauri v2
- **Database**: SQLite with optional Qdrant vector store
- **AI**: Ollama for local LLM inference
- **Build**: GitHub Actions for cross-platform builds

### Development Setup
See [CONTRIBUTING.md](.github/CONTRIBUTING.md) for detailed setup instructions.

```bash
# Quick start
git clone https://github.com/nnn-gif/personalassistant.git
cd personalassistant
npm install
npm run tauri:dev
```

## 📋 System Requirements

- **macOS**: 10.15+ (Catalina or newer)
- **Windows**: Windows 10 version 1903+
- **Linux**: Ubuntu 18.04+ or equivalent
- **RAM**: 4GB minimum, 8GB recommended
- **Storage**: 500MB available space
- **Dependencies**: Ollama (required), Qdrant (optional)

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](.github/CONTRIBUTING.md) for details.

### Ways to Contribute
- 🐛 Report bugs and issues
- 💡 Suggest new features
- 📖 Improve documentation
- 🔧 Submit code improvements
- 🧪 Help with testing

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Tauri](https://tauri.app/) for the cross-platform framework
- [Ollama](https://ollama.ai/) for local AI model management
- [Qdrant](https://qdrant.tech/) for vector search capabilities
- The Rust and React communities for excellent tooling

## 📞 Support

- 📋 [GitHub Issues](https://github.com/nnn-gif/personalassistant/issues) for bug reports
- 💬 [GitHub Discussions](https://github.com/nnn-gif/personalassistant/discussions) for questions
- 📖 [Documentation](INSTALLATION.md) for setup help

---

Made with ❤️ by the Personal Assistant team. Your productivity, your data, your control.