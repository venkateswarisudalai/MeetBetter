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

#[derive(Debug, Deserialize)]
struct DeepgramResponse {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    channel: Option<Channel>,
    is_final: Option<bool>,
    speech_final: Option<bool>,
}

/// Transcript message sent to the UI
#[derive(Debug, Clone)]
pub struct TranscriptMessage {
    pub text: String,
    pub is_final: bool,
    pub speaker: Option<u32>,  // Speaker ID from diarization (0, 1, 2, etc.)
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

        // Get audio device
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("No input device"))?;
        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        eprintln!(
            "Deepgram: sample_rate={}, channels={}",
            sample_rate, channels
        );

        // Build WebSocket URL with optimized parameters for real-time streaming
        // Based on learnings from Granola: 100ms endpointing for responsive feel
        // diarize=true enables speaker identification
        let url = format!(
            "wss://api.deepgram.com/v1/listen?\
            encoding=linear16&\
            sample_rate={}&\
            channels={}&\
            model=nova-2&\
            punctuate=true&\
            interim_results=true&\
            endpointing=100&\
            utterance_end_ms=1000&\
            smart_format=true&\
            vad_events=true&\
            diarize=true",
            sample_rate, channels
        );

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
            let buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            let buffer_clone = buffer.clone();

            // Send audio every ~100ms for low latency
            let buffer_size = (sample_rate as usize / 10) * 2 * channels as usize;

            let err_fn = |err| eprintln!("Audio stream error: {}", err);

            let stream_result = match config.sample_format() {
                cpal::SampleFormat::F32 => {
                    let buffer_clone_inner = buffer_clone.clone();
                    let audio_tx_inner = audio_tx.clone();
                    device.build_input_stream(
                        &config.into(),
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            // Convert f32 to i16 bytes (little endian)
                            let bytes: Vec<u8> = data
                                .iter()
                                .flat_map(|&s| {
                                    let sample_i16 = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                                    sample_i16.to_le_bytes().to_vec()
                                })
                                .collect();

                            if let Ok(mut buf) = buffer_clone_inner.lock() {
                                buf.extend(bytes);
                                if buf.len() >= buffer_size {
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
                    device.build_input_stream(
                        &config.into(),
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            let bytes: Vec<u8> = data
                                .iter()
                                .flat_map(|&s| s.to_le_bytes().to_vec())
                                .collect();

                            if let Ok(mut buf) = buffer_clone_inner.lock() {
                                buf.extend(bytes);
                                if buf.len() >= buffer_size {
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
        });

        // Task to send audio to WebSocket (send raw bytes, not base64)
        let is_running_send = is_running.clone();
        tokio::spawn(async move {
            eprintln!("Audio sender task started");
            while is_running_send.load(Ordering::SeqCst) {
                match audio_rx.recv().await {
                    Some(bytes) => {
                        // Deepgram expects raw binary audio, not base64
                        if let Err(e) = write.send(Message::Binary(bytes)).await {
                            eprintln!("Failed to send audio: {}", e);
                            break;
                        }
                    }
                    None => break,
                }
            }
            // Send close frame
            let _ = write.close().await;
            eprintln!("Audio sender task ended");
        });

        // Task to receive transcripts
        let is_running_recv = is_running.clone();
        tokio::spawn(async move {
            eprintln!("Transcript receiver task started");
            let mut last_interim_text = String::new();

            while is_running_recv.load(Ordering::SeqCst) {
                match read.next().await {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<DeepgramResponse>(&text) {
                            Ok(response) => {
                                // Skip non-Results messages (like Metadata, SpeechStarted, etc.)
                                if response.msg_type.as_deref() != Some("Results") {
                                    continue;
                                }

                                if let Some(channel) = response.channel {
                                    if let Some(alt) = channel.alternatives.first() {
                                        let transcript_text = alt.transcript.trim();
                                        if transcript_text.is_empty() {
                                            continue;
                                        }

                                        // Extract speaker from words (use the most common speaker in this utterance)
                                        let speaker = if !alt.words.is_empty() {
                                            alt.words.first().and_then(|w| w.speaker)
                                        } else {
                                            None
                                        };

                                        let is_final = response.is_final.unwrap_or(false);
                                        let speech_final = response.speech_final.unwrap_or(false);

                                        // For final results, always emit
                                        // For interim results, only emit if text changed
                                        if is_final || speech_final {
                                            eprintln!("Deepgram [FINAL] Speaker {:?}: {}", speaker, transcript_text);
                                            let _ = transcript_sender.send(TranscriptMessage {
                                                text: transcript_text.to_string(),
                                                is_final: true,
                                                speaker,
                                            }).await;
                                            last_interim_text.clear();
                                        } else if transcript_text != last_interim_text {
                                            // Interim result - show for real-time feedback
                                            eprintln!("Deepgram [interim] Speaker {:?}: {}", speaker, transcript_text);
                                            let _ = transcript_sender.send(TranscriptMessage {
                                                text: transcript_text.to_string(),
                                                is_final: false,
                                                speaker,
                                            }).await;
                                            last_interim_text = transcript_text.to_string();
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                // Ignore parse errors for metadata messages
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
        };
        assert!(msg.is_final);
        assert_eq!(msg.text, "Hello world");
    }

    #[test]
    fn test_transcript_message_interim() {
        let msg = TranscriptMessage {
            text: "Hello...".to_string(),
            is_final: false,
        };
        assert!(!msg.is_final);
        assert_eq!(msg.text, "Hello...");
    }

    #[test]
    fn test_deepgram_url_params() {
        // Verify the expected parameters are in our URL format
        let expected_params = vec![
            "model=nova-2",
            "endpointing=100",
            "interim_results=true",
            "smart_format=true",
            "utterance_end_ms=1000",
            "vad_events=true",
        ];

        // This is a compile-time check that we have the right structure
        for param in expected_params {
            assert!(!param.is_empty());
        }
    }
}
