# Meeting Assistant

A powerful, privacy-focused desktop application for real-time meeting transcription with AI-powered summaries and smart reply suggestions.

![Meeting Assistant](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-blue)
![License](https://img.shields.io/badge/License-MIT-green)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![TypeScript](https://img.shields.io/badge/TypeScript-5.0+-blue)

## Features

- **Real-time Transcription** - Live speech-to-text using Deepgram (1-2 second latency)
- **AI-Powered Summaries** - Generate meeting summaries with one click
- **Smart Reply Suggestions** - Get contextual reply suggestions based on the conversation
- **Offline Recording** - Record meetings for later transcription
- **Privacy First** - Your audio stays on your device, only transcription is sent to APIs
- **Beautiful UI** - Modern, responsive interface with dark mode support
- **Cross-Platform** - Works on macOS, Windows, and Linux

## Screenshots

<p align="center">
  <img src="docs/screenshot-light.png" alt="Light Mode" width="45%">
  <img src="docs/screenshot-dark.png" alt="Dark Mode" width="45%">
</p>

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) (v18 or higher)
- [Rust](https://rustup.rs/) (latest stable)
- API Keys (see [API Setup](#api-setup))

### Installation

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/meeting-assistant.git
cd meeting-assistant

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## API Setup

You'll need API keys from the following services:

| Service | Purpose | Get Key | Free Tier |
|---------|---------|---------|-----------|
| **Deepgram** | Real-time transcription | [console.deepgram.com](https://console.deepgram.com) | $200 credit |
| **Groq** | AI summaries & replies | [console.groq.com/keys](https://console.groq.com/keys) | Free tier |
| **AssemblyAI** (optional) | Batch transcription | [assemblyai.com/app](https://www.assemblyai.com/app) | 100 hrs/month |

### Minimum Setup
- **Deepgram + Groq** = Full real-time experience with AI features

### Setting Up Keys
1. Open the app
2. Click **Settings** in the header
3. Enter your API keys
4. Start transcribing!

## Usage

### Live Transcription
1. Click **"Start Live Transcription"**
2. Speak into your microphone
3. Watch real-time transcription appear
4. Click **"Stop"** when done

### Generate Summary
1. After transcription, click **"Generate"** in the Summary panel
2. AI will create a concise meeting summary

### Get Reply Suggestions
1. Click **"Generate from Transcript"**
2. Get smart, contextual reply suggestions
3. Click any suggestion to copy it

## Tech Stack

| Layer | Technology |
|-------|------------|
| **Frontend** | React + TypeScript + Vite |
| **Backend** | Rust + Tauri 2.0 |
| **Transcription** | Deepgram (real-time), AssemblyAI (batch) |
| **AI/LLM** | Groq (Llama 3.1, Mixtral) |
| **Audio** | cpal (cross-platform audio capture) |
| **Styling** | CSS with dark mode support |

## Project Structure

```
meeting-assistant/
├── src/                    # React frontend
│   ├── App.tsx            # Main React component
│   └── App.css            # Styles
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── lib.rs         # Tauri commands & state
│   │   ├── deepgram.rs    # Real-time transcription
│   │   ├── assemblyai.rs  # Batch transcription
│   │   └── audio.rs       # Audio recording
│   └── Cargo.toml         # Rust dependencies
├── package.json           # Node dependencies
└── README.md
```

## Contributing

Contributions are welcome! Here's how you can help:

### Ways to Contribute
- Report bugs
- Suggest features
- Submit pull requests
- Improve documentation
- Share the project

### Development Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and setup
git clone https://github.com/YOUR_USERNAME/meeting-assistant.git
cd meeting-assistant
npm install

# Run development server
npm run tauri dev
```

### Pull Request Process

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Roadmap

- [ ] Speaker diarization (identify who is speaking)
- [ ] Export to various formats (PDF, Word, Markdown)
- [ ] Meeting templates
- [ ] Calendar integration (Google, Outlook)
- [ ] Keyboard shortcuts
- [ ] Local LLM support (Ollama)
- [ ] Browser extension
- [ ] Mobile companion app
- [ ] Multi-language support

## FAQ

**Q: Is my audio data stored anywhere?**
A: No. Audio is processed in real-time and only the transcription text is sent to APIs. Nothing is stored on external servers.

**Q: Can I use this without internet?**
A: Recording works offline, but transcription and AI features require internet connection.

**Q: Which API should I get first?**
A: Start with Deepgram (for transcription) + Groq (for AI). Both have generous free tiers.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Tauri](https://tauri.app/) - Desktop framework
- [Deepgram](https://deepgram.com/) - Real-time transcription
- [Groq](https://groq.com/) - Fast LLM inference
- [AssemblyAI](https://www.assemblyai.com/) - Batch transcription

## Support

- Star this repo if you find it useful!
- [Report bugs](https://github.com/YOUR_USERNAME/meeting-assistant/issues)
- [Request features](https://github.com/YOUR_USERNAME/meeting-assistant/issues)

---

<p align="center">
  Made with love using Tauri + React + Rust
</p>
