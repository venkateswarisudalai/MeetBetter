/**
 * Shared test helpers for Vantage desktop E2E tests.
 *
 * The key challenge: the desktop app uses Tauri IPC (invoke/listen) instead of
 * direct WebSocket. We mock `window.__TAURI_INTERNALS__` so the React app
 * runs in a plain Chromium browser without the Rust backend.
 */
import { Page } from '@playwright/test';
import { ConversationScenario } from './fake-conversations';

/**
 * Install Tauri IPC mocks via page.addInitScript so they're available
 * before any app code runs.
 *
 * This creates:
 * - window.__TAURI_INTERNALS__ with invoke() and transformCallback()
 * - window.__TAURI_EVENT_PLUGIN_INTERNALS__ with unregisterListener()
 * - window.__TAURI_TEST_EMIT__(event, payload) for tests to inject events
 */
export async function installTauriMocks(page: Page) {
  await page.addInitScript(() => {
    // ---- Callback registry (mirrors Tauri's transformCallback) ----
    let callbackId = 0;
    const callbacks: Record<number, Function> = {};

    // ---- Event listener registry ----
    // Maps event name -> array of { id, handlerCallbackId }
    const eventListeners: Record<string, Array<{ id: number; handlerCallbackId: number }>> = {};
    let nextEventId = 1;

    // ---- Default command handler ----
    const commandDefaults: Record<string, any> = {
      get_meeting_state: {
        has_groq_key: true,
        has_deepgram_key: true,
        has_proxy: false,
        meeting_context: '',
      },
      check_microphone_permission: 'granted',
      check_screen_recording_permission: 'granted',
      get_saved_meetings: [],
      get_meeting_by_id: null,
      search_meetings: [],
      start_live_transcription: undefined,
      stop_live_transcription: '',
      start_recording: undefined,
      stop_recording: '',
      save_meeting: `mock-meeting-id-${Date.now()}`,
      delete_meeting: undefined,
      clear_transcription: undefined,
      generate_summary: '',
      generate_structured_summary: { key_points: [], action_items: [], decisions: [], notes: [], raw_summary: '' },
      generate_reply_suggestions: [],
      generate_auto_replies: [],
      check_connection: true,
      set_groq_api_key: undefined,
      set_deepgram_api_key: undefined,
      set_assemblyai_api_key: undefined,
      set_proxy_url: undefined,
      set_model: undefined,
      set_transcription_provider: undefined,
      set_meeting_context: undefined,
      set_screen_share_exclusion: undefined,
      is_screen_share_exclusion_supported: false,
      get_audio_diagnostics: { devices: [], default_device: null },
      get_screen_share_platform_info: { platform: 'test' },
      get_transcription_providers: ['deepgram'],
      get_available_models: ['llama-3.3-70b-versatile'],
      list_recordings: [],
      get_recordings_folder: '/tmp/recordings',
      transcribe_recording: '',
      add_transcription: undefined,
      add_manual_transcript: undefined,
      ask_about_meeting: 'Mock AI answer.',
      enhance_notes: 'Enhanced notes content.',
      start_mock_transcription: undefined,
      stop_mock_transcription: undefined,
      is_calendar_connected: false,
      get_upcoming_events: [],
      get_past_calendar_events: [],
      get_google_auth_url: '',
      exchange_google_code: undefined,
      disconnect_calendar: undefined,
      get_meeting_monitor_settings: {
        enabled: false,
        start_buffer_minutes: 2,
        detect_meeting_apps: true,
        auto_start_on_time: true,
      },
      update_meeting_monitor_settings: undefined,
      get_meeting_status: {
        is_meeting_detected: false,
        meeting_app_running: null,
        upcoming_meeting: null,
        minutes_until_meeting: null,
        auto_start_triggered: false,
      },
      check_for_meetings_now: undefined,
      reset_meeting_monitor_trigger: undefined,
      toggle_cloud_sync: undefined,
      get_cloud_sync_status: { enabled: false },
      sync_meetings_to_cloud: undefined,
      update_calendar_event_description: undefined,
      wait_for_oauth_callback: undefined,
    };

    // ---- window.__TAURI_INTERNALS__ ----
    (window as any).__TAURI_INTERNALS__ = {
      transformCallback(fn: Function, once = false): number {
        const id = callbackId++;
        callbacks[id] = once
          ? (...args: any[]) => { fn(...args); delete callbacks[id]; }
          : fn;
        return id;
      },

      unregisterCallback(id: number) {
        delete callbacks[id];
      },

      async invoke(cmd: string, args: any = {}, _options?: any): Promise<any> {
        // Handle event plugin commands specially
        if (cmd === 'plugin:event|listen') {
          const { event, handler: handlerCallbackId } = args;
          const eventId = nextEventId++;
          if (!eventListeners[event]) eventListeners[event] = [];
          eventListeners[event].push({ id: eventId, handlerCallbackId });
          return eventId;
        }

        if (cmd === 'plugin:event|unlisten') {
          const { event, eventId } = args;
          if (eventListeners[event]) {
            eventListeners[event] = eventListeners[event].filter(l => l.id !== eventId);
          }
          return;
        }

        // Default command handling
        if (cmd in commandDefaults) {
          return commandDefaults[cmd];
        }

        // Unknown commands return undefined (safe no-op)
        return undefined;
      },
    };

    // ---- window.__TAURI_EVENT_PLUGIN_INTERNALS__ ----
    (window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener(event: string, eventId: number) {
        if (eventListeners[event]) {
          eventListeners[event] = eventListeners[event].filter(l => l.id !== eventId);
        }
      },
    };

    // ---- Test helper: emit events from test code ----
    (window as any).__TAURI_TEST_EMIT__ = (event: string, payload: any) => {
      const listeners = eventListeners[event] || [];
      for (const listener of listeners) {
        const cb = callbacks[listener.handlerCallbackId];
        if (cb) {
          // Tauri wraps event payloads in { event, id, payload } structure
          cb({ event, id: listener.id, payload });
        }
      }
    };
  });
}

/**
 * Emit a single transcript-update event into the running app.
 */
export async function emitTranscript(
  page: Page,
  segment: { text: string; speaker: string; is_final: boolean },
) {
  await page.evaluate(
    ({ text, speaker, is_final }) => {
      (window as any).__TAURI_TEST_EMIT__('transcript-update', {
        text,
        timestamp: new Date().toISOString(),
        speaker,
        is_final,
      });
    },
    segment,
  );
}

/**
 * Deliver a full conversation scenario as a sequence of transcript-update events.
 */
export async function emitConversation(page: Page, scenario: ConversationScenario) {
  for (const msg of scenario) {
    if (msg.delayMs > 0) {
      await page.waitForTimeout(msg.delayMs);
    }
    await emitTranscript(page, {
      text: msg.text,
      speaker: msg.speaker,
      is_final: msg.is_final,
    });
  }
}

/**
 * Click the Start Meeting button and wait for recording state.
 */
export async function startMeeting(page: Page) {
  await page.locator('.start-btn').click();
  await page.waitForSelector('.app-minimal.recording', { timeout: 5000 });
}

/**
 * Click the Stop button and wait for done state.
 */
export async function stopMeeting(page: Page) {
  await page.locator('.stop-btn').click();
  await page.waitForSelector('.app-minimal.done', { timeout: 5000 });
}
