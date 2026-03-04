export interface TranscriptWord {
  word: string;
  start: number;
  end: number;
  confidence: number;
  speaker?: number;
}

export interface TranscriptResult {
  text: string;
  words: TranscriptWord[];
  isFinal: boolean;
  speaker?: number;
}

export type OnTranscript = (result: TranscriptResult) => void;
export type OnError = (error: string) => void;
export type OnStatus = (status: string) => void;

export interface DeepgramOptions {
  captureMic: boolean;
  captureTab: boolean;
}

export class DeepgramClient {
  private ws: WebSocket | null = null;
  private micStream: MediaStream | null = null;
  private tabStream: MediaStream | null = null;
  private mediaRecorder: MediaRecorder | null = null;
  private audioContext: AudioContext | null = null;
  private onTranscript: OnTranscript;
  private onError: OnError;
  private onStatus: OnStatus;
  private apiKey: string;
  private options: DeepgramOptions;

  constructor(
    apiKey: string,
    onTranscript: OnTranscript,
    onError?: OnError,
    onStatus?: OnStatus,
    options?: DeepgramOptions,
  ) {
    this.apiKey = apiKey;
    this.onTranscript = onTranscript;
    this.onError = onError || (() => {});
    this.onStatus = onStatus || (() => {});
    this.options = options || { captureMic: true, captureTab: false };
  }

  async start(): Promise<void> {
    let combinedStream: MediaStream;

    try {
      // Get tab/screen audio first (requires user gesture + picker)
      if (this.options.captureTab) {
        this.onStatus('Pick a Chrome Tab and check "Share audio"...');
        this.tabStream = await navigator.mediaDevices.getDisplayMedia({
          video: true,
          audio: true,
          // @ts-ignore — preferCurrentTab is a Chrome-specific hint
          preferCurrentTab: false,
        });

        // Check if we actually got audio tracks
        const audioTracks = this.tabStream.getAudioTracks();
        if (audioTracks.length === 0) {
          // User probably selected a screen/window instead of a tab, or didn't check "Share audio"
          this.tabStream.getTracks().forEach(t => t.stop());
          this.tabStream = null;

          if (this.options.captureMic) {
            this.onStatus('No tab audio captured (select a Chrome Tab + check "Share audio"). Using mic only...');
          } else {
            throw new Error('No audio captured. Select a Chrome Tab (not screen/window) and check "Share audio".');
          }
        } else {
          // Discard video tracks — we only want audio
          this.tabStream.getVideoTracks().forEach(t => t.stop());
        }
      }

      // Get mic audio
      if (this.options.captureMic) {
        this.onStatus('Requesting microphone...');
        this.micStream = await navigator.mediaDevices.getUserMedia({
          audio: {
            echoCancellation: true,
            noiseSuppression: true,
          },
        });
      }

      // Mix audio streams if we have both
      if (this.micStream && this.tabStream?.getAudioTracks().length) {
        this.audioContext = new AudioContext();
        const dest = this.audioContext.createMediaStreamDestination();
        const micSource = this.audioContext.createMediaStreamSource(this.micStream);
        micSource.connect(dest);
        const tabSource = this.audioContext.createMediaStreamSource(this.tabStream);
        tabSource.connect(dest);
        combinedStream = dest.stream;
        this.onStatus('Mic + tab audio captured. Connecting...');
      } else if (this.tabStream?.getAudioTracks().length) {
        combinedStream = new MediaStream(this.tabStream.getAudioTracks());
        this.onStatus('Tab audio captured. Connecting...');
      } else if (this.micStream) {
        combinedStream = this.micStream;
        this.onStatus('Mic captured. Connecting...');
      } else {
        throw new Error('No audio source available.');
      }
    } catch (e: any) {
      // Clean up any partial streams
      this.stop();
      if (e.name === 'NotAllowedError' || e.name === 'AbortError') {
        throw new Error('Cancelled. Please allow audio access to start.');
      }
      throw e;
    }

    // Connect to Deepgram
    const url = 'wss://api.deepgram.com/v1/listen?model=nova-3&language=en&smart_format=true&punctuate=true&diarize=true&interim_results=true&utterance_end_ms=1500';

    this.ws = new WebSocket(url, ['token', this.apiKey]);

    this.ws.onopen = () => {
      this.onStatus('Connected — start speaking');
      this.startRecording(combinedStream);
    };

    this.ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.channel?.alternatives?.[0]) {
          const alt = data.channel.alternatives[0];
          if (alt.transcript) {
            this.onTranscript({
              text: alt.transcript,
              words: alt.words || [],
              isFinal: data.is_final,
              speaker: alt.words?.[0]?.speaker,
            });
          }
        }
      } catch {
        // ignore parse errors
      }
    };

    this.ws.onerror = () => {
      this.onError('Connection failed. Check your Deepgram API key.');
    };

    this.ws.onclose = (event) => {
      if (event.code !== 1000 && event.code !== 1005) {
        this.onError(`Disconnected (code ${event.code}). Check your API key.`);
      }
    };
  }

  private startRecording(stream: MediaStream) {
    if (!this.ws) return;

    this.mediaRecorder = new MediaRecorder(stream);

    this.mediaRecorder.addEventListener('dataavailable', (event) => {
      if (event.data.size > 0 && this.ws?.readyState === WebSocket.OPEN) {
        this.ws.send(event.data);
      }
    });

    this.mediaRecorder.start(250);
  }

  stop() {
    if (this.mediaRecorder && this.mediaRecorder.state !== 'inactive') {
      this.mediaRecorder.stop();
      this.mediaRecorder = null;
    }

    if (this.ws) {
      if (this.ws.readyState === WebSocket.OPEN) {
        this.ws.send(JSON.stringify({ type: 'CloseStream' }));
      }
      this.ws.close();
      this.ws = null;
    }

    if (this.micStream) {
      this.micStream.getTracks().forEach(t => t.stop());
      this.micStream = null;
    }

    if (this.tabStream) {
      this.tabStream.getTracks().forEach(t => t.stop());
      this.tabStream = null;
    }

    if (this.audioContext) {
      this.audioContext.close();
      this.audioContext = null;
    }
  }
}
