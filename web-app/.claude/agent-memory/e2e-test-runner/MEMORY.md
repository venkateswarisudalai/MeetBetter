# MeetBetter / Vantage E2E Test Runner Memory

## Project Basics
- App: Vite + React + TypeScript SPA, single `App.tsx` with all pages
- Deployed at: https://web-app-eta-beige.vercel.app
- E2E Framework: Playwright (installed as devDependency @playwright/test)
- Config: `/Users/vigneshsubbiah/Documents/MeetBetter/web-app/playwright.config.ts`
- Tests: `/Users/vigneshsubbiah/Documents/MeetBetter/web-app/e2e/`
- Run command: `npx playwright test` (or `npm run test:e2e`)

## Test File Map
- `e2e/helpers.ts` — shared utilities: storage keys, seed helpers, meeting mocks
- `e2e/01-navigation.spec.ts` — all page transitions (7 tests)
- `e2e/02-settings.spec.ts` — API key saving/loading (8 tests)
- `e2e/03-home-ui.spec.ts` — audio mode selector, context textarea, setup banner (15 tests)
- `e2e/04-meeting-flow.spec.ts` — start/stop meeting with mocks (12 tests)
- `e2e/05-history.spec.ts` — history list, view meeting, delete (14 tests)
- `e2e/06-save-meeting.spec.ts` — save after stop, verify in history (7 tests)
- `e2e/07-copy-transcript.spec.ts` — copy buttons from done page and view-meeting (9 tests)
- `e2e/08-empty-states.spec.ts` — no-data scenarios (9 tests)

## localStorage Keys
- API keys: `vantage_keys`  (JSON `{deepgram, groq}`)
- Meetings: `vantage_meetings` (JSON array of Meeting objects)

## Key Mocking Patterns
See `e2e/helpers.ts` `installMeetingMocks()` — installs via `page.addInitScript`.
- `navigator.mediaDevices.getUserMedia/getDisplayMedia` → returns `new MediaStream()` (empty, no tracks)
- `window.WebSocket` → `FakeWebSocket` for URLs containing "deepgram" only
- `window.MediaRecorder` → `FakeMediaRecorder` (no-op recording)
- `window.AudioContext` → `FakeAudioContext` (stub)

## Critical Timing Lesson
**Do NOT use `new AudioContext()` inside `addInitScript` before FakeAudioContext is installed.**
The `silentStream` helper using real AudioContext caused `getUserMedia` to fail.
Solution: return `new MediaStream()` directly (empty stream is sufficient).

**FakeWebSocket message timing:** `onmessage` is set by the app AFTER the constructor returns.
Messages fired inside the constructor will miss the handler.
Fix: delay `_deliverTranscript()` to 200ms after construction (separate from onopen at 50ms).
Wait at least 1200ms after clicking Start before stopping to capture all 3 fake transcript messages.

## Playwright Config Notes
- `workers: 1`, `fullyParallel: false` — tests run serially (important for localStorage isolation)
- `permissions: ['microphone']` in use config
- Chromium launch args: `--use-fake-ui-for-media-stream --use-fake-device-for-media-stream`
- `baseURL` defaults to deployed Vercel URL; set `BASE_URL` env var to override for local dev

## Test Baseline
Last run: 81/81 tests passing (~1.2 minutes on Chromium headless, macOS arm64)
