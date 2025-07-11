# Personal Assistant

An AI-powered productivity assistant that combines comprehensive activity tracking, intelligent research capabilities, and local AI processing to help users achieve peak productivity while maintaining complete data privacy.

## Features

- **Browser AI Research Assistant**: Advanced web research tool with intelligent query splitting, parallel web scraping, and AI-powered result synthesis
- **Comprehensive Activity Tracking**: Monitor application usage, window titles, and system events with real-time updates
- **Goal Management System**: Create and track productivity goals with allowed applications and time-based targets
- **Privacy-First Architecture**: All data stored and processed locally with no cloud uploads
- **AI-Powered Insights**: Local LLM integration for productivity scoring and personalized recommendations
- **Interactive Dashboards**: Real-time visualizations of productivity metrics and goal progress

## Tech Stack

- **Backend**: Rust with Tauri 2
- **Frontend**: React + TypeScript + Vite + Tailwind CSS
- **Database**: SurrealDB (embedded)
- **AI**: Ollama (local LLM) via genai-rust
- **Web Scraping**: Playwright
- **UI Components**: Framer Motion, Recharts, Lucide Icons

## Prerequisites

- Rust (latest stable)
- Node.js 18+
- npm or pnpm
- Ollama installed and running locally
- macOS (for activity tracking features)

## Installation

1. Clone the repository:
```bash
git clone https://github.com/nnn-gif/personalassistant.git
cd personalassistant
```

2. Install frontend dependencies:
```bash
npm install
```

3. Install Rust dependencies:
```bash
cd src-tauri
cargo build
```

4. Start Ollama with a model:
```bash
ollama pull llama3.2
ollama serve
```

## Development

Run the development server:
```bash
npm run tauri:dev
```

## Building

Build for production:
```bash
npm run tauri build
```

## Architecture

The application follows a modular architecture:

- **Activity Tracking**: Platform-specific implementations for monitoring user activity
- **Browser AI**: Playwright-based web scraping with LLM-powered analysis
- **Goal Management**: Session-based goal tracking with progress monitoring
- **LLM Integration**: Abstracted LLM client supporting multiple providers
- **Services**: Tauri commands exposing backend functionality to frontend

## Privacy

- All data is stored locally on your device
- No telemetry or analytics
- LLM inference happens on-device via Ollama
- Web scraping respects robots.txt

## License

MIT