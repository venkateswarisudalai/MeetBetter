# MeetBetter

A powerful, privacy-focused desktop application for real-time meeting transcription with AI-powered summaries, dual audio capture, and calendar integration.

![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-blue)
![License](https://img.shields.io/badge/License-MIT-green)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![TypeScript](https://img.shields.io/badge/TypeScript-5.0+-blue)
![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB)

> **Want to test this app?** See [TESTING.md](TESTING.md) for a quick 5-minute setup guide!

## Overview

MeetBetter is a production-ready desktop application built with **Tauri 2.0**, **Rust**, and **React** that transforms how meetings are transcribed and managed. It features real-time speech-to-text with sub-2-second latency, intelligent speaker separation through dual-channel audio processing, and calendar-driven automation.

**Key Innovation:** Dual audio capture technology that differentiates between your microphone and system audio in real-time, solving the common problem of "who said what" in virtual meetings.

**Tech Stack:** Rust (backend), React + TypeScript (frontend), Tauri 2.0 (framework), Deepgram API (transcription), Groq API (AI), SQLite (storage), WebSockets (real-time streaming)

## Features

- **Real-time Transcription** - Live speech-to-text using Deepgram (1-2 second latency)
- **Dual Audio Capture** - Separate transcription for "You" (microphone) vs "Participant" (system audio/remote speakers)
  - Uses BlackHole virtual audio device for multichannel routing
  - Prevents duplicate transcriptions with intelligent deduplication
- **Calendar Integration** - Auto-start transcription when meetings begin (Google Calendar OAuth)
- **Meeting Detection** - Automatically detects Zoom, Teams, Google Meet, Webex, Slack processes
- **AI-Powered Summaries** - Generate meeting summaries with key points, action items, and decisions
- **Smart Reply Suggestions** - Get contextual reply suggestions based on the conversation
- **Meeting Management** - Save, search, and review past meetings with full transcripts
- **Privacy First** - Your audio stays on your device, only transcription text is sent to APIs
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
git clone https://github.com/venkateswarisudalai/MeetBetter.git
cd MeetBetter

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Complete Setup Guide

### Step 1: Install System Dependencies

#### macOS:
```bash
# Install Homebrew (if not already installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Node.js
brew install node

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Optional: Install BlackHole for dual audio capture
brew install blackhole-2ch
```

#### Windows:
```powershell
# Install Node.js from https://nodejs.org/

# Install Rust
# Download and run: https://win.rustup.rs/

# Optional: Install VB-Cable for dual audio capture
# Download from: https://vb-audio.com/Cable/
```

#### Linux:
```bash
# Install Node.js (Ubuntu/Debian)
sudo apt update
sudo apt install nodejs npm

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install build dependencies
sudo apt install libwebkit2gtk-4.0-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
```

### Step 2: Clone and Build

```bash
# Clone the repository
git clone https://github.com/venkateswarisudalai/MeetBetter.git
cd MeetBetter

# Install JavaScript dependencies
npm install

# Build and run in development mode
npm run tauri dev
```

The app should launch automatically! 🚀

### Step 3: Configure API Keys

1. **Get API Keys** (both have free tiers):
   - **Deepgram**: Sign up at https://console.deepgram.com
     - Navigate to API Keys → Create New Key
     - Copy the key
   - **Groq**: Sign up at https://console.groq.com
     - Go to API Keys → Create API Key
     - Copy the key

2. **Add Keys to App**:
   - Open MeetBetter app
   - Click **Settings** icon (gear icon in top right)
   - Paste your Deepgram API key
   - Paste your Groq API key
   - Click **Save**

### Step 4: Set Up Dual Audio (Optional)

**Why do this?** Separates "You" (microphone) from "Participant" (system audio/remote speakers) in transcriptions.

#### Option A: BlackHole Only (Testing - No Audio Playback)

```bash
# macOS - Install BlackHole
brew install blackhole-2ch

# Set audio output
# System Settings → Sound → Output → Select "BlackHole 2ch"
```

⚠️ **Note:** You won't hear audio with this setup, but channel separation will work perfectly for testing.

#### Option B: Multi-Output Device (Recommended - Hear Audio)

1. **Install BlackHole** (if not already):
   ```bash
   brew install blackhole-2ch
   ```

2. **Create Multi-Output Device**:
   - Open **Audio MIDI Setup** app (in /Applications/Utilities/)
   - Click the **"+"** button at bottom left
   - Select **"Create Multi-Output Device"**
   - In the right panel, check **both**:
     - ✓ **BlackHole 2ch**
     - ✓ **MacBook Pro Speakers** (or your output device)
   - Optional: Right-click the Multi-Output Device → "Use This Device For Sound Output"

3. **Set System Output**:
   - Open **System Settings** → **Sound** → **Output**
   - Select **"Multi-Output Device"**

4. **Adjust Volume**:
   - Keep speaker volume **low to medium** (prevents microphone from picking up speaker audio)
   - For best results during real meetings, use **headphones** instead

5. **Test It**:
   ```bash
   # Run the included test script
   ./switch-audio.sh

   # Or manually test
   say "This is participant audio" &
   # Then speak into your mic
   ```

6. **Verify in MeetBetter**:
   - Start Live Transcription
   - Play a video → should show **"Participant:"**
   - Speak into mic → should show **"You:"**

### Step 5: Set Up Calendar Auto-Start (Optional)

**Why do this?** Automatically start transcription when your meetings begin.

1. **Enable Feature**:
   - Open MeetBetter → **Settings**
   - Scroll to **"Meeting Auto-Start"** section
   - Toggle **"Auto-start on meeting time"** to ON

2. **Connect Google Calendar**:
   - Click **"Connect Calendar"** button
   - Sign in with your Google account
   - Grant calendar read permissions
   - You'll be redirected back to the app

3. **Configure Settings**:
   - **Start buffer time**: How many minutes before meeting to start (default: 2 minutes)
   - **Detect meeting apps**: Auto-detect Zoom, Teams, Google Meet, etc. (recommended: ON)
   - **Auto-start on time**: Start transcription automatically (recommended: ON)

4. **Test It**:
   - Create a test meeting in Google Calendar (5 minutes from now)
   - Open Zoom/Teams/Meet app
   - MeetBetter should show "Meeting starting in X minutes"
   - Transcription should auto-start when buffer time is reached

### Step 6: Grant Permissions (macOS)

When you first run the app, macOS will ask for permissions:

1. **Microphone Access**: Click **"OK"** to allow
   - Required for transcription
   - Can manage later in: System Settings → Privacy & Security → Microphone

2. **Accessibility** (if using calendar auto-start):
   - System Settings → Privacy & Security → Accessibility
   - Add MeetBetter and toggle ON

### Troubleshooting Setup

**Build fails with "xcrun: error"** (macOS):
```bash
xcode-select --install
```

**Rust not found**:
```bash
source $HOME/.cargo/env
# Or restart your terminal
```

**Node version too old**:
```bash
# macOS
brew upgrade node

# Or use nvm
nvm install 18
nvm use 18
```

**Can't hear audio with Multi-Output**:
- Verify both devices are checked in Audio MIDI Setup
- Check System Settings → Sound → Output shows "Multi-Output Device"
- Increase speaker volume slightly

**Dual audio not working**:
```bash
# Verify BlackHole is installed
ls /Library/Audio/Plug-Ins/HAL/BlackHole2ch.driver

# If missing, reinstall
brew reinstall blackhole-2ch

# Restart Mac after installation
sudo reboot
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
