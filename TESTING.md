# Testing Vantage (developer guide)

This file is for **developers building from source**. If you just downloaded the DMG from the website and want to use Vantage, see [TESTER_SETUP.md](TESTER_SETUP.md) instead.

---

## 1. Prerequisites

| Tool | Version | Install |
|---|---|---|
| Xcode Command Line Tools | any | `xcode-select --install` |
| Node.js | 18+ (tested on 22) | `brew install node` |
| Rust | stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| BlackHole 2ch *(optional)* | 2.x | `brew install blackhole-2ch` — only for dual-channel audio testing |

You'll also need free API keys for Deepgram and Groq (see [TESTER_SETUP.md §3](TESTER_SETUP.md#3-get-two-free-api-keys-2-min)).

## 2. Clone and install

```bash
git clone https://github.com/venkateswarisudalai/MeetBetter.git
cd MeetBetter
npm install
```

Compile-time secrets (Google OAuth client, Supabase URL/key) live in `.env.build` and are auto-loaded by `src-tauri/build.rs` — no shell sourcing needed. To override, set the env vars in your shell before building.

## 3. Three ways to run

### Full desktop app (Tauri + Rust)
```bash
npm run tauri dev      # ~5 min first build, ~10s incremental
```
Use this for testing anything that touches Rust: audio capture, calendar OAuth, persistence, system audio.

### Frontend only (Vite, no Rust)
```bash
npm run dev            # serves http://localhost:1420
```
Tauri IPC must be mocked (see `e2e/helpers.ts → installTauriMocks`). Fastest loop for pure UI work.

### Playwright e2e tests
```bash
npm run test:e2e           # headless
npm run test:e2e:headed    # see the browser
npm run test:e2e:report    # open last HTML report (e2e-report/)
```
The suite mocks Tauri events to verify chat-bubble layout, speaker labels, interim → final replacement, and full-meeting rendering. Playwright config reuses an existing dev server on port 1420 outside CI; if another project is on that port, kill it first.

## 4. Pre-PR checks

Before opening a PR, all of these must pass:

```bash
npm run build               # tsc + vite, must be warning-free
(cd src-tauri && cargo check)
npm run test:e2e
```

For a real release, also smoke-test `npm run tauri dev` end-to-end, then build the DMG:

```bash
npm run tauri build
./scripts/sign-and-package.sh
```

## 5. Manual verification checklist

Run through the same scenarios real users hit. A clean transcript here is the strongest signal that nothing regressed.

### Core flow
- [ ] Welcome screen accepts Deepgram + Groq keys
- [ ] Live transcription starts, latency < 2s
- [ ] Stop → transcript renders fully (final segments only, no leftover interims)
- [ ] Generate → summary returns key points / action items / decisions
- [ ] Save Meeting → it persists across app relaunch
- [ ] Re-open a saved meeting → transcript and summary render

### Dual audio (with BlackHole + Multi-Output Device)
- [ ] Mic input → labeled "You" (right side bubble)
- [ ] System audio (e.g. YouTube playback) → labeled "Participant" (left side bubble)
- [ ] No duplicate transcriptions
- [ ] Stopping and restarting doesn't change the labeling

### Calendar auto-start
- [ ] OAuth completes without redirect loop
- [ ] Upcoming events list populates
- [ ] Test event 5 min out + Zoom/Meet open → auto-start fires within the buffer window

### Permissions
- [ ] Fresh install asks for Microphone → grant → works
- [ ] Asks for Screen Recording → grant → system audio works
- [ ] Revoking either permission shows a clear error in-app, not a silent failure

### Edge cases
- [ ] No internet → clear error, no crash
- [ ] Bad API key → clear error in Settings
- [ ] Deepgram quota exhausted → clear error
- [ ] Very long meeting (30+ min) → no memory blow-up, summary still works
- [ ] Privacy: confirm via Network tab that no audio bytes leave the device — only transcript text

## 6. What to write in a bug report

- macOS version (`sw_vers`)
- Vantage version (Settings → About) and `git rev-parse --short HEAD` if running from source
- What you did, what you expected, what happened
- Any error text from the app
- Relevant lines from the dev console (Right-click in Tauri window → Inspect Element → Console) if frontend
- For Rust panics: terminal output from `npm run tauri dev`

## 7. Known gotchas

- **Port 1420 already in use** — another Vite project is running. `lsof -i :1420` to find it; kill it before running tests.
- **`xcrun: error` during `cargo build`** — `xcode-select --install`.
- **`source $HOME/.cargo/env` not run** — `cargo` not on PATH; restart terminal or source it.
- **`.env.build` missing** — Rust build fails with `env! GOOGLE_CLIENT_ID not defined at compile time`. The file is committed; if you cleaned the working tree too aggressively, restore it from git.
- **Stale Cargo target after a Rust toolchain update** — `(cd src-tauri && cargo clean)` and rebuild.

---

For end-user / tester install instructions, see [TESTER_SETUP.md](TESTER_SETUP.md). For the high-level architecture, see [README.md](README.md).
