# Vantage

Privacy-first desktop app for real-time meeting transcription with AI-powered summaries, dual audio capture, and calendar auto-start.

Live site: https://vantage-meeting-app.netlify.app · Latest release: **v0.4.0** (macOS universal)

![Platform](https://img.shields.io/badge/Platform-macOS%2012.3%2B-blue)
![License](https://img.shields.io/badge/License-MIT-green)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![TypeScript](https://img.shields.io/badge/TypeScript-5.0+-blue)
![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB)

> **Just want to try it?** Run `curl -fsSL https://vantage-meeting-app.netlify.app/install.sh | bash` or grab the [v0.4.0 DMG](https://github.com/venkateswarisudalai/Vantage/releases/download/v0.4.0/Vantage-0.4.0-universal.dmg). Building from source? Read on.

## Overview

Vantage is a Tauri 2 desktop app (Rust backend, React 19 frontend) that does real-time speech-to-text with sub-2-second latency, separates "You" vs "Participant" via dual-channel audio routing, and can auto-start transcription when a calendar meeting begins.

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
- **macOS Universal** - Single DMG works on Intel and Apple Silicon (macOS 12.3+)

## Screenshots

<p align="center">
  <img src="docs/screenshot-light.png" alt="Light Mode" width="45%">
  <img src="docs/screenshot-dark.png" alt="Dark Mode" width="45%">
</p>

## Run it locally

> Vantage is currently macOS-only (12.3+). The Rust backend uses ScreenCaptureKit + cpal for audio, so Windows/Linux dev hasn't been wired up yet.

### Prerequisites

| Tool | Version | Install |
|---|---|---|
| **Xcode Command Line Tools** | any | `xcode-select --install` |
| **Node.js** | 18+ (tested on 22) | `brew install node` or [nodejs.org](https://nodejs.org/) |
| **Rust** | stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **BlackHole 2ch** *(optional)* | 2.x | `brew install blackhole-2ch` — only needed for dual-channel "You" vs "Participant" audio |

### Three things you can run

```bash
git clone https://github.com/venkateswarisudalai/Vantage.git
cd Vantage
npm install
```

**1. Full desktop app (Tauri + Rust)** — what you ship to users.
```bash
npm run tauri dev      # ~5 min first build, ~10s thereafter
npm run tauri build    # produces a universal DMG in src-tauri/target/
```

**2. Frontend only (Vite, no Rust)** — fast iteration on UI when the Tauri IPC layer is mocked.
```bash
npm run dev            # serves http://localhost:1420
```

**3. End-to-end tests (Playwright against the mocked frontend).**
```bash
npm run test:e2e           # headless
npm run test:e2e:headed    # see the browser
npm run test:e2e:report    # open last HTML report
```

### Compile-time secrets

Google OAuth and Supabase keys are baked into the Rust binary via `env!()`. The defaults live in `.env.build` at the repo root and are auto-loaded by `src-tauri/build.rs` — no shell sourcing needed. To override, set the env vars in your shell before building:

```bash
GOOGLE_CLIENT_ID=… GOOGLE_CLIENT_SECRET=… npm run tauri dev
```

### Per-user API keys

Deepgram and Groq keys are entered through the in-app Settings UI on first launch. Optionally, drop them into a local `.env` for development:

```bash
cp .env.example .env
# then fill in VANTAGE_DEEPGRAM_API_KEY and VANTAGE_GROQ_API_KEY
```

The app should launch automatically on `npm run tauri dev`. If `cargo` is missing, `source $HOME/.cargo/env` (or restart your terminal).

## First-run setup

### Configure API Keys

You only need **2 free API keys** to get started. Calendar integration and cloud sync are built in — no extra configuration needed.

1. **Get API Keys** (both have free tiers):
   - **Deepgram**: Sign up at https://console.deepgram.com — includes $200 free credit
   - **Groq**: Sign up at https://console.groq.com — free tier

2. **Add Keys to App**:
   - Open Vantage app — the welcome screen guides you through both steps
   - Paste your Deepgram API key
   - Paste your Groq API key
   - Click **Save**

### Set Up Dual Audio (Optional)

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

6. **Verify in Vantage**:
   - Start Live Transcription
   - Play a video → should show **"Participant:"**
   - Speak into mic → should show **"You:"**

### Set Up Calendar Auto-Start (Optional)

**Why do this?** Automatically start transcription when your meetings begin. Calendar integration is built in — just click connect.

1. **Connect Google Calendar**:
   - Open Vantage → **Settings**
   - Click **"Connect Calendar"** — your browser opens for Google sign-in
   - Grant calendar permissions and you'll be redirected back

2. **Enable Auto-Start**:
   - Toggle **"Auto-start on meeting time"** to ON
   - **Start buffer time**: How many minutes before meeting to start (default: 2 minutes)
   - **Detect meeting apps**: Auto-detect Zoom, Teams, Google Meet, etc. (recommended: ON)

3. **Test It**:
   - Create a test meeting in Google Calendar (5 minutes from now)
   - Open Zoom/Teams/Meet app
   - Vantage should show "Meeting starting in X minutes"
   - Transcription should auto-start when buffer time is reached

### Grant macOS Permissions

When you first run the app, macOS will ask for permissions:

1. **Microphone Access**: Click **"OK"** to allow
   - Required for transcription
   - Can manage later in: System Settings → Privacy & Security → Microphone

2. **Accessibility** (if using calendar auto-start):
   - System Settings → Privacy & Security → Accessibility
   - Add Vantage and toggle ON

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

You only need **2 free API keys** to get started. Calendar and cloud sync are built in.

| Service | Purpose | Get Key | Free Tier |
|---------|---------|---------|-----------|
| **Deepgram** | Real-time transcription | [console.deepgram.com](https://console.deepgram.com) | $200 credit |
| **Groq** | AI summaries & replies | [console.groq.com/keys](https://console.groq.com/keys) | Free tier |

### Setting Up Keys
1. Open the app — the welcome screen guides you
2. Get your Deepgram key (includes $200 free credit)
3. Get your Groq key (free tier)
4. Paste both in Settings → Start transcribing!

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
vantage/
├── src/                      # React frontend (App.tsx, App.css)
├── src-tauri/                # Rust backend
│   ├── src/
│   │   ├── lib.rs            # Tauri commands & shared state
│   │   ├── deepgram.rs       # Real-time multichannel transcription
│   │   ├── groq.rs           # AI summaries & reply suggestions
│   │   ├── system_audio.rs   # BlackHole audio device detection
│   │   ├── meeting_monitor.rs# Calendar polling & meeting detection
│   │   ├── calendar.rs       # Google Calendar OAuth (env-baked client ID)
│   │   ├── supabase.rs       # Cloud sync (env-baked URL/key)
│   │   ├── database.rs       # SQLite meeting storage
│   │   ├── settings.rs       # Per-user keys & preferences
│   │   └── audio.rs          # Microphone capture
│   ├── build.rs              # macOS link flags + auto-loads ../.env.build
│   └── Cargo.toml
├── e2e/                      # Playwright tests against the mocked frontend
├── playwright.config.ts
├── website/                  # Marketing site (deployed to Netlify)
├── web-app/                  # Standalone web build (separate Vite project)
├── browser-extension/        # Companion browser extension
├── proxy/                    # Optional Cloudflare Worker proxy for demo mode
├── scripts/sign-and-package.sh
├── switch-audio.sh           # BlackHole audio routing helper
├── .env.build                # Compile-time secrets (Google OAuth, Supabase)
├── .env.example              # Template for runtime API keys
└── package.json
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

See [Run it locally](#run-it-locally) above for prerequisites and the three dev workflows (Tauri, Vite-only, Playwright). Before opening a PR, run:

```bash
npm run build       # tsc + vite build (must be clean)
npm run test:e2e    # Playwright suite
(cd src-tauri && cargo check)
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
- [Report bugs](https://github.com/venkateswarisudalai/MeetBetter/issues)
- [Request features](https://github.com/venkateswarisudalai/MeetBetter/issues)

---

<p align="center">
  Made with love using Tauri + React + Rust
</p>
