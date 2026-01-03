# Test Audio Files for Mock Transcription

This directory contains audio files for testing the mock transcription feature.

## Required Files

Place the following WAV files in this directory:

- `mock_mic.wav` - Audio simulating the user's microphone input (your voice)
- `mock_speaker.wav` - Audio simulating other meeting participants (system audio)

## Usage

From the browser console in the running app:

```javascript
// Using Tauri's invoke API
const { invoke } = window.__TAURI__.core;

await invoke('start_mock_transcription', {
  micPath: '/path/to/src-tauri/test_audio/mock_mic.wav',
  speakerPath: '/path/to/src-tauri/test_audio/mock_speaker.wav'
});

// To stop:
await invoke('stop_mock_transcription');
```

## What Happens

1. The mock transcription reads both audio files
2. Transcribes `mock_mic.wav` and emits it as speaker "You"
3. Transcribes `mock_speaker.wav` and emits it as speaker "Participant"
4. Both transcripts appear in the UI with different speaker labels
5. AI reply suggestions are triggered automatically

## Audio File Requirements

- Format: WAV (recommended) or other formats supported by Groq Whisper
- Duration: Any length, but shorter files (< 30 seconds) are faster for testing
- Sample rate: 16kHz or 48kHz recommended
- Channels: Mono or stereo

## Creating Test Audio

You can create test audio using:
- macOS: QuickTime Player > File > New Audio Recording
- Free online tools: Various text-to-speech services
- Command line: `say -o mock_mic.wav "Hello, this is a test message"`
