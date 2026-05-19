# Vantage — Install & Test Guide

You downloaded Vantage from https://vantage-meeting-app.netlify.app or got a DMG link. This page is everything you need to make it work, in order.

**Time:** ~5 minutes (longer if you set up dual audio).
**You'll need:** macOS 12.3+ (Intel or Apple Silicon) and a couple of minutes to grab two free API keys.

---

## 1. Install

### Option A — One-line install (recommended)

Open Terminal and paste:

```bash
curl -fsSL https://vantage-meeting-app.netlify.app/install.sh | bash
```

This downloads the latest DMG, copies `Vantage.app` to `/Applications`, clears the macOS quarantine flag, and offers to open the app.

### Option B — Manual install

1. Download the DMG from the website (or from [GitHub Releases](https://github.com/venkateswarisudalai/MeetBetter/releases)).
2. Double-click the DMG, drag **Vantage.app** to `/Applications`.
3. Clear the quarantine flag once. Vantage is ad-hoc signed (not Apple-notarized), so without this macOS will say "damaged" or "unidentified developer":

   ```bash
   xattr -cr /Applications/Vantage.app
   ```

   *Or, instead:* right-click `Vantage.app` → **Open**. macOS will warn you once; click **Open** anyway.

---

## 2. Grant macOS permissions on first launch

When you open Vantage for the first time, macOS will prompt for two permissions. **Both are required.**

| Permission | What it's used for |
|---|---|
| **Microphone** | Captures your voice |
| **Screen Recording** | Captures system audio (the *other* person's voice on Zoom/Meet/Teams). Vantage doesn't actually record your screen — this is just the macOS permission that gates ScreenCaptureKit, which is how it gets system audio. |

If you accidentally deny either one:
**System Settings → Privacy & Security → Microphone / Screen Recording** → toggle Vantage on, then **quit and relaunch** the app (the permission only takes effect after relaunch).

---

## 3. Get two free API keys (~2 min)

Vantage uses your own keys so audio never goes through someone else's server.

### Deepgram (real-time transcription)
1. Sign up at https://console.deepgram.com — free tier includes **$200 credit**.
2. **API Keys → Create a New API Key**.
3. Copy it.

### Groq (AI summaries and reply suggestions)
1. Sign up at https://console.groq.com/keys — free tier.
2. **Create API Key**.
3. Copy it.

---

## 4. Paste keys into Vantage

The welcome screen walks you through it:

1. Open Vantage.
2. Paste your **Deepgram** key → **Save**.
3. Paste your **Groq** key → **Save**.

You're ready to go.

---

## 5. Try it (the smoke test)

1. Click **Start Live Transcription**.
2. Speak into your mic for ~10 seconds. Words should appear within 1–2 seconds, labeled **You** (right side, blue bubbles).
3. Click **Stop**.
4. In the Summary panel, click **Generate** → an AI summary appears (key points, action items, decisions).
5. Click **Save Meeting**, give it a title — it appears in the past meetings list.

If transcription doesn't appear, check the [troubleshooting table](#troubleshooting).

---

## 6. (Optional) Set up dual audio — separate "You" from "Participant"

By default, **everything** in a Zoom/Meet/Teams call is labeled "You" because Vantage only hears your microphone. To label the remote person as "Participant", route system audio through BlackHole.

```bash
brew install blackhole-2ch
```

Then pick one:

### Quick (no audio playback)
**System Settings → Sound → Output → BlackHole 2ch.** You won't hear your meeting, but the channel split works perfectly. Good for verification, bad for actual meetings.

### Real use (you can still hear)
1. Open **Audio MIDI Setup** (in `/Applications/Utilities/`).
2. Click **+** at bottom-left → **Create Multi-Output Device**.
3. In the right panel, check **both**:
   - BlackHole 2ch
   - Your speakers (e.g. MacBook Pro Speakers)
4. **System Settings → Sound → Output → Multi-Output Device**.
5. Keep speaker volume **low**, or use **headphones** — otherwise the mic re-captures speaker output and you'll get echo.

### Verify it works
1. Start Live Transcription in Vantage.
2. Play any YouTube video → text should appear labeled **Participant** (left side, gray bubbles).
3. Speak into mic → text labeled **You** (right side, blue bubbles).

---

## 7. (Optional) Connect Google Calendar for auto-start

So Vantage starts transcribing automatically when your meeting begins.

1. Open Vantage → **Settings** (gear icon).
2. Click **Connect Calendar** — your browser opens for Google sign-in.
3. Grant calendar read access. You're redirected back.
4. Toggle **Auto-start on meeting time** on. Set a buffer (default 2 min before meeting).
5. Test: create a Google Calendar event 5 min from now, open Zoom/Meet/Teams. Vantage should detect the meeting and auto-start.

---

## What to test (checklist for testers)

- [ ] App opens after `xattr -cr` (or right-click → Open)
- [ ] Both macOS permissions granted, no relaunch loop
- [ ] Real-time transcription latency feels acceptable (< 2s)
- [ ] AI summary generates from transcript
- [ ] Meeting saves and reappears after relaunch
- [ ] (Dual audio set up) "You" and "Participant" labels are correct
- [ ] (Calendar connected) Auto-start triggers near a real calendar event
- [ ] Privacy expectation holds: no audio uploaded, only transcript text leaves the device

---

## Troubleshooting

| Symptom | Fix |
|---|---|
| `"Vantage" is damaged and can't be opened` | `xattr -cr /Applications/Vantage.app`, then re-open |
| `"Vantage" cannot be opened because the developer cannot be verified` | Right-click app → **Open**, then click **Open** in the dialog. Or use the `xattr` command above. |
| No transcription appearing | Check Deepgram key in Settings; verify mic permission is on; check internet |
| Everything labeled "You", never "Participant" | BlackHole isn't installed, or system audio output isn't BlackHole / Multi-Output Device |
| Transcriptions repeat or echo | Multi-Output speaker volume too high — mic is re-hearing the speakers. Lower volume or use headphones. |
| Mic permission revoked | System Settings → Privacy & Security → Microphone → enable Vantage → **quit and relaunch** |
| Screen Recording permission revoked | Same path → Screen Recording → enable → relaunch |
| `Not authenticated with Google` for calendar | Settings → **Connect Calendar** → complete OAuth in browser |
| AI summary fails or is blank | Check Groq key; very short transcripts may produce empty summaries |
| App opens but window is blank | Quit, run `xattr -cr /Applications/Vantage.app`, reopen |

---

## What runs on your machine vs. in the cloud

- **Local only:** raw audio, recordings (if enabled), the SQLite meeting database, your API keys.
- **Sent to APIs:** transcript text → Deepgram (during transcription); transcript text → Groq (when you click Generate / Get Replies).
- **Never sent anywhere:** raw audio bytes.

---

## Reporting issues

- **Bugs / feature requests:** https://github.com/venkateswarisudalai/MeetBetter/issues
- Include: macOS version, Vantage version (Settings → About), what you did, what you expected, what happened, any error text or screenshots.

---

## System expectations

- macOS 12.3+ (universal — Intel and Apple Silicon)
- ~300 MB RAM, low single-digit CPU on M-series during transcription
- ~50–100 KB/s network during a live transcription session
- Internet required for transcription and AI features; recording works offline

---

For developer setup (clone the repo, `npm run tauri dev`), see [TESTING.md](TESTING.md) and the [README](README.md#run-it-locally).
