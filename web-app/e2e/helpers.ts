/**
 * Shared test helpers for MeetBetter / Vantage E2E tests.
 */
import { Page } from '@playwright/test';
import { ConversationScenario } from './fake-conversations';

// localStorage key constants (must match src/lib/storage.ts)
export const KEYS_STORAGE = 'vantage_keys';
export const MEETINGS_STORAGE = 'vantage_meetings';

// Fake API keys that are long enough to pass the hasKeys check (> 5 chars each)
export const FAKE_DG_KEY = 'fake_deepgram_key_abcdef';
export const FAKE_GROQ_KEY = 'fake_groq_key_abcdef1234';

// A pre-built meeting object used to seed localStorage in several tests
export const SEED_MEETING = {
  id: '1700000000000',
  title: 'Engineering standup',
  date: new Date('2024-01-15T10:00:00Z').toISOString(),
  duration: 300,
  transcript: '[You]: Hello everyone.\n[Speaker 2]: Hi there.',
  summary: '## Summary\nQuick engineering standup.',
  context: 'Engineering standup',
};

/** Clear all app-related localStorage keys and reload. */
export async function clearStorage(page: Page) {
  await page.evaluate(
    ({ keys, meetings }) => {
      localStorage.removeItem(keys);
      localStorage.removeItem(meetings);
    },
    { keys: KEYS_STORAGE, meetings: MEETINGS_STORAGE },
  );
}

/** Seed API keys into localStorage without going through the UI. */
export async function seedApiKeys(page: Page, dg = FAKE_DG_KEY, groq = FAKE_GROQ_KEY) {
  await page.evaluate(
    ({ storageKey, value }) => {
      localStorage.setItem(storageKey, JSON.stringify(value));
    },
    { storageKey: KEYS_STORAGE, value: { deepgram: dg, groq } },
  );
}

/** Seed a meeting into localStorage. */
export async function seedMeeting(page: Page, meeting = SEED_MEETING) {
  await page.evaluate(
    ({ storageKey, m }) => {
      const existing = JSON.parse(localStorage.getItem(storageKey) || '[]');
      existing.unshift(m);
      localStorage.setItem(storageKey, JSON.stringify(existing));
    },
    { storageKey: MEETINGS_STORAGE, m: meeting },
  );
}

/**
 * Install mocks that intercept the DeepgramClient constructor and
 * navigator.mediaDevices so that "Start Meeting" can complete without
 * real hardware or network access.
 *
 * Call this BEFORE the page navigates to the URL that loads the app bundle
 * (i.e., inside page.addInitScript or before page.goto).
 */
