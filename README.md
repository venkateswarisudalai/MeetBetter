# Vantage

Privacy-first desktop app for real-time meeting transcription with AI-powered summaries, dual audio capture, and calendar auto-start.

Live site: https://vantage-meeting-app.netlify.app · Latest release: **v0.4.0** (macOS universal)

![Platform](https://img.shields.io/badge/Platform-macOS%2012.3%2B-blue)
![License](https://img.shields.io/badge/License-MIT-green)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![TypeScript](https://img.shields.io/badge/TypeScript-5.0+-blue)
![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB)

> **Just want to try it?** Run `curl -fsSL https://vantage-meeting-app.netlify.app/install.sh | bash` or grab the [v0.4.0 DMG](https://github.com/venkateswarisudalai/MeetBetter/releases/download/v0.4.0/Vantage-0.4.0-universal.dmg). Step-by-step install + permissions guide: [**TESTER_SETUP.md**](TESTER_SETUP.md).
>
> **No install, any computer:** open [meetbetter-app.netlify.app](https://meetbetter-app.netlify.app) in Chrome or Edge. See [Run it in the browser](#run-it-in-the-browser).
>
> **Building from source?** Keep reading, or jump to [TESTING.md](TESTING.md) for the developer test plan.

## Overview

Vantage is a Tauri 2 desktop app (Rust backend, React 19 frontend) that does real-time speech-to-text with sub-2-second latency, separates "You" vs "Participant" via dual-channel audio routing, and can auto-start transcription when a calendar meeting begins.

**Key Innovation:** Dual audio capture technology that differentiates between your microphone and system audio in real-time, solving the common problem of "who said what" in virtual meetings.

**Tech Stack:** Rust (backend), React + TypeScript (frontend), Tauri 2.0 (framework), Deepgram API (transcription), Groq API (AI), SQLite (storage), WebSockets (real-time streaming)

## Status: what works and what's next

| | Status | Notes |
|---|---|---|
| **Desktop app** (macOS) | ✅ Works | Live transcription, You vs Participant (with BlackHole), calendar auto-start, meeting-app detection, summaries, reply suggestions, saved meetings |
| **AI summaries and suggestions** | ✅ Fixed | Groq now uses `openai/gpt-oss-120b` (or `gpt-oss-20b`, faster). Groq retired the old Llama, Mixtral, and Gemma models on 2026-08-16; a saved retired model switches to the new default automatically |
| **Local models (Ollama)** | ✅ Works (desktop) | Settings → *AI for summaries & suggestions* → **Ollama (on this Mac)**. Transcripts never leave the Mac and no Groq key is needed. See [Local AI with Ollama](#local-ai-with-ollama) |
| **Windows / Linux desktop** | 🧪 Builds, mic only | GitHub Actions builds Windows (`.msi`, `.exe`) and Linux (`.deb`, `.AppImage`) installers on every push. They transcribe your microphone; hearing the other side of a call is macOS-only for now. Not yet tried on a real PC |
| **Web app** (any computer) | ✅ Works | [meetbetter-app.netlify.app](https://meetbetter-app.netlify.app): live transcription, tab + mic capture, summary, "ask about this meeting", history in the browser. See [Run it in the browser](#run-it-in-the-browser) |
| **Browser extension** | 🧪 Prototype | Detects Meet, Zoom, and Teams tabs. Load it unpacked from `browser-extension/`; not in the Chrome Web Store |

### Known issues

- **Windows and Linux hear only the microphone.** Everything shows as "You" there until system-audio capture is added (see below).
- **Builds without the Google/Supabase keys** (forks, CI, a fresh clone without `.env.build`) work, but Google Calendar and cloud sync are turned off in that build. See [Compile-time secrets](#compile-time-secrets).

### What's next

1. Try the Windows and Linux installers on real machines (download them from a [Desktop build](https://github.com/venkateswarisudalai/MeetBetter/actions/workflows/desktop-build.yml) run's **Artifacts**).
2. Hear the other side of calls on Windows (WASAPI loopback, no driver needed) and Linux (PipeWire/PulseAudio monitor source).
3. Local models in the web app.
4. Publish the browser extension.
5. Deploy the web app automatically on merge (today it's deployed by hand: `cd web-app && npm run build`, then publish `web-app/dist` to Netlify).

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

> The full experience is on macOS (12.3+). Windows and Linux build and run with microphone-only transcription; see [Windows and Linux](#windows-and-linux).

### Prerequisites

| Tool | Version | Install |
|---|---|---|
| **Xcode Command Line Tools** | any | `xcode-select --install` |
| **Node.js** | 18+ (tested on 22) | `brew install node` or [nodejs.org](https://nodejs.org/) |
| **Rust** | stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **BlackHole 2ch** *(optional)* | 2.x | `brew install blackhole-2ch` — only needed for dual-channel "You" vs "Participant" audio |
| **Ollama** *(optional)* | any | [ollama.com/download](https://ollama.com/download) — only for running the AI on your own Mac |

### Three things you can run

```bash
git clone https://github.com/venkateswarisudalai/MeetBetter.git
cd MeetBetter
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

A fresh clone builds as-is: the patched `screencapturekit` crate is vendored in `src-tauri/screencapturekit-patch/` (see its `PATCH.md`).

### Compile-time secrets

Google OAuth and Supabase keys are baked into the Rust binary via `env!()`. They're read from `.env.build` at the repo root (not committed) or from your shell, and auto-loaded by `src-tauri/build.rs`:

```bash
GOOGLE_CLIENT_ID=… GOOGLE_CLIENT_SECRET=… SUPABASE_URL=… SUPABASE_ANON_KEY=… npm run tauri dev
```

Without them the build still succeeds, with a warning, and Google Calendar and cloud sync are turned off in that build. In GitHub Actions, add them as repository secrets with the same names.

### Windows and Linux

Install Node.js and Rust as above. On Linux, also install Tauri's system libraries:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev libssl-dev patchelf
```

Then `npm run tauri dev` or `npm run tauri build`. Every push also builds installers in GitHub Actions ([Desktop build](https://github.com/venkateswarisudalai/MeetBetter/actions/workflows/desktop-build.yml) → a run → **Artifacts**). On both, the app transcribes your microphone only for now.

### Per-user API keys

Deepgram and Groq keys are entered through the in-app Settings UI on first launch. Optionally, drop them into a local `.env` for development:

```bash
cp .env.example .env
# then fill in VANTAGE_DEEPGRAM_API_KEY and VANTAGE_GROQ_API_KEY
```

The app should launch automatically on `npm run tauri dev`. If `cargo` is missing, `source $HOME/.cargo/env` (or restart your terminal).

## Local AI with Ollama

Run summaries and suggestions on your own Mac instead of Groq: transcripts never leave it, and you don't need a Groq key. Transcription itself still uses Deepgram.

1. Install [Ollama](https://ollama.com/download) and start it (`ollama serve`, or open the app).
2. Download a model:
   - **16 GB+ of memory:** `ollama pull qwen3:14b` (recommended, about 9 GB)
   - **Smaller machines:** `ollama pull llama3.2` (2 GB; works, but summaries are more generic)
3. In MeetBetter, open **Settings → AI for summaries & suggestions → Ollama (on this Mac)**. Your installed models appear in the list, with the best one preselected; **Refresh** picks up new ones.

MeetBetter turns off Ollama's "thinking" mode so replies come back in seconds instead of about a minute with `qwen3`, and sizes the context window to the meeting so long transcripts aren't cut off. A `qwen3:14b` summary takes about 15–20 seconds on an M1 Pro, including loading the model.

## Run it in the browser

The web app is a separate build in `web-app/` with no Rust. It runs in **Chrome or Edge** on Windows, Linux, ChromeOS, or macOS. Firefox and Safari can't share tab audio.

**Use the hosted version:**

1. Open [meetbetter-app.netlify.app](https://meetbetter-app.netlify.app).
2. In Settings, paste a **Deepgram** key ([console.deepgram.com](https://console.deepgram.com), free credit) for transcription and a **Groq** key ([console.groq.com/keys](https://console.groq.com/keys), free) for summaries. Keys stay in your browser's local storage and go only to Deepgram and Groq.
3. Under **Audio source**, choose what to hear:
   - **Tab audio:** a call or video playing in another Chrome tab (Google Meet, Teams or Zoom on the web, YouTube).
   - **Mic only:** in-person meetings.
   - **Mic + Tab:** a call, with your own voice on the mic.
4. Press **Start Meeting**. With Tab audio or Mic + Tab, Chrome asks what to share: pick **Chrome Tab**, choose the tab with the call or video, and tick **Share tab audio**. On macOS, sharing a window or the entire screen gives **no** audio; it has to be a tab.
5. Stop when you're done. The AI summary is written automatically.

**Run it from source:**

```bash
cd web-app
npm ci
npm run dev          # http://localhost:5173
npm run test:e2e     # Playwright tests
npm run build        # static files in web-app/dist, deployable to Netlify or GitHub Pages
```

AI summaries in the web app use Groq (`openai/gpt-oss-120b`). Local models aren't available in the web app yet.

## First-run setup

### Configure API Keys

You only need **2 free API keys** to get started. Calendar integration and cloud sync are built in — no extra configuration needed.

1. **Get API Keys** (both have free tiers):
   - **Deepgram**: Sign up at https://console.deepgram.com — free credit
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
Not supported yet: the desktop app hears only the microphone there. See [What's next](#whats-next).

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
| **Transcription** | Deepgram `nova-3` (real-time, multichannel), AssemblyAI (batch) |
| **AI/LLM** | Groq (`openai/gpt-oss-120b`, `gpt-oss-20b`) or local Ollama (`qwen3:14b`, `llama3.2`, …) |
| **Audio** | cpal (cross-platform audio capture) |
| **Calendar** | Google Calendar OAuth2 integration |
| **Virtual Audio** | BlackHole 2ch (macOS) |
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
│   │   ├── ollama.rs         # Local AI through Ollama
│   ├── screencapturekit-patch/ # Vendored screencapturekit 1.5.0 + one build fix (PATCH.md)
│   ├── build.rs              # macOS link flags + loads ../.env.build (optional)
│   └── Cargo.toml
├── e2e/                      # Playwright tests against the mocked frontend
├── playwright.config.ts
├── website/                  # Marketing site (deployed to Netlify)
├── web-app/                  # Standalone web build (separate Vite project)
├── browser-extension/        # Companion browser extension
├── proxy/                    # Optional Cloudflare Worker proxy for demo mode
├── scripts/sign-and-package.sh
├── switch-audio.sh           # BlackHole audio routing helper
├── .github/workflows/        # Desktop build: tests + Windows/Linux installers
├── .env.build                # Compile-time secrets (Google OAuth, Supabase); not committed
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
- [x] Web app for any computer (`web-app/`)
- [x] Browser extension (prototype, load unpacked)
- [x] Move off the retired Groq models
- [x] Windows and Linux builds (microphone only)
- [ ] Outlook calendar support
- [ ] Speaker diarization (identify multiple participants)
- [ ] Export to various formats (PDF, Word, Markdown)
- [ ] Meeting templates
- [ ] Keyboard shortcuts
- [x] Local LLM support (Ollama) in the desktop app
- [ ] Publish the browser extension to the Chrome Web Store
- [ ] Mobile companion app
- [ ] Multi-language support
- [ ] Windows/Linux dual audio support (hear the other side of calls)

## FAQ

**Q: Is my audio data stored anywhere?**
A: No. Audio is processed in real-time and only the transcription text is sent to APIs. Nothing is stored on external servers.

**Q: Can I use this without internet?**
A: Recording works offline. Transcription needs the internet (Deepgram). AI summaries and suggestions can run offline with [Ollama](#local-ai-with-ollama).

**Q: Can I run the AI on my own computer?**
A: Yes, in the desktop app: choose **Ollama (on this Mac)** in Settings. See [Local AI with Ollama](#local-ai-with-ollama).

**Q: Does it work on Windows or Linux?**
A: The desktop app builds for both and transcribes your microphone; hearing the other side of a call is macOS-only for now. The [web app](#run-it-in-the-browser) works in Chrome or Edge on any computer and can hear a shared tab.

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
A: Not yet. On Windows and Linux the desktop app hears only your microphone, so everything shows as "You". Capturing what's playing (WASAPI loopback on Windows, the PipeWire/PulseAudio monitor on Linux) is next on the roadmap. Meanwhile, the [web app](#run-it-in-the-browser) can hear a call in a shared Chrome tab on any OS.

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
