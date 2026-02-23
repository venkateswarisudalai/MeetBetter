use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::system_audio::{
    get_system_audio_backend, get_system_audio_device_name, AudioSource,
    SystemAudioCaptureMethod, start_screencapturekit_capture,
};

#[derive(Debug, Deserialize)]
struct DeepgramResponse {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    channel: Option<Channel>,
    channel_index: Option<Vec<usize>>,  // [channel_idx, num_channels] for multichannel
    is_final: Option<bool>,
    speech_final: Option<bool>,
}

/// Transcript message sent to the UI
#[derive(Debug, Clone)]
pub struct TranscriptMessage {
    pub text: String,
    pub is_final: bool,
    pub speaker: Option<u32>,  // Speaker ID from diarization (0, 1, 2, etc.)
    pub source: AudioSource,   // Which audio source this came from
}

#[derive(Debug, Deserialize)]
struct Channel {
    alternatives: Vec<Alternative>,
}

#[derive(Debug, Deserialize)]
struct Alternative {
    transcript: String,
    confidence: f32,
    #[serde(default)]
    words: Vec<Word>,
}

#[derive(Debug, Deserialize)]
struct Word {
    word: String,
    #[serde(default)]
    speaker: Option<u32>,
}

/// Audio setup info acquired once, reused across WebSocket retries.
/// This avoids re-requesting microphone permission on each retry.
pub struct AudioSetup {
    pub sample_rate: u32,
    pub has_system_audio: bool,
    pub channels: u16,
    pub capture_method: SystemAudioCaptureMethod,
    /// Sender side: audio capture thread sends audio bytes here
    audio_rx: mpsc::Receiver<Vec<u8>>,
    /// Keep audio capture thread alive via is_running flag
    _is_running_audio: Arc<AtomicBool>,
}

pub struct DeepgramTranscriber {
    is_running: Arc<AtomicBool>,
    transcript_sender: mpsc::Sender<TranscriptMessage>,
}