export async function installMeetingMocks(page: Page, scenario?: ConversationScenario) {
  const scenarioJson = scenario ? JSON.stringify(scenario) : '';
  await page.addInitScript((serializedScenario: string) => {
    // ---------- Mock getUserMedia / getDisplayMedia ----------
    // Return a plain empty MediaStream — no oscillator or AudioContext needed.
    // The DeepgramClient only checks that tracks exist; we add a fake audio track.
    // Capture the real AudioContext before we overwrite it below.
    const RealAudioContext = window.AudioContext || (window as any).webkitAudioContext;

    const makeFakeStream = () => {
      // We need a MediaStream with at least one audio track so that
      // getAudioTracks().length > 0 (required for tab audio mode).
      // Create a silent oscillator via the real AudioContext.
      try {
        const ctx = new RealAudioContext();
        const osc = ctx.createOscillator();
        const dest = ctx.createMediaStreamDestination();
        osc.connect(dest);
        osc.start();
        // The destination's stream has a real audio track
        return dest.stream;
      } catch {
        // Fallback to empty stream if AudioContext isn't available
        return new MediaStream();
      }
    };

    Object.defineProperty(navigator, 'mediaDevices', {
      configurable: true,
      writable: true,
      value: {
        getUserMedia: async () => makeFakeStream(),
        getDisplayMedia: async () => makeFakeStream(),
      },
    });

    // ---------- Mock WebSocket (Deepgram) ----------
    class FakeWebSocket extends EventTarget {
      static CONNECTING = 0;
      static OPEN = 1;
      static CLOSING = 2;
      static CLOSED = 3;

      readyState = FakeWebSocket.OPEN;
      url: string;
      protocol: string;
      private _listeners: Record<string, ((e: any) => void)[]> = {};

      constructor(url: string, protocols?: string | string[]) {
        super();
        this.url = url;
        this.protocol = Array.isArray(protocols) ? protocols[0] : protocols ?? '';

        // Fire onopen after the constructor returns so the caller can assign
        // onopen/onmessage before the events fire.
        setTimeout(() => {
          const ev = new Event('open');
          this._dispatch('open', ev);
          if (typeof (this as any).onopen === 'function') (this as any).onopen(ev);
        }, 50);

        // Deliver transcript messages well after open so onmessage is set.
        // The app assigns ws.onmessage synchronously after constructing the WS.
        setTimeout(() => {
          this._deliverTranscript();
        }, 200);
      }

      private _dispatch(type: string, ev: any) {
        (this._listeners[type] || []).forEach(fn => fn(ev));
      }

      private _deliverTranscript() {
        // If a custom scenario was provided, use it; otherwise fall back to hardcoded messages
        if (serializedScenario) {
          const scenario: Array<{ transcript: string; speaker: number; isFinal: boolean; delayMs: number }> =
            JSON.parse(serializedScenario);
          let cumulativeDelay = 0;
          scenario.forEach((item) => {
            cumulativeDelay += item.delayMs;
            const delay = cumulativeDelay;
            setTimeout(() => {
              const msg = {
                channel: {
                  alternatives: [
                    {
                      transcript: item.transcript,
                      words: [{ word: item.transcript.split(' ')[0], speaker: item.speaker }],
                    },
                  ],
                },
                is_final: item.isFinal,
              };
              const ev = new MessageEvent('message', { data: JSON.stringify(msg) });
              this._dispatch('message', ev);
              if (typeof (this as any).onmessage === 'function') (this as any).onmessage(ev);
            }, delay);
          });
        } else {
          const messages = [
            {
              channel: {
                alternatives: [
                  { transcript: 'Hello everyone', words: [{ word: 'Hello', speaker: 0 }] },
                ],
              },
              is_final: false,
            },
            {
              channel: {
                alternatives: [
                  { transcript: 'Hello everyone', words: [{ word: 'Hello', speaker: 0 }] },
                ],
              },
              is_final: true,
            },
            {
              channel: {
                alternatives: [
                  {
                    transcript: 'Welcome to the standup',
                    words: [{ word: 'Welcome', speaker: 1 }],
                  },
                ],
              },
              is_final: true,
            },
          ];

          // Stagger messages so React has time to re-render between each
          messages.forEach((msg, i) => {
            setTimeout(() => {
              const ev = new MessageEvent('message', { data: JSON.stringify(msg) });
              this._dispatch('message', ev);
              if (typeof (this as any).onmessage === 'function') (this as any).onmessage(ev);
            }, i * 200);
          });
        }
      }

      addEventListener(type: string, fn: (e: any) => void) {
        (this._listeners[type] = this._listeners[type] || []).push(fn);
      }

      removeEventListener(type: string, fn: (e: any) => void) {
        this._listeners[type] = (this._listeners[type] || []).filter(f => f !== fn);
      }

      send(_data: any) { /* no-op */ }

      close(code = 1000) {
        this.readyState = FakeWebSocket.CLOSED;
        const ev = new CloseEvent('close', { code, wasClean: code === 1000 });
        this._dispatch('close', ev);
        if (typeof (this as any).onclose === 'function') (this as any).onclose(ev);
      }
    }

    // Replace global WebSocket with fake only for Deepgram URL pattern
    const OrigWebSocket = window.WebSocket;
    (window as any).WebSocket = function (url: string, protocols?: string | string[]) {
      if (typeof url === 'string' && url.includes('deepgram')) {
        return new FakeWebSocket(url, protocols);
      }
      return new OrigWebSocket(url, protocols as any);
    };
    // Copy static properties
    Object.assign((window as any).WebSocket, OrigWebSocket);

    // ---------- Mock MediaRecorder ----------
    class FakeMediaRecorder extends EventTarget {
      state: 'inactive' | 'recording' | 'paused' = 'inactive';
      stream: MediaStream;
      ondataavailable: ((e: any) => void) | null = null;

      constructor(stream: MediaStream) {
        super();
        this.stream = stream;
      }

      start(_timeslice?: number) {
        this.state = 'recording';
      }

      stop() {
        this.state = 'inactive';
      }

      pause() { this.state = 'paused'; }
      resume() { this.state = 'recording'; }
    }

    (window as any).MediaRecorder = FakeMediaRecorder;

    // ---------- Mock AudioContext ----------
    class FakeAudioNode {
      connect() {}
      disconnect() {}
    }
    class FakeAudioContext {
      createMediaStreamSource() { return new FakeAudioNode(); }
      createMediaStreamDestination() {
        return { stream: new MediaStream() };
      }
      close() {}
    }
    (window as any).AudioContext = FakeAudioContext;
  }, scenarioJson);
}

/** Read all meetings from localStorage (evaluated inside the page). */
export async function getStoredMeetings(page: Page) {
  return page.evaluate((key) => {
    const raw = localStorage.getItem(key);
    return raw ? JSON.parse(raw) : [];
  }, MEETINGS_STORAGE);
}
