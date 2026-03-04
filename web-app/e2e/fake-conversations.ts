/**
 * Fake conversation scenarios for E2E testing.
 *
 * Each scenario is an array of FakeDeepgramMessage objects that the
 * FakeWebSocket will deliver in sequence with the specified delays.
 */

export interface FakeDeepgramMessage {
  transcript: string;
  speaker: number;
  isFinal: boolean;
  /** Delay in ms before delivering this message (relative to previous message). */
  delayMs: number;
}

export type ConversationScenario = FakeDeepgramMessage[];

/** Calculate total duration for a scenario (sum of all delays). */
export function totalDurationMs(scenario: ConversationScenario): number {
  return scenario.reduce((sum, msg) => sum + msg.delayMs, 0);
}

/**
 * MIC_ONLY_MONOLOGUE — 6 messages, speaker 0 only.
 * Simulates a user talking with mic-only mode.
 */
export const MIC_ONLY_MONOLOGUE: ConversationScenario = [
  { transcript: 'Good morning everyone', speaker: 0, isFinal: false, delayMs: 100 },
  { transcript: 'Good morning everyone', speaker: 0, isFinal: true, delayMs: 150 },
  { transcript: 'Let me share the update', speaker: 0, isFinal: false, delayMs: 200 },
  { transcript: 'Let me share the update from yesterday', speaker: 0, isFinal: true, delayMs: 150 },
  { transcript: 'We shipped the new dashboard', speaker: 0, isFinal: true, delayMs: 200 },
  { transcript: 'That is all from my side', speaker: 0, isFinal: true, delayMs: 200 },
];

/**
 * TAB_AUDIO_YOUTUBE — 7 messages, speakers 1+2 only.
 * Simulates capturing tab audio from a YouTube video or webinar.
 */
export const TAB_AUDIO_YOUTUBE: ConversationScenario = [
  { transcript: 'Welcome to the presentation', speaker: 1, isFinal: true, delayMs: 100 },
  { transcript: 'Today we will cover three topics', speaker: 1, isFinal: true, delayMs: 200 },
  { transcript: 'First up is the architecture overview', speaker: 1, isFinal: true, delayMs: 200 },
  { transcript: 'Thanks for that introduction', speaker: 2, isFinal: false, delayMs: 200 },
  { transcript: 'Thanks for that introduction', speaker: 2, isFinal: true, delayMs: 150 },
  { transcript: 'Let me dive into the details', speaker: 2, isFinal: true, delayMs: 200 },
  { transcript: 'Here is the system diagram', speaker: 1, isFinal: true, delayMs: 200 },
];

/**
 * FULL_MEETING_CONVERSATION — 13 messages, speakers 0+1+2 with interims.
 * Simulates a realistic meeting with mic + tab audio.
 */
export const FULL_MEETING_CONVERSATION: ConversationScenario = [
  { transcript: 'Hi team lets get started', speaker: 0, isFinal: true, delayMs: 100 },
  { transcript: 'Sounds good', speaker: 1, isFinal: true, delayMs: 200 },
  { transcript: 'I have a quick update', speaker: 2, isFinal: false, delayMs: 200 },
  { transcript: 'I have a quick update on the backend', speaker: 2, isFinal: true, delayMs: 150 },
  { transcript: 'We migrated to the new database', speaker: 2, isFinal: true, delayMs: 200 },
  { transcript: 'Nice work on that migration', speaker: 0, isFinal: true, delayMs: 200 },
  { transcript: 'Any issues with the rollout', speaker: 1, isFinal: false, delayMs: 200 },
  { transcript: 'Any issues with the rollout so far', speaker: 1, isFinal: true, delayMs: 150 },
  { transcript: 'No issues everything is stable', speaker: 2, isFinal: true, delayMs: 200 },
  { transcript: 'Great lets move on', speaker: 0, isFinal: true, delayMs: 200 },
  { transcript: 'I will cover frontend next', speaker: 1, isFinal: true, delayMs: 200 },
  { transcript: 'We redesigned the settings page', speaker: 1, isFinal: true, delayMs: 200 },
  { transcript: 'Looks clean nice job', speaker: 0, isFinal: true, delayMs: 200 },
];

/**
 * RAPID_INTERIM_UPDATES — 7 messages testing interim→final replacement.
 * Multiple interim results followed by final for same utterance.
 */
export const RAPID_INTERIM_UPDATES: ConversationScenario = [
  { transcript: 'The', speaker: 0, isFinal: false, delayMs: 100 },
  { transcript: 'The quick', speaker: 0, isFinal: false, delayMs: 80 },
  { transcript: 'The quick brown fox', speaker: 0, isFinal: false, delayMs: 80 },
  { transcript: 'The quick brown fox jumps', speaker: 0, isFinal: true, delayMs: 100 },
  { transcript: 'Over', speaker: 1, isFinal: false, delayMs: 150 },
  { transcript: 'Over the lazy', speaker: 1, isFinal: false, delayMs: 80 },
  { transcript: 'Over the lazy dog', speaker: 1, isFinal: true, delayMs: 100 },
];
