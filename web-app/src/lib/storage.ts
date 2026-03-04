export interface Meeting {
  id: string;
  title: string;
  date: string;
  duration: number; // seconds
  transcript: string;
  summary?: string;
  context?: string;
}

const KEYS_STORAGE = 'vantage_keys';
const MEETINGS_STORAGE = 'vantage_meetings';

export function getApiKeys(): { deepgram: string; groq: string } {
  const raw = localStorage.getItem(KEYS_STORAGE);
  if (raw) return JSON.parse(raw);
  return { deepgram: '', groq: '' };
}

export function saveApiKeys(keys: { deepgram: string; groq: string }) {
  localStorage.setItem(KEYS_STORAGE, JSON.stringify(keys));
}

export function getMeetings(): Meeting[] {
  const raw = localStorage.getItem(MEETINGS_STORAGE);
  if (raw) return JSON.parse(raw);
  return [];
}

export function saveMeeting(meeting: Meeting) {
  const meetings = getMeetings();
  meetings.unshift(meeting);
  localStorage.setItem(MEETINGS_STORAGE, JSON.stringify(meetings));
}

export function deleteMeeting(id: string) {
  const meetings = getMeetings().filter(m => m.id !== id);
  localStorage.setItem(MEETINGS_STORAGE, JSON.stringify(meetings));
}
