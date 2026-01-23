# Meeting Assistant

A powerful, privacy-focused desktop application for real-time meeting transcription with AI-powered summaries and smart reply suggestions.

![Meeting Assistant](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-blue)
![License](https://img.shields.io/badge/License-MIT-green)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![TypeScript](https://img.shields.io/badge/TypeScript-5.0+-blue)

## Features

- **Real-time Transcription** - Live speech-to-text using Deepgram (1-2 second latency)
- **Dual Audio Capture** - Separate transcription for "You" (microphone) vs "Participant" (system audio/remote speakers)
- **Calendar Integration** - Auto-start transcription when meetings begin (Google Calendar support)
- **Meeting Detection** - Automatically detects Zoom, Teams, Google Meet, Webex, Slack processes
- **AI-Powered Summaries** - Generate meeting summaries with key points and action items
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
- **[Optional but Recommended]** [BlackHole 2ch](https://github.com/ExistentialAudio/BlackHole) for dual audio capture (macOS only)

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

### Dual Audio Capture (Optional)

**What it does:** Separates "You" (your microphone) from "Participant" (system audio/remote speakers) in transcriptions.

#### macOS Setup:

1. **Install BlackHole 2ch:**
   ```bash
   brew install blackhole-2ch
   ```
   Or download from: https://github.com/ExistentialAudio/BlackHole

2. **For Testing (No Audio Playback):**
   - System Settings → Sound → Output
   - Select **"BlackHole 2ch"**
   - ⚠️ You won't hear audio, but channel separation will work perfectly

3. **For Actual Use (Hear Audio While Recording):**
   - Open **Audio MIDI Setup** app
   - Click **"+"** → **"Create Multi-Output Device"**
   - Check both:
     - ✓ BlackHole 2ch
     - ✓ MacBook Pro Speakers (or your preferred output)
   - System Settings → Sound → Output → Select **"Multi-Output Device"**
   - 💡 Keep speaker volume low to prevent feedback

#### Windows/Linux:
- Windows: Install [VB-Cable](https://vb-audio.com/Cable/) (similar setup)
- Linux: Use PulseAudio loopback

#### Without BlackHole:
✅ App works normally, but all audio shows as "You"

### Calendar Auto-Start

1. Open **Settings** → **Meeting Auto-Start**
2. Enable **"Auto-start on meeting time"**
3. Click **"Connect Calendar"** → Sign in with Google
4. Set start buffer time (default: 2 minutes before meeting)
5. App will automatically start transcribing when meetings begin!

### Generate Summary
1. After transcription, click **"Generate"** in the Summary panel
2. AI will create a concise meeting summary with key points and action items

### Get Reply Suggestions
1. Click **"Generate from Transcript"**
2. Get smart, contextual reply suggestions
3. Click any suggestion to copy it

## Tech Stack

| Layer | Technology |
|-------|------------|
| **Frontend** | React + TypeScript + Vite |
| **Backend** | Rust + Tauri 2.0 |
| **Transcription** | Deepgram (real-time with multichannel), AssemblyAI (batch) |
| **AI/LLM** | Groq (Llama 3.1, Mixtral) |
| **Audio** | cpal (cross-platform audio capture) |
| **Calendar** | Google Calendar OAuth2 integration |
| **Virtual Audio** | BlackHole 2ch (macOS), VB-Cable (Windows) |
| **Styling** | CSS with dark mode support |

## Project Structure

```
meetbetter/
├── src/                       # React frontend
│   ├── App.tsx               # Main React component
│   └── App.css               # Styles
├── src-tauri/                # Rust backend
│   ├── src/
│   │   ├── lib.rs            # Tauri commands & state
│   │   ├── deepgram.rs       # Real-time multichannel transcription
│   │   ├── system_audio.rs   # BlackHole audio device detection
│   │   ├── meeting_monitor.rs # Calendar polling & meeting detection
│   │   ├── calendar.rs       # Google Calendar OAuth integration
│   │   ├── assemblyai.rs     # Batch transcription
│   │   ├── database.rs       # SQLite meeting storage
│   │   └── audio.rs          # Audio recording
│   └── Cargo.toml            # Rust dependencies
├── switch-audio.sh           # Helper script for audio routing
├── package.json              # Node dependencies
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

- [x] Dual audio capture (You vs Participant)
- [x] Calendar integration (Google Calendar)
- [x] Meeting auto-start detection
- [ ] Outlook calendar support
- [ ] Speaker diarization (identify multiple participants)
- [ ] Export to various formats (PDF, Word, Markdown)
- [ ] Meeting templates
- [ ] Keyboard shortcuts
- [ ] Local LLM support (Ollama)
- [ ] Browser extension
- [ ] Mobile companion app
- [ ] Multi-language support
- [ ] Windows/Linux dual audio support

## FAQ

**Q: Is my audio data stored anywhere?**
A: No. Audio is processed in real-time and only the transcription text is sent to APIs. Nothing is stored on external servers.

**Q: Can I use this without internet?**
A: Recording works offline, but transcription and AI features require internet connection.

**Q: Which API should I get first?**
A: Start with Deepgram (for transcription) + Groq (for AI). Both have generous free tiers.

**Q: Do I need BlackHole for the app to work?**
A: No! The app works perfectly without BlackHole. BlackHole is only needed if you want to differentiate between "You" (microphone) and "Participant" (system audio/remote speakers) in transcriptions.

**Q: Why does everything show as "You" in my transcription?**
A: This means BlackHole isn't installed or your audio output isn't set to BlackHole/Multi-Output Device. See the [Dual Audio Capture](#dual-audio-capture-optional) section for setup instructions.

**Q: Can I hear audio while using dual channel capture?**
A: Yes! Create a Multi-Output Device in Audio MIDI Setup that includes both BlackHole and your speakers. See the detailed setup instructions in the [Usage](#usage) section.

**Q: Does calendar auto-start work with Zoom/Teams?**
A: Yes! The app detects when Zoom, Teams, Google Meet, Webex, or Slack processes are running and can auto-start transcription based on your calendar events.

**Q: Will dual audio capture work on Windows/Linux?**
A: Currently, dual audio is macOS-only with BlackHole. Windows users can use VB-Cable with similar setup. Linux support is planned for future releases.

## Troubleshooting

### Dual Audio Issues

**Problem: Everything shows as "You", no "Participant" label**
- ✅ Ensure BlackHole 2ch is installed: `brew install blackhole-2ch`
- ✅ Set System Settings → Sound → Output to "BlackHole 2ch" or "Multi-Output Device"
- ✅ Restart the app after changing audio settings

**Problem: Transcriptions are repeating multiple times**
- ❌ Your audio output is set to speakers, not BlackHole
- ❌ If using Multi-Output Device, speaker volume is too high (mic picks up echo)
- ✅ Switch to BlackHole-only for testing, or lower speaker volume significantly

**Problem: I can't hear any audio**
- This is expected if using BlackHole 2ch only
- ✅ Create a Multi-Output Device (see [Usage](#usage) section)
- ✅ Include both BlackHole 2ch and your speakers in the Multi-Output Device

### Calendar Auto-Start Issues

**Problem: Auto-start not triggering**
- ✅ Check Settings → Enable "Auto-start on meeting time"
- ✅ Ensure Google Calendar is connected
- ✅ Verify meeting app (Zoom, Teams, etc.) is running
- ✅ Check start buffer time setting (default: 2 minutes before meeting)

**Problem: "Not authenticated with Google" error**
- ✅ Click "Connect Calendar" in settings
- ✅ Complete Google OAuth flow
- ✅ Grant calendar read permissions

### General Issues

**Problem: Build fails on macOS**
```bash
# Update Xcode Command Line Tools
xcode-select --install

# Update Rust
rustup update stable
```

**Problem: Microphone not detected**
- ✅ Grant microphone permissions: System Settings → Privacy & Security → Microphone
- ✅ Restart the app

**Problem: Deepgram connection fails**
- ✅ Check your API key in Settings
- ✅ Verify internet connection
- ✅ Check Deepgram API status: https://status.deepgram.com

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
