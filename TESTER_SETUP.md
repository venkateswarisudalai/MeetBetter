# Vantage — Tester Setup Guide

Real-time meeting transcription with AI summaries. 2 minutes to set up.

---

## Step 1: Install the App

1. Open **Vantage_0.2.0_aarch64.dmg**
2. Drag **Vantage** to your Applications folder
3. **First launch:** Right-click Vantage.app > "Open" (required because the app isn't notarized)
   - If you see "cannot be opened because the developer cannot be verified":
     - Go to **System Settings > Privacy & Security**
     - Scroll down and click **"Open Anyway"** next to the Vantage message
     - Or run in Terminal: `xattr -cr /Applications/Vantage.app`

## Step 2: Get 2 Free API Keys (takes ~2 minutes)

You only need **2 free API keys** — everything else (calendar, cloud sync) is built in.

### Key 1: Deepgram (real-time transcription)
1. Go to https://console.deepgram.com
2. Sign up (free — includes **$200 credit**)
3. Go to **API Keys** > **Create a New API Key**
4. Copy the key

### Key 2: Groq (AI summaries & suggestions)
1. Go to https://console.groq.com/keys
2. Sign up (free tier)
3. Click **Create API Key**
4. Copy the key

## Step 3: Enter Keys in the App

1. Open Vantage — the welcome screen shows 2 steps
2. Click **"Enter API Keys"**
3. Paste your **Deepgram** key > Save
4. Paste your **Groq** key > Save
5. Close settings

## You're Done!

Click **"Start"** to begin transcribing. The app captures your microphone and system audio separately ("You" vs "Participant").

---

## Optional: Better Audio Separation

By default, all audio shows as "You". To separate your voice from remote participants:

### Install BlackHole (free virtual audio device)
```bash
brew install blackhole-2ch
```

Then create a Multi-Output Device:
1. Open **Audio MIDI Setup** (in /Applications/Utilities/)
2. Click **"+"** > **Create Multi-Output Device**
3. Check both **BlackHole 2ch** and **MacBook Pro Speakers**
4. Go to **System Settings > Sound > Output** > select **Multi-Output Device**

Now Vantage will show "You" (mic) and "Participant" (system audio) separately.

## Optional: Connect Google Calendar

1. Open Vantage > **Settings** (gear icon)
2. Click **"Connect Calendar"**
3. Sign in with Google in the browser
4. Meetings auto-start transcription!

## Permissions

macOS will ask for:
- **Microphone** — click Allow (required)
- **Screen Recording** — click Allow (needed for system audio capture via ScreenCaptureKit)

## Troubleshooting

**App won't open:** Right-click > Open, or: `xattr -cr /Applications/Vantage.app`

**No transcription appearing:** Check your Deepgram key in Settings

**"You" only, no "Participant":** Install BlackHole (see above)

**Calendar not connecting:** Make sure you complete the Google sign-in in the browser

---

Built with Tauri + React + Rust