impl DeepgramTranscriber {
    pub fn new(transcript_sender: mpsc::Sender<TranscriptMessage>) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            transcript_sender,
        }
    }

    /// Set up audio devices and start capturing. Call this ONCE before retrying WebSocket connections.
    /// This is the step that requests microphone permission from macOS.
    pub fn setup_audio(&self) -> Result<AudioSetup> {
        // Get audio devices (this triggers the macOS microphone permission prompt)
        let host = cpal::default_host();
        let mic_device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("No microphone found"))?;
        let mic_config = mic_device.default_input_config()?;
        let sample_rate = mic_config.sample_rate().0;

        // Determine system audio capture method
        let capture_method = get_system_audio_backend();
        let has_system_audio = capture_method != SystemAudioCaptureMethod::None;

        // Determine channels: 2 for stereo (mic + system), 1 for mono (mic only)
        let channels = if has_system_audio { 2 } else { 1 };

        eprintln!(
            "Deepgram: sample_rate={}, channels={}, capture_method={} ({})",
            sample_rate,
            channels,
            capture_method,
            if has_system_audio { "stereo: mic + system" } else { "mono: mic only" }
        );

        if has_system_audio {
            eprintln!("Multichannel mode enabled: Channel 0 = You (mic), Channel 1 = Participants (system audio)");
        } else {
            eprintln!("No system audio available. Grant Screen Recording permission or install BlackHole for speaker separation.");
        }

        // Channel for audio data
        let (audio_tx, audio_rx) = mpsc::channel::<Vec<u8>>(100);
        let is_running_audio = Arc::new(AtomicBool::new(true));
        let is_running_audio_clone = is_running_audio.clone();

        // Audio capture thread — started once, lives until stop() is called
        std::thread::spawn(move || {
            let samples_per_100ms = sample_rate as usize / 10;
            let buffer_size_mono = samples_per_100ms * 2; // 16-bit = 2 bytes per sample

            if has_system_audio {
                // STEREO MODE: Capture mic and system audio separately, interleave
                let mic_buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
                let system_buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));

                // Build mic stream (same for both SCK and BlackHole)
                let mic_buffer_clone = mic_buffer.clone();
                let mic_config_stream = cpal::StreamConfig {
                    channels: 1,
                    sample_rate: cpal::SampleRate(sample_rate),
                    buffer_size: cpal::BufferSize::Default,
                };

                let mic_stream = mic_device.build_input_stream(
                    &mic_config_stream,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let samples: Vec<i16> = data
                            .iter()
                            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                            .collect();
                        if let Ok(mut buf) = mic_buffer_clone.lock() {
                            buf.extend(samples);
                        }
                    },
                    |err| eprintln!("Mic stream error: {}", err),
                    None,
                );

                // Start system audio capture — branch on method
                let system_stream = match capture_method {
                    SystemAudioCaptureMethod::ScreenCaptureKit => {
                        // ScreenCaptureKit fills system_buffer via its own callback
                        match start_screencapturekit_capture(
                            system_buffer.clone(),
                            sample_rate,
                            is_running_audio_clone.clone(),
                        ) {
                            Ok(()) => eprintln!("ScreenCaptureKit system audio capture active"),
                            Err(e) => eprintln!("ScreenCaptureKit capture failed: {}. System audio channel will be silent.", e),
                        }
                        None // no cpal stream to hold
                    }
                    SystemAudioCaptureMethod::BlackHole => {
                        // BlackHole: use cpal to read from the virtual device
                        let system_buffer_clone = system_buffer.clone();
                        let sys_device = crate::system_audio::get_blackhole_device().unwrap();
                        let sys_config = cpal::StreamConfig {
                            channels: 2, // BlackHole is typically stereo
                            sample_rate: cpal::SampleRate(sample_rate),
                            buffer_size: cpal::BufferSize::Default,
                        };

                        match sys_device.build_input_stream(
                            &sys_config,
                            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                // Mix stereo to mono (average left and right)
                                let samples: Vec<i16> = data
                                    .chunks(2)
                                    .map(|chunk| {
                                        let left = chunk.get(0).copied().unwrap_or(0.0);
                                        let right = chunk.get(1).copied().unwrap_or(0.0);
                                        let mono = (left + right) / 2.0;
                                        (mono.clamp(-1.0, 1.0) * 32767.0) as i16
                                    })
                                    .collect();
                                if let Ok(mut buf) = system_buffer_clone.lock() {
                                    buf.extend(samples);
                                }
                            },
                            |err| eprintln!("System audio stream error: {}", err),
                            None,
                        ) {
                            Ok(stream) => Some(stream),
                            Err(e) => {
                                eprintln!("Failed to build BlackHole stream: {}", e);
                                None
                            }
                        }
                    }
                    SystemAudioCaptureMethod::None => None,
                };

                // Start streams
                if let Ok(ref stream) = mic_stream {
                    let _ = stream.play();
                    eprintln!("Microphone capture started (Channel 0 = You)");
                }

                if let Some(ref stream) = system_stream {
                    let _ = stream.play();
                    eprintln!("BlackHole system audio capture started (Channel 1 = Participants)");
                }

                // Main loop: interleave audio and send (identical for SCK and BlackHole)
                while is_running_audio_clone.load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(50));

                    let mic_samples: Vec<i16>;
                    let system_samples: Vec<i16>;

                    // Get mic samples
                    {
                        let mut buf = mic_buffer.lock().unwrap();
                        if buf.len() >= samples_per_100ms {
                            mic_samples = buf.drain(..samples_per_100ms).collect();
                        } else {
                            continue;
                        }
                    }

                    // Get system samples
                    {
                        let mut buf = system_buffer.lock().unwrap();
                        if buf.len() >= samples_per_100ms {
                            system_samples = buf.drain(..samples_per_100ms).collect();
                        } else {
                            // Pad with silence if system audio is behind
                            system_samples = vec![0i16; samples_per_100ms];
                        }
                    }

                    // Interleave as stereo: [mic_0, sys_0, mic_1, sys_1, ...]
                    // Channel 0 (left) = Microphone = You
                    // Channel 1 (right) = System Audio = Participants
                    let mut stereo_bytes: Vec<u8> = Vec::with_capacity(samples_per_100ms * 4);
                    for i in 0..samples_per_100ms {
                        let mic_sample = mic_samples.get(i).copied().unwrap_or(0);
                        let sys_sample = system_samples.get(i).copied().unwrap_or(0);

                        // Left channel (mic) - Channel 0
                        stereo_bytes.extend_from_slice(&mic_sample.to_le_bytes());
                        // Right channel (system) - Channel 1
                        stereo_bytes.extend_from_slice(&sys_sample.to_le_bytes());
                    }

                    if audio_tx.blocking_send(stereo_bytes).is_err() {
                        break;
                    }
                }
            } else {
                // MONO MODE: Just capture mic (original behavior)
                let buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
                let buffer_clone = buffer.clone();

                let err_fn = |err| eprintln!("Audio stream error: {}", err);

                let stream_result = match mic_config.sample_format() {
                    cpal::SampleFormat::F32 => {
                        let buffer_clone_inner = buffer_clone.clone();
                        let audio_tx_inner = audio_tx.clone();
                        mic_device.build_input_stream(
                            &mic_config.into(),
                            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                let bytes: Vec<u8> = data
                                    .iter()
                                    .flat_map(|&s| {
                                        let sample_i16 = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                                        sample_i16.to_le_bytes().to_vec()
                                    })
                                    .collect();

                                if let Ok(mut buf) = buffer_clone_inner.lock() {
                                    buf.extend(bytes);
                                    if buf.len() >= buffer_size_mono {
                                        let chunk: Vec<u8> = buf.drain(..).collect();
                                        let _ = audio_tx_inner.blocking_send(chunk);
                                    }
                                }
                            },
                            err_fn,
                            None,
                        )
                    }
                    cpal::SampleFormat::I16 => {
                        let buffer_clone_inner = buffer_clone.clone();
                        let audio_tx_inner = audio_tx.clone();
                        mic_device.build_input_stream(
                            &mic_config.into(),
                            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                                let bytes: Vec<u8> = data
                                    .iter()
                                    .flat_map(|&s| s.to_le_bytes().to_vec())
                                    .collect();

                                if let Ok(mut buf) = buffer_clone_inner.lock() {
                                    buf.extend(bytes);
                                    if buf.len() >= buffer_size_mono {
                                        let chunk: Vec<u8> = buf.drain(..).collect();
                                        let _ = audio_tx_inner.blocking_send(chunk);
                                    }
                                }
                            },
                            err_fn,
                            None,
                        )
                    }
                    _ => {
                        eprintln!("Unsupported sample format");
                        return;
                    }
                };

                match stream_result {
                    Ok(stream) => {
                        if let Err(e) = stream.play() {
                            eprintln!("Failed to play stream: {}", e);
                            return;
                        }
                        eprintln!("Audio capture started!");
                        while is_running_audio_clone.load(Ordering::SeqCst) {
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                        eprintln!("Audio capture stopped");
                    }
                    Err(e) => {
                        eprintln!("Failed to build audio stream: {}", e);
                    }
                }
            }

            eprintln!("Audio capture thread ended");
        });

        Ok(AudioSetup {
            sample_rate,
            has_system_audio,
            channels,
            capture_method,
            audio_rx,
            _is_running_audio: is_running_audio,
        })
    }

    /// Connect to Deepgram and stream audio. This can be retried without re-requesting
    /// microphone permission since audio_setup was created once by setup_audio().
    pub async fn start(&self, api_key: &str, app_handle: Option<AppHandle>, audio_setup: &mut AudioSetup) -> Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Err(anyhow!("Already running"));
        }

        self.is_running.store(true, Ordering::SeqCst);

        let sample_rate = audio_setup.sample_rate;
        let has_system_audio = audio_setup.has_system_audio;
        let channels = audio_setup.channels;

        // Build WebSocket URL with multichannel support
        // multichannel=true tells Deepgram to transcribe each channel separately
        let url = if has_system_audio {
            format!(
                "wss://api.deepgram.com/v1/listen?\
                encoding=linear16&\
                sample_rate={}&\
                channels=2&\
                model=nova-2&\
                punctuate=true&\
                interim_results=true&\
                endpointing=100&\
                utterance_end_ms=1000&\
                smart_format=true&\
                vad_events=true&\
                multichannel=true",
                sample_rate
            )
        } else {
            // Fallback to mono with diarization
            format!(
                "wss://api.deepgram.com/v1/listen?\
                encoding=linear16&\
                sample_rate={}&\
                channels=1&\
                model=nova-2&\
                punctuate=true&\
                interim_results=true&\
                endpointing=100&\
                utterance_end_ms=1000&\
                smart_format=true&\
                vad_events=true&\
                diarize=true",
                sample_rate
            )
        };

        eprintln!("Connecting to Deepgram...");

        // Log API key info for debugging (first/last few chars only)
        let key_len = api_key.len();
        if key_len > 8 {
            eprintln!("API key: {}...{} (len={})", &api_key[..4], &api_key[key_len-4..], key_len);
        } else {
            eprintln!("API key seems too short: len={}", key_len);
        }

        // Build WebSocket request with proper Authorization header
        let request = tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(&url)
            .header("Authorization", format!("Token {}", api_key))
            .header("Host", "api.deepgram.com")
            .header("Upgrade", "websocket")
            .header("Connection", "Upgrade")
            .header("Sec-WebSocket-Key", tokio_tungstenite::tungstenite::handshake::client::generate_key())
            .header("Sec-WebSocket-Version", "13")
            .body(())
            .map_err(|e| {
                eprintln!("Failed to create request: {}", e);
                anyhow!("Failed to create request: {}", e)
            })?;

        let (ws_stream, response) = tokio_tungstenite::connect_async(request).await.map_err(|e| {
            eprintln!("Deepgram connection failed: {}", e);
            anyhow!("WebSocket connection failed: {}", e)
        })?;

        eprintln!("WebSocket response status: {:?}", response.status());

        eprintln!("Connected to Deepgram!");

        // Emit audio-mode event to frontend
        if let Some(ref app) = app_handle {
            let mode = if has_system_audio { "multichannel" } else { "diarize" };
            let device_name = get_system_audio_device_name().unwrap_or_default();
            let capture_method_str = audio_setup.capture_method.to_string();
            let _ = app.emit("audio-mode", serde_json::json!({
                "mode": mode,
                "system_audio_device": device_name,
                "channels": channels,
                "capture_method": capture_method_str,
            }));
        }

        let (mut write, mut read) = ws_stream.split();

        let is_running = self.is_running.clone();
        let transcript_sender = self.transcript_sender.clone();

        // Task to send audio to WebSocket — reads from the shared audio_rx
        let is_running_send = is_running.clone();
        // Take ownership of audio_rx for the sender task
        let mut audio_rx = std::mem::replace(&mut audio_setup.audio_rx, mpsc::channel::<Vec<u8>>(1).1);
        tokio::spawn(async move {
            eprintln!("Audio sender task started");
            while is_running_send.load(Ordering::SeqCst) {
                match audio_rx.recv().await {
                    Some(bytes) => {
                        if let Err(e) = write.send(Message::Binary(bytes)).await {
                            eprintln!("Failed to send audio: {}", e);
                            break;
                        }
                    }
                    None => break,
                }
            }
            let _ = write.close().await;
            eprintln!("Audio sender task ended");
        });

        // Task to receive transcripts
        let is_running_recv = is_running.clone();
        let has_system_audio_recv = has_system_audio;
        let app_handle_recv = app_handle.clone();
        tokio::spawn(async move {
            eprintln!("Transcript receiver task started");
            let mut last_interim_text_ch0 = String::new();
            let mut last_interim_text_ch1 = String::new();
            // Track whether we've seen transcription from each channel (stereo diagnostics)
            let mut seen_ch0 = false;
            let mut seen_ch1 = false;
            let mut diagnostic_logged = false;
            let start_time = std::time::Instant::now();

            while is_running_recv.load(Ordering::SeqCst) {
                match read.next().await {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<DeepgramResponse>(&text) {
                            Ok(response) => {
                                // Skip non-Results messages
                                if response.msg_type.as_deref() != Some("Results") {
                                    continue;
                                }

                                if let Some(channel) = response.channel {
                                    if let Some(alt) = channel.alternatives.first() {
                                        let transcript_text = alt.transcript.trim();
                                        if transcript_text.is_empty() {
                                            continue;
                                        }

                                        // Determine audio source from channel index
                                        let (source, last_interim) = if has_system_audio_recv {
                                            // Multichannel mode: channel_index[0] tells us which channel
                                            let channel_idx = match response.channel_index.as_ref().and_then(|arr| arr.first().copied()) {
                                                Some(idx) => idx,
                                                None => {
                                                    eprintln!("WARNING: channel_index missing from Deepgram multichannel response, defaulting to channel 0 (mic)");
                                                    0
                                                }
                                            };

                                            if channel_idx == 0 {
                                                (AudioSource::Microphone, &mut last_interim_text_ch0)
                                            } else {
                                                (AudioSource::SystemAudio, &mut last_interim_text_ch1)
                                            }
                                        } else {
                                            // Mono mode: use diarization speaker IDs
                                            // Cannot reliably determine "You" vs "Participant" without
                                            // separate audio channels - use diarized speaker labels
                                            let speaker_id = alt.words.first()
                                                .and_then(|w| w.speaker)
                                                .unwrap_or(0);
                                            let interim_tracker = if speaker_id % 2 == 0 {
                                                &mut last_interim_text_ch0
                                            } else {
                                                &mut last_interim_text_ch1
                                            };
                                            (AudioSource::Diarized(speaker_id), interim_tracker)
                                        };

                                        // Track channel activity for diagnostics (stereo mode)
                                        if has_system_audio_recv {
                                            match source {
                                                AudioSource::Microphone => seen_ch0 = true,
                                                AudioSource::SystemAudio => seen_ch1 = true,
                                                _ => {}
                                            }
                                            // After 30 seconds, warn if system audio channel is silent
                                            if !diagnostic_logged && start_time.elapsed().as_secs() > 30 && seen_ch0 && !seen_ch1 {
                                                eprintln!("WARNING: System audio channel has no audio after 30s.");
                                                eprintln!("  Ensure Screen Recording permission is granted in System Settings > Privacy & Security.");
                                                eprintln!("  If using BlackHole: check your Multi-Output Device configuration.");
                                                // Emit event to frontend
                                                if let Some(ref app) = app_handle_recv {
                                                    let _ = app.emit("system-audio-silent", serde_json::json!({
                                                        "message": "System audio channel has no audio after 30s. Check that Screen Recording permission is granted, or verify your audio setup.",
                                                    }));
                                                }
                                                diagnostic_logged = true;
                                            }
                                        }

                                        // Extract speaker from words for additional context
                                        let speaker = alt.words.first().and_then(|w| w.speaker);

                                        let is_final = response.is_final.unwrap_or(false);
                                        let speech_final = response.speech_final.unwrap_or(false);

                                        let source_label_owned;
                                        let source_label = match source {
                                            AudioSource::Microphone => "You",
                                            AudioSource::SystemAudio => "Participant",
                                            AudioSource::Diarized(id) => {
                                                source_label_owned = format!("Speaker {}", id + 1);
                                                &source_label_owned
                                            }
                                        };

                                        if is_final || speech_final {
                                            eprintln!("Deepgram [FINAL] {} (ch={:?}): {}",
                                                source_label,
                                                response.channel_index,
                                                transcript_text
                                            );
                                            let _ = transcript_sender.send(TranscriptMessage {
                                                text: transcript_text.to_string(),
                                                is_final: true,
                                                speaker,
                                                source,
                                            }).await;
                                            last_interim.clear();
                                        } else if transcript_text != *last_interim {
                                            eprintln!("Deepgram [interim] {} (ch={:?}): {}",
                                                source_label,
                                                response.channel_index,
                                                transcript_text
                                            );
                                            let _ = transcript_sender.send(TranscriptMessage {
                                                text: transcript_text.to_string(),
                                                is_final: false,
                                                speaker,
                                                source,
                                            }).await;
                                            *last_interim = transcript_text.to_string();
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                if !text.contains("Metadata") && !text.contains("SpeechStarted") {
                                    eprintln!("Parse warning: {}", e);
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        eprintln!("WebSocket closed by server");
                        break;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        eprintln!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        eprintln!("WebSocket closed");
                        break;
                    }
                }
            }
            eprintln!("Transcript receiver task ended");
        });

        eprintln!("Deepgram transcriber completed normally");
        Ok(())
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcript_message_final() {
        let msg = TranscriptMessage {
            text: "Hello world".to_string(),
            is_final: true,
            speaker: Some(0),
            source: AudioSource::Microphone,
        };
        assert!(msg.is_final);
        assert_eq!(msg.text, "Hello world");
        assert_eq!(msg.source, AudioSource::Microphone);
    }

    #[test]
    fn test_transcript_message_interim() {
        let msg = TranscriptMessage {
            text: "Hello...".to_string(),
            is_final: false,
            speaker: Some(1),
            source: AudioSource::SystemAudio,
        };
        assert!(!msg.is_final);
        assert_eq!(msg.text, "Hello...");
        assert_eq!(msg.source, AudioSource::SystemAudio);
    }
}
