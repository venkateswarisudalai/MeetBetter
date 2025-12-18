use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const ASSEMBLYAI_REALTIME_URL: &str = "wss://api.assemblyai.com/v2/realtime/ws";

#[derive(Serialize)]
struct CreateRealtimeTokenRequest {
    expires_in: u32,
}

#[derive(Deserialize)]
struct CreateRealtimeTokenResponse {
    token: String,
}

async fn get_temporary_token(api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.assemblyai.com/v2/realtime/token")
        .header("Authorization", api_key)
        .json(&CreateRealtimeTokenRequest { expires_in: 3600 })
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Failed to get token: {}", error_text));
    }

    let token_response: CreateRealtimeTokenResponse = response.json().await?;
    Ok(token_response.token)
}

#[derive(Debug, Deserialize)]
#[serde(tag = "message_type")]
pub enum RealtimeMessage {
    SessionBegins {
        session_id: String,
    },
    PartialTranscript {
        text: String,
    },
    FinalTranscript {
        text: String,
    },
    SessionTerminated,
    #[serde(other)]
    Unknown,
}

pub struct RealtimeTranscriber {
    is_running: Arc<AtomicBool>,
    transcript_sender: mpsc::Sender<String>,
}

impl RealtimeTranscriber {
    pub fn new(transcript_sender: mpsc::Sender<String>) -> Self {
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

        eprintln!("Connecting to AssemblyAI with sample_rate: {}", sample_rate);

        // Get temporary token for real-time API
        eprintln!("Getting temporary token...");
        let temp_token = get_temporary_token(api_key).await?;
        eprintln!("Got temporary token");

        // Connect to AssemblyAI WebSocket with temporary token
        let url = format!("{}?sample_rate={}&token={}", ASSEMBLYAI_REALTIME_URL, sample_rate, temp_token);
        eprintln!("Connecting to WebSocket...");
        let (ws_stream, _) = connect_async(&url).await.map_err(|e| {
            eprintln!("WebSocket connection failed: {}", e);
            anyhow!("WebSocket connection failed: {}", e)
        })?;

        eprintln!("Connected to AssemblyAI WebSocket!");
        let (mut write, mut read) = ws_stream.split();

        // Channel for audio data
        let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<u8>>(100);
        let is_running = self.is_running.clone();
        let transcript_sender = self.transcript_sender.clone();

        // Audio capture thread - capture raw PCM and send as base64
        let is_running_audio = is_running.clone();
        let sample_rate_for_buffer = sample_rate;

        std::thread::spawn(move || {
            let buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            let buffer_clone = buffer.clone();
            let audio_tx_f32 = audio_tx.clone();

            // Calculate buffer size for ~250ms of audio
            let buffer_size = (sample_rate_for_buffer as usize / 4) * 2; // 250ms worth of 16-bit samples

            let err_fn = |err| eprintln!("Audio stream error: {}", err);

            let stream_result = match config.sample_format() {
                cpal::SampleFormat::F32 => {
                    let buffer_clone_inner = buffer_clone.clone();
                    let audio_tx_inner = audio_tx_f32.clone();
                    device.build_input_stream(
                        &config.into(),
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            // Convert f32 to i16 bytes (little endian)
                            let bytes: Vec<u8> = data.iter()
                                .flat_map(|&s| {
                                    let sample_i16 = (s * 32767.0) as i16;
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
                },
                cpal::SampleFormat::I16 => {
                    let buffer_clone_inner = buffer_clone.clone();
                    let audio_tx_inner = audio_tx.clone();
                    device.build_input_stream(
                        &config.into(),
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            let bytes: Vec<u8> = data.iter()
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
                },
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

        // Task to send audio to WebSocket
        let is_running_send = is_running.clone();
        tokio::spawn(async move {
            eprintln!("Audio sender task started");
            while is_running_send.load(Ordering::SeqCst) {
                match audio_rx.recv().await {
                    Some(bytes) => {
                        let encoded = BASE64.encode(&bytes);
                        let msg = serde_json::json!({ "audio_data": encoded });
                        if let Err(e) = write.send(Message::Text(msg.to_string())).await {
                            eprintln!("Failed to send audio: {}", e);
                            break;
                        }
                    }
                    None => break,
                }
            }
            // Send terminate message
            let _ = write.send(Message::Text(r#"{"terminate_session": true}"#.to_string())).await;
            eprintln!("Audio sender task ended");
        });

        // Task to receive transcripts
        let is_running_recv = is_running.clone();
        tokio::spawn(async move {
            eprintln!("Transcript receiver task started");
            while is_running_recv.load(Ordering::SeqCst) {
                match read.next().await {
                    Some(Ok(Message::Text(text))) => {
                        eprintln!("Received from AssemblyAI: {}", &text[..text.len().min(100)]);
                        match serde_json::from_str::<RealtimeMessage>(&text) {
                            Ok(msg) => {
                                match msg {
                                    RealtimeMessage::SessionBegins { session_id } => {
                                        eprintln!("Session started: {}", session_id);
                                    }
                                    RealtimeMessage::FinalTranscript { text } => {
                                        if !text.is_empty() {
                                            eprintln!("Final transcript: {}", text);
                                            let _ = transcript_sender.send(text).await;
                                        }
                                    }
                                    RealtimeMessage::PartialTranscript { text } => {
                                        if !text.is_empty() {
                                            eprintln!("Partial: {}", text);
                                        }
                                    }
                                    RealtimeMessage::SessionTerminated => {
                                        eprintln!("Session terminated");
                                        break;
                                    }
                                    RealtimeMessage::Unknown => {
                                        // Ignore unknown messages
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse message: {}", e);
                            }
                        }
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
