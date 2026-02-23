# Creating a GitHub Release

Since the `gh` CLI requires re-authentication, here's how to manually create a GitHub Release with the built DMG file.

## Steps to Create the Release

### 1. Navigate to GitHub Releases

Go to: https://github.com/venkateswarisudalai/MeetBetter/releases/new

### 2. Fill in Release Details

**Tag:** `v0.1.0` (already pushed to GitHub)

**Release Title:** `MeetBetter v0.1.0 - Initial Release`

**Description:** Copy the content below:

```markdown
# MeetBetter v0.1.0 🎉

First production release of MeetBetter - a powerful desktop app for real-time meeting transcription with AI-powered summaries.

## ✨ Features

- **Real-time Transcription** - Live speech-to-text using Deepgram (1-2 second latency)
- **Dual Audio Capture** - Separate transcription for "You" (microphone) vs "Participant" (system audio)
  - Uses BlackHole virtual audio device for multichannel routing
  - Intelligent deduplication prevents duplicate transcriptions
- **Calendar Integration** - Auto-start transcription when meetings begin (Google Calendar OAuth)
- **Meeting Detection** - Automatically detects Zoom, Teams, Google Meet, Webex, Slack
- **AI-Powered Summaries** - Generate meeting summaries with key points, action items, and decisions
- **Smart Reply Suggestions** - Get contextual reply suggestions based on conversation
- **Meeting Management** - Save, search, and review past meetings with full transcripts
- **Privacy First** - Your audio stays on your device, only transcription text is sent to APIs

## 📦 Installation

### macOS (Apple Silicon)

1. Download `MeetBetter_0.1.0_macOS_Apple_Silicon.dmg`
2. Open the DMG and drag MeetBetter to Applications
3. Right-click → Open (first time only, to bypass Gatekeeper)

**Note:** You may need to go to System Settings → Privacy & Security → Allow if blocked

## 🚀 Quick Start

See [TESTING.md](https://github.com/venkateswarisudalai/MeetBetter/blob/feature/audio-enable/TESTING.md) for a comprehensive 5-minute setup guide.

### Prerequisites
- Free API keys from [Deepgram](https://console.deepgram.com/signup) and [Groq](https://console.groq.com/keys)
- macOS permissions for Microphone and Screen Recording

### Basic Usage
1. Launch MeetBetter
2. Click Settings (⚙️) and add your API keys
3. Click "Start Live Transcription"
4. Speak into your microphone
5. Click "Stop Transcription" and "Generate" for AI summary

## 🔧 Tech Stack

- **Framework:** Tauri 2.0
- **Backend:** Rust
- **Frontend:** React + TypeScript
- **APIs:** Deepgram (transcription), Groq (AI)
- **Storage:** SQLite
- **Real-time:** WebSockets

## 📖 Documentation

- [Testing Guide](https://github.com/venkateswarisudalai/MeetBetter/blob/feature/audio-enable/TESTING.md)
- [README](https://github.com/venkateswarisudalai/MeetBetter/blob/feature/audio-enable/README.md)

## 🐛 Known Issues

- DMG signature not yet configured (use right-click → Open on first launch)
- Windows/Linux builds coming soon
- Intel Mac build coming soon

## 💬 Feedback

Found a bug or have a feature request? [Open an issue](https://github.com/venkateswarisudalai/MeetBetter/issues)!

---

Built with ❤️ using Tauri, Rust, and React
```

### 3. Upload the DMG File

Click "Attach binaries by dropping them here or selecting them."

Upload this file:
```
/Users/vigneshsubbiah/Documents/MeetBetter/src-tauri/target/release/bundle/macos/MeetBetter_0.1.0_macOS_Apple_Silicon.dmg
```

### 4. Publish Release

- Check "Set as the latest release"
- Click "Publish release"

## After Publishing

Once the release is live, you can update the README with a download badge:

```markdown
[![Download](https://img.shields.io/github/v/release/venkateswarisudalai/MeetBetter?label=Download&style=for-the-badge)](https://github.com/venkateswarisudalai/MeetBetter/releases/latest)
```

## For Testers

Share this link with anyone you want to test the app:

**Testing Guide:** https://github.com/venkateswarisudalai/MeetBetter/blob/feature/audio-enable/TESTING.md

**Download:** https://github.com/venkateswarisudalai/MeetBetter/releases/latest

They can download the DMG and follow the TESTING.md guide to get started in 5 minutes!
