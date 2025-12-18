# Contributing to Meeting Assistant

First off, thank you for considering contributing to Meeting Assistant! It's people like you that make this tool better for everyone.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check existing issues to avoid duplicates.

**When reporting a bug, include:**
- Your operating system (macOS, Windows, Linux)
- Node.js and Rust versions
- Steps to reproduce the issue
- Expected vs actual behavior
- Screenshots if applicable
- Error messages from the console

### Suggesting Features

Feature suggestions are welcome! Please:
- Check if the feature is already in the roadmap
- Describe the feature and its use case
- Explain why it would be useful to most users

### Pull Requests

1. Fork the repo and create your branch from `main`
2. If you've added code, add tests if applicable
3. Ensure the code compiles without errors
4. Update documentation if needed
5. Make sure your code follows the existing style

## Development Setup

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js (v18+)
# Use nvm or download from nodejs.org

# Verify installations
rustc --version
node --version
npm --version
```

### Getting Started

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/meeting-assistant.git
cd meeting-assistant

# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

### Project Structure

```
meeting-assistant/
├── src/                    # React frontend (TypeScript)
│   ├── App.tsx            # Main component
│   └── App.css            # Styles
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── lib.rs         # Main Tauri commands
│   │   ├── deepgram.rs    # Deepgram integration
│   │   ├── assemblyai.rs  # AssemblyAI integration
│   │   └── audio.rs       # Audio recording
│   └── Cargo.toml         # Rust dependencies
└── package.json           # Node dependencies
```

### Code Style

**Rust:**
- Run `cargo fmt` before committing
- Run `cargo clippy` to check for issues
- Follow Rust naming conventions

**TypeScript/React:**
- Use functional components with hooks
- Use TypeScript types (avoid `any`)
- Keep components focused and small

### Testing

```bash
# Rust tests
cd src-tauri
cargo test

# Frontend (if tests exist)
npm test
```

### Building

```bash
# Development build
npm run tauri dev

# Production build
npm run tauri build
```

## Commit Messages

Use clear, descriptive commit messages:

```
feat: add speaker diarization support
fix: resolve audio capture on Windows
docs: update API setup instructions
refactor: simplify transcription handling
```

## Need Help?

- Open an issue with your question
- Tag it with `question` label

## Recognition

Contributors will be recognized in the README and release notes.

Thank you for contributing!
