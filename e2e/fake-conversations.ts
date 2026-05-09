/**
 * Fake conversation scenarios for desktop E2E testing.
 *
 * Each scenario is an array of DesktopTranscriptMessage objects that
 * will be emitted as Tauri `transcript-update` events with the specified delays.
 *
 * Key difference from web app: speakers are strings ("You", "Participant", "Speaker 1")
 * instead of numeric IDs.
 */

export interface DesktopTranscriptMessage {
  text: string;
  speaker: string; // 'You' | 'Participant' | 'Speaker 1' etc.
  is_final: boolean;
  /** Delay in ms before delivering this message (relative to previous message). */
  delayMs: number;
}

export type ConversationScenario = DesktopTranscriptMessage[];

/** Calculate total duration for a scenario (sum of all delays). */
export function totalDurationMs(scenario: ConversationScenario): number {
  return scenario.reduce((sum, msg) => sum + msg.delayMs, 0);
}

/**
 * MIC_ONLY_MONOLOGUE — 6 messages, speaker "You" only.
 * Simulates a user talking with mic-only mode.
 */
export const MIC_ONLY_MONOLOGUE: ConversationScenario = [
  { text: 'Good morning everyone', speaker: 'You', is_final: false, delayMs: 100 },
  { text: 'Good morning everyone', speaker: 'You', is_final: true, delayMs: 150 },
  { text: 'Let me share the update', speaker: 'You', is_final: false, delayMs: 200 },
  { text: 'Let me share the update from yesterday', speaker: 'You', is_final: true, delayMs: 150 },
  { text: 'We shipped the new dashboard', speaker: 'You', is_final: true, delayMs: 200 },
  { text: 'That is all from my side', speaker: 'You', is_final: true, delayMs: 200 },
];

/**
 * PARTICIPANT_ONLY — 7 messages, speaker "Participant" only.
 * Simulates capturing tab audio from a YouTube video or webinar.
 */
export const PARTICIPANT_ONLY: ConversationScenario = [
  { text: 'Welcome to the presentation', speaker: 'Participant', is_final: true, delayMs: 100 },
  { text: 'Today we will cover three topics', speaker: 'Participant', is_final: true, delayMs: 200 },
  { text: 'First up is the architecture overview', speaker: 'Participant', is_final: true, delayMs: 200 },
  { text: 'Thanks for that introduction', speaker: 'Participant', is_final: false, delayMs: 200 },
  { text: 'Thanks for that introduction', speaker: 'Participant', is_final: true, delayMs: 150 },
  { text: 'Let me dive into the details', speaker: 'Participant', is_final: true, delayMs: 200 },
  { text: 'Here is the system diagram', speaker: 'Participant', is_final: true, delayMs: 200 },
];

/**
 * FULL_MEETING — 13 messages, "You" + "Participant" with interims.
 * Simulates a realistic meeting with mic + tab audio.
 */
export const FULL_MEETING: ConversationScenario = [
  { text: 'Hi team lets get started', speaker: 'You', is_final: true, delayMs: 100 },
  { text: 'Sounds good', speaker: 'Participant', is_final: true, delayMs: 200 },
  { text: 'I have a quick update', speaker: 'Participant', is_final: false, delayMs: 200 },
  { text: 'I have a quick update on the backend', speaker: 'Participant', is_final: true, delayMs: 150 },
  { text: 'We migrated to the new database', speaker: 'Participant', is_final: true, delayMs: 200 },
  { text: 'Nice work on that migration', speaker: 'You', is_final: true, delayMs: 200 },
  { text: 'Any issues with the rollout', speaker: 'Participant', is_final: false, delayMs: 200 },
  { text: 'Any issues with the rollout so far', speaker: 'Participant', is_final: true, delayMs: 150 },
  { text: 'No issues everything is stable', speaker: 'You', is_final: true, delayMs: 200 },
  { text: 'Great lets move on', speaker: 'You', is_final: true, delayMs: 200 },
  { text: 'I will cover frontend next', speaker: 'Participant', is_final: true, delayMs: 200 },
  { text: 'We redesigned the settings page', speaker: 'Participant', is_final: true, delayMs: 200 },
  { text: 'Looks clean nice job', speaker: 'You', is_final: true, delayMs: 200 },
];

/**
 * RAPID_INTERIM — 7 messages testing interim->final replacement.
 * Multiple interim results followed by final for same utterance.
 */
export const RAPID_INTERIM: ConversationScenario = [
  { text: 'The', speaker: 'You', is_final: false, delayMs: 100 },
  { text: 'The quick', speaker: 'You', is_final: false, delayMs: 80 },
  { text: 'The quick brown fox', speaker: 'You', is_final: false, delayMs: 80 },
  { text: 'The quick brown fox jumps', speaker: 'You', is_final: true, delayMs: 100 },
  { text: 'Over', speaker: 'Participant', is_final: false, delayMs: 150 },
  { text: 'Over the lazy', speaker: 'Participant', is_final: false, delayMs: 80 },
  { text: 'Over the lazy dog', speaker: 'Participant', is_final: true, delayMs: 100 },
];
