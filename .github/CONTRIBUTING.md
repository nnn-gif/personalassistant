# Contributing to Personal Assistant

Thank you for your interest in contributing to Personal Assistant! This document provides guidelines and information for contributors.

## Development Setup

### Prerequisites
- Node.js 18+ 
- Rust 1.70+
- Git

### Local Development
1. **Clone the repository**
   ```bash
   git clone https://github.com/nnn-gif/personalassistant.git
   cd personalassistant
   ```

2. **Install dependencies**
   ```bash
   npm install
   ```

3. **Install Ollama and models**
   ```bash
   curl -fsSL https://ollama.ai/install.sh | sh
   ollama pull nomic-embed-text
   ollama pull llama3.2:1b
   ```

4. **Run in development mode**
   ```bash
   npm run tauri:dev
   ```

## Code Style

### Frontend (TypeScript/React)
- Use TypeScript for all new code
- Follow existing component patterns
- Use Tailwind CSS for styling
- Add proper error handling

### Backend (Rust)
- Follow Rust naming conventions
- Add comprehensive error handling
- Include unit tests for new features
- Use `cargo fmt` and `cargo clippy`

### Commit Messages
Use conventional commits format:
```
feat: add new document processor
fix: resolve PDF encoding issues
docs: update installation guide
```

## Pull Request Process

1. **Fork the repository** and create a feature branch
2. **Make your changes** following the code style guidelines
3. **Add tests** for new functionality
4. **Update documentation** if needed
5. **Run tests locally**:
   ```bash
   npm run build
   cargo test --manifest-path=src-tauri/Cargo.toml
   cargo clippy --manifest-path=src-tauri/Cargo.toml
   ```
6. **Submit a pull request** with a clear description

## Feature Guidelines

### Privacy-First Development
- All data must stay local to the user's device
- No external API calls without explicit user consent
- Clear documentation of data storage and usage

### Performance Considerations
- Optimize for resource usage (RAM, CPU, disk)
- Test on lower-end hardware when possible
- Consider battery impact on laptops

### Cross-Platform Compatibility
- Test on macOS, Windows, and Linux
- Use platform-specific code sparingly
- Document platform-specific requirements

## Issue Reporting

- Use the provided issue templates
- Include system information and logs
- Provide clear reproduction steps
- Check for existing issues first

## Documentation

- Update relevant documentation for new features
- Include code comments for complex logic
- Update the README if needed
- Add examples for new APIs

## Security

- Report security issues privately via email
- Don't include sensitive data in examples
- Follow secure coding practices
- Validate all user inputs

## Getting Help

- Check existing documentation
- Search closed issues for solutions
- Join discussions in GitHub Discussions
- Tag maintainers for urgent issues

## Code of Conduct

- Be respectful and inclusive
- Help others learn and contribute
- Focus on constructive feedback
- Maintain professional communication

Thank you for contributing to Personal Assistant! 🚀