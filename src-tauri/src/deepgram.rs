use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::system_audio::{get_system_audio_device, AudioSource};

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

    pub async fn start(&self, api_key: &str) -> Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Err(anyhow!("Already running"));
        }

        self.is_running.store(true, Ordering::SeqCst);

        // Get audio devices
        let host = cpal::default_host();
        let mic_device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("No microphone found"))?;
        let mic_config = mic_device.default_input_config()?;
        let sample_rate = mic_config.sample_rate().0;

        // Check for system audio device (BlackHole, etc.)
        let system_device = get_system_audio_device();
        let has_system_audio = system_device.is_some();

        // Determine channels: 2 for stereo (mic + system), 1 for mono (mic only)
        let channels = if has_system_audio { 2 } else { 1 };

        eprintln!(
            "Deepgram: sample_rate={}, channels={} ({})",
            sample_rate,
            channels,
            if has_system_audio { "stereo: mic + system" } else { "mono: mic only" }
        );

        if has_system_audio {
            eprintln!("Multichannel mode enabled: Channel 0 = You (mic), Channel 1 = Participants (system audio)");
        } else {
            eprintln!("No system audio device found. Install BlackHole for speaker separation.");
            eprintln!("  Download: https://github.com/ExistentialAudio/BlackHole");
        }

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
        let (mut write, mut read) = ws_stream.split();

        // Channel for audio data
        let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<u8>>(100);
        let is_running = self.is_running.clone();
        let transcript_sender = self.transcript_sender.clone();

        // Audio capture thread
        let is_running_audio = is_running.clone();

        std::thread::spawn(move || {
            // Buffer size for ~100ms of audio (per channel)
            let samples_per_100ms = sample_rate as usize / 10;
            let buffer_size_mono = samples_per_100ms * 2; // 16-bit = 2 bytes per sample
            let buffer_size_stereo = samples_per_100ms * 4; // 2 channels * 2 bytes

            if has_system_audio {
                // STEREO MODE: Capture mic and system audio separately, interleave
                let mic_buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
                let system_buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));

                // Build mic stream
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

                // Build system audio stream
                let system_buffer_clone = system_buffer.clone();
                let sys_device = system_device.unwrap();
                let sys_config = cpal::StreamConfig {
                    channels: 2, // BlackHole is typically stereo
                    sample_rate: cpal::SampleRate(sample_rate),
                    buffer_size: cpal::BufferSize::Default,
                };

                let system_stream = sys_device.build_input_stream(
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
                );

                // Start streams
                if let Ok(ref stream) = mic_stream {
                    let _ = stream.play();
                    eprintln!("Microphone capture started (Channel 0 = You)");
                }

                if let Ok(ref stream) = system_stream {
                    let _ = stream.play();
                    eprintln!("System audio capture started (Channel 1 = Participants)");
                }

                // Main loop: interleave audio and send
                while is_running_audio.load(Ordering::SeqCst) {
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
                        while is_running_audio.load(Ordering::SeqCst) {
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

        // Task to send audio to WebSocket
        let is_running_send = is_running.clone();
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
        tokio::spawn(async move {
            eprintln!("Transcript receiver task started");
            let mut last_interim_text_ch0 = String::new();
            let mut last_interim_text_ch1 = String::new();

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
                                            let channel_idx = response.channel_index
                                                .as_ref()
                                                .and_then(|arr| arr.first().copied())
                                                .unwrap_or(0);

                                            if channel_idx == 0 {
                                                (AudioSource::Microphone, &mut last_interim_text_ch0)
                                            } else {
                                                (AudioSource::SystemAudio, &mut last_interim_text_ch1)
                                            }
                                        } else {
                                            // Mono mode: use diarization speaker ID
                                            // Speaker 0 assumed to be you (first detected)
                                            let speaker = alt.words.first().and_then(|w| w.speaker);
                                            if speaker == Some(0) {
                                                (AudioSource::Microphone, &mut last_interim_text_ch0)
                                            } else {
                                                (AudioSource::SystemAudio, &mut last_interim_text_ch1)
                                            }
                                        };

                                        // Extract speaker from words for additional context
                                        let speaker = alt.words.first().and_then(|w| w.speaker);

                                        let is_final = response.is_final.unwrap_or(false);
                                        let speech_final = response.speech_final.unwrap_or(false);

                                        let source_label = match source {
                                            AudioSource::Microphone => "You",
                                            AudioSource::SystemAudio => "Participant",
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
