# Testing MeetBetter

Thank you for testing MeetBetter! This guide will help you get started quickly.

## Quick Test (5 minutes)

### 1. Prerequisites
- macOS (Apple Silicon or Intel)
- Free API keys (takes 2 minutes to get)

### 2. Get API Keys

#### Deepgram (Required - for transcription)
1. Go to https://console.deepgram.com/signup
2. Sign up with email
3. Navigate to **API Keys** → **Create a New API Key**
4. Copy the key

#### Groq (Required - for AI summaries)
1. Go to https://console.groq.com/keys
2. Sign up with Google/GitHub
3. Click **Create API Key**
4. Copy the key

### 3. Install & Run

```bash
# Clone the repository
git clone https://github.com/venkateswarisudalai/MeetBetter.git
cd MeetBetter

# Install dependencies
npm install

# Run the app
npm run tauri dev
```

The app should launch automatically!

### 4. Configure API Keys

1. Click the **⚙️ Settings** icon (top right)
2. Paste your **Deepgram API key**
3. Paste your **Groq API key**
4. Click **Save**

### 5. Test Basic Transcription

1. Click **"Start Live Transcription"**
2. Speak into your microphone
3. Watch real-time transcription appear
4. Click **"Stop Transcription"**
5. Click **"Generate"** to get AI summary

**Expected Result:** You should see your words transcribed in real-time with very low latency (1-2 seconds).

---

## Advanced Test: Dual Audio Capture (Optional - 10 minutes)

Test the unique feature that separates "You" (microphone) from "Participant" (system audio).

### Setup

```bash
# Install BlackHole virtual audio device
brew install blackhole-2ch

# Create Multi-Output Device (so you can hear audio)
# 1. Open Audio MIDI Setup app
# 2. Click "+" → "Create Multi-Output Device"
# 3. Check: BlackHole 2ch + MacBook Pro Speakers
# 4. System Settings → Sound → Output → Select "Multi-Output Device"
```

### Test

1. Start Live Transcription
2. Play a YouTube video
3. Also speak into your microphone

**Expected Result:**
- Video audio shows as **"Participant:"**
- Your voice shows as **"You:"**

---

## Test Calendar Auto-Start (Optional - 5 minutes)

1. Click **Settings** → **Meeting Auto-Start**
2. Enable **"Auto-start on meeting time"**
3. Click **"Connect Calendar"** → Sign in with Google
4. Create a test meeting in Google Calendar (5 minutes from now)
5. Open Zoom/Teams/Meet

**Expected Result:** App shows "Meeting starting in X minutes" and auto-starts transcription when the time comes.

---

## What to Test & Report

### ✅ Core Features
- [ ] Real-time transcription works
- [ ] Transcription latency is acceptable (1-2 seconds)
- [ ] AI summary generation works
- [ ] Can save and view past meetings
- [ ] UI is intuitive and responsive

### ✅ Dual Audio (if tested)
- [ ] Can differentiate "You" vs "Participant"
- [ ] No duplicate transcriptions
- [ ] Audio quality is good

### ✅ Calendar Integration (if tested)
- [ ] Google Calendar connection works
- [ ] Auto-start triggers correctly
- [ ] Meeting detection works

### 🐛 Issues to Report
- Any crashes or errors
- Features that don't work as expected
- UI/UX issues
- Performance problems
- Installation difficulties

---

## Reporting Feedback

Please report issues or feedback via:
- GitHub Issues: https://github.com/venkateswarisudalai/MeetBetter/issues
- Email: [your email]
- Or provide feedback directly

**What to include:**
- What you were trying to do
- What happened (vs what you expected)
- Any error messages
- macOS version
- Screenshots (if applicable)

---

## FAQ

**Q: Do I need to install BlackHole?**
A: No! BlackHole is only needed for the dual audio capture feature (separating "You" vs "Participant"). The app works perfectly without it.

**Q: Can I use this for real meetings?**
A: Yes! But be aware:
- You're responsible for API costs (both have generous free tiers)
- Always inform meeting participants you're recording/transcribing
- Check your local laws about recording consent

**Q: Is my audio data stored somewhere?**
A: No. Audio is processed in real-time and only the transcription text is sent to APIs. Nothing is stored on external servers except in your local database.

**Q: What happens if I run out of API credits?**
A: The app will show an error. You can:
- Upgrade your Deepgram/Groq plan
- Get a new free tier account
- The app won't charge you anything - you only pay APIs directly

---

## Performance Expectations

**System Requirements:**
- macOS 10.15 or later
- 4GB RAM minimum (8GB recommended)
- Internet connection (for transcription APIs)

**Expected Performance:**
- Transcription latency: 1-2 seconds
- CPU usage: Low (5-10% on M1)
- Memory usage: ~200-300 MB
- Network usage: ~50-100 KB/s during transcription

---

## Thank You!

Your feedback is invaluable for improving MeetBetter. Thank you for taking the time to test!

For questions or issues, reach out via GitHub Issues or email.
