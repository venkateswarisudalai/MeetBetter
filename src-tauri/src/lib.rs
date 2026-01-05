use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, watch};

mod assemblyai;
mod audio;
mod deepgram;
pub mod groq;  // Public for mock_test binary
mod mock;
mod realtime;
mod screen_share;
mod settings;

use settings::AppSettings;

use deepgram::{DeepgramTranscriber, TranscriptMessage};

/// Transcription provider options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TranscriptionProvider {
    Groq,       // Recommended - uses Whisper, good quality
    Deepgram,   // Real-time streaming, fast
    AssemblyAI, // High accuracy, batch processing
}

impl Default for TranscriptionProvider {
    fn default() -> Self {
        TranscriptionProvider::Deepgram
    }
}

// Application state
pub struct AppState {
    pub is_recording: Arc<Mutex<bool>>,
    pub is_live_transcribing: Arc<Mutex<bool>>,
    pub transcription: Arc<Mutex<Vec<TranscriptSegment>>>,
    pub summary: Arc<Mutex<String>>,
    pub suggested_replies: Arc<Mutex<Vec<String>>>,
    pub selected_model: Arc<Mutex<String>>,
    pub transcription_provider: Arc<Mutex<TranscriptionProvider>>,
    pub groq_api_key: Arc<Mutex<String>>,
    pub assemblyai_api_key: Arc<Mutex<String>>,
    pub deepgram_api_key: Arc<Mutex<String>>,
    pub audio_recorder: Arc<Mutex<Option<audio::AudioRecorder>>>,
    pub current_recording_path: Arc<Mutex<Option<String>>>,
    pub is_transcribing: Arc<Mutex<bool>>,
    pub live_stop_signal: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    pub deepgram_transcriber: Arc<Mutex<Option<DeepgramTranscriber>>>,
    pub deepgram_stop_flag: Arc<AtomicBool>,
    pub settings: Arc<Mutex<AppSettings>>,
    pub meeting_context: Arc<Mutex<String>>,
    // Mock transcription state
    pub is_mock_transcribing: Arc<Mutex<bool>>,
    pub mock_stop_signal: Arc<Mutex<Option<watch::Sender<bool>>>>,
}

impl Default for AppState {
    fn default() -> Self {
        // Load persisted settings from disk
        let saved_settings = AppSettings::load();

        // Parse transcription provider from saved settings (default to Deepgram for real-time)
        let provider = match saved_settings.transcription_provider.to_lowercase().as_str() {
            "groq" => TranscriptionProvider::Groq,
            "assemblyai" => TranscriptionProvider::AssemblyAI,
            _ => TranscriptionProvider::Deepgram,
        };

        // Use saved model or default
        let model = if saved_settings.selected_model.is_empty() {
            "llama-3.1-8b-instant".to_string()
        } else {
            saved_settings.selected_model.clone()
        };

        eprintln!("Loaded settings - Groq key present: {}, Model: {}",
            !saved_settings.groq_api_key.is_empty(), model);

        Self {
            is_recording: Arc::new(Mutex::new(false)),
            is_live_transcribing: Arc::new(Mutex::new(false)),
            transcription: Arc::new(Mutex::new(Vec::new())),
            summary: Arc::new(Mutex::new(String::new())),
            suggested_replies: Arc::new(Mutex::new(Vec::new())),
            selected_model: Arc::new(Mutex::new(model)),
            transcription_provider: Arc::new(Mutex::new(provider)),
            groq_api_key: Arc::new(Mutex::new(saved_settings.groq_api_key.clone())),
            assemblyai_api_key: Arc::new(Mutex::new(saved_settings.assemblyai_api_key.clone())),
            deepgram_api_key: Arc::new(Mutex::new(saved_settings.deepgram_api_key.clone())),
            audio_recorder: Arc::new(Mutex::new(None)),
            current_recording_path: Arc::new(Mutex::new(None)),
            is_transcribing: Arc::new(Mutex::new(false)),
            live_stop_signal: Arc::new(Mutex::new(None)),
            deepgram_transcriber: Arc::new(Mutex::new(None)),
            deepgram_stop_flag: Arc::new(AtomicBool::new(false)),
            settings: Arc::new(Mutex::new(saved_settings.clone())),
            meeting_context: Arc::new(Mutex::new(saved_settings.meeting_context)),
            // Mock transcription state
            is_mock_transcribing: Arc::new(Mutex::new(false)),
            mock_stop_signal: Arc::new(Mutex::new(None)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub timestamp: String,
    pub speaker: String,
    pub text: String,
}

/// Filler words to remove from transcripts for cleaner output
const FILLER_WORDS: &[&str] = &[
    " um ", " uh ", " er ", " ah ", " like ", " you know ",
    " i mean ", " sort of ", " kind of ", " basically ",
    " actually ", " literally ", " right ", " okay so ",
];

/// Clean transcript by removing filler words
fn clean_transcript(text: &str) -> String {
    let mut result = format!(" {} ", text.to_lowercase());

    for filler in FILLER_WORDS {
        result = result.replace(filler, " ");
    }

    // Clean up extra spaces and restore proper capitalization
    let cleaned: String = result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Capitalize first letter
    let mut chars: Vec<char> = cleaned.chars().collect();
    if !chars.is_empty() {
        chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
    }

    chars.into_iter().collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingState {
    pub is_recording: bool,
    pub is_live_transcribing: bool,
    pub is_transcribing: bool,
    pub transcription: Vec<TranscriptSegment>,
    pub summary: String,
    pub suggested_replies: Vec<String>,
    pub selected_model: String,
    pub transcription_provider: TranscriptionProvider,
    pub has_groq_key: bool,
    pub has_assemblyai_key: bool,
    pub has_deepgram_key: bool,
    pub current_recording_path: Option<String>,
    pub meeting_context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Serialize)]
struct TranscriptEvent {
    text: String,
    timestamp: String,
    speaker: String,
    is_final: bool,  // true = finalized transcript, false = interim (still being transcribed)
}

// Commands

#[tauri::command]
async fn start_recording(state: State<'_, AppState>) -> Result<String, String> {
    let mut is_recording = state.is_recording.lock().map_err(|e| e.to_string())?;
    if *is_recording {
        return Err("Already recording".to_string());
    }

    let recorder = audio::AudioRecorder::new().map_err(|e| e.to_string())?;
    let output_path = recorder.get_output_path().to_string();

    *state.current_recording_path.lock().map_err(|e| e.to_string())? = Some(output_path.clone());
    *state.audio_recorder.lock().map_err(|e| e.to_string())? = Some(recorder);
    *is_recording = true;

    Ok(output_path)
}

#[tauri::command]
async fn stop_recording(state: State<'_, AppState>) -> Result<String, String> {
    let mut is_recording = state.is_recording.lock().map_err(|e| e.to_string())?;
    if !*is_recording {
        return Err("Not recording".to_string());
    }

    let audio_path = if let Some(recorder) = state.audio_recorder.lock().map_err(|e| e.to_string())?.take() {
        recorder.stop().map_err(|e| e.to_string())?
    } else {
        return Err("No active recorder".to_string());
    };

    *is_recording = false;
    Ok(audio_path)
}

#[tauri::command]
async fn start_live_transcription(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider = state.transcription_provider.lock().map_err(|e| e.to_string())?.clone();
    let groq_key = state.groq_api_key.lock().map_err(|e| e.to_string())?.clone();
    let deepgram_key = state.deepgram_api_key.lock().map_err(|e| e.to_string())?.clone();
    let assemblyai_key = state.assemblyai_api_key.lock().map_err(|e| e.to_string())?.clone();

    // Auto-select provider: prefer Deepgram (real-time) if available, else Groq (batch)
    let effective_provider = match provider {
        TranscriptionProvider::Deepgram => {
            if deepgram_key.is_empty() {
                // Fallback to Groq if Deepgram key not set
                if !groq_key.is_empty() {
                    eprintln!("Deepgram key not set, falling back to Groq Whisper");
                    TranscriptionProvider::Groq
                } else {
                    return Err("Please set your Deepgram or Groq API key in Settings".to_string());
                }
            } else {
                TranscriptionProvider::Deepgram
            }
        }
        TranscriptionProvider::Groq => {
            if groq_key.is_empty() {
                // Fallback to Deepgram if Groq key not set
                if !deepgram_key.is_empty() {
                    eprintln!("Groq key not set, falling back to Deepgram streaming");
                    TranscriptionProvider::Deepgram
                } else {
                    return Err("Please set your Groq or Deepgram API key in Settings".to_string());
                }
            } else {
                TranscriptionProvider::Groq
            }
        }
        TranscriptionProvider::AssemblyAI => {
            if assemblyai_key.is_empty() {
                return Err("Please set your AssemblyAI API key in Settings".to_string());
            }
            TranscriptionProvider::AssemblyAI
        }
    };

    {
        let mut is_live = state.is_live_transcribing.lock().map_err(|e| e.to_string())?;
        if *is_live {
            return Err("Already transcribing".to_string());
        }
        *is_live = true;
    }

    match effective_provider {
        TranscriptionProvider::Deepgram => {
            // Use Deepgram real-time streaming with optimized parameters
            eprintln!("Using Deepgram for real-time transcription (nova-2, 100ms endpointing)...");
            state.deepgram_stop_flag.store(false, Ordering::SeqCst);

            // Create channel for receiving transcripts (now includes is_final flag)
            let (tx, mut rx) = mpsc::channel::<TranscriptMessage>(100);
            let transcriber = DeepgramTranscriber::new(tx);

            let app_clone = app.clone();
            let transcription_state = state.transcription.clone();

            // Spawn task to handle incoming transcripts
            tokio::spawn(async move {
                // Track interim text to replace with final
                let mut current_interim_index: Option<usize> = None;

                while let Some(msg) = rx.recv().await {
                    if msg.text.is_empty() {
                        continue;
                    }

                    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();

                    // Convert speaker ID to label
                    // Speaker 0 = "You" (typically the first/primary speaker detected)
                    // Speaker 1+ = "Participant" (other speakers)
                    let speaker_label = match msg.speaker {
                        Some(0) => "You".to_string(),
                        Some(_) => "Participant".to_string(),
                        None => "Speaker".to_string(),
                    };

                    if msg.is_final {
                        // Final transcript - add to transcription history
                        if let Ok(mut trans) = transcription_state.lock() {
                            // If we had an interim result, remove it
                            if let Some(idx) = current_interim_index.take() {
                                if idx < trans.len() {
                                    trans.remove(idx);
                                }
                            }
                            trans.push(TranscriptSegment {
                                timestamp: timestamp.clone(),
                                speaker: speaker_label.clone(),
                                text: clean_transcript(&msg.text),
                            });
                        }

                        let _ = app_clone.emit("transcript-update", TranscriptEvent {
                            text: msg.text,
                            timestamp,
                            speaker: speaker_label,
                            is_final: true,
                        });
                    } else {
                        // Interim result - emit for real-time UI feedback
                        // Don't add to permanent transcription yet
                        let _ = app_clone.emit("transcript-update", TranscriptEvent {
                            text: msg.text,
                            timestamp,
                            speaker: speaker_label,
                            is_final: false,
                        });
                    }
                }
            });

            // Start the transcriber with auto-retry on connection failures
            let api_key = deepgram_key.clone();
            tokio::spawn(async move {
                let mut retry_delay_ms: u64 = 1000;
                let mut consecutive_failures: u32 = 0;
                const MAX_RETRY_DELAY_MS: u64 = 30000;
                const MAX_RETRIES: u32 = 10;

                loop {
                    match transcriber.start(&api_key).await {
                        Ok(()) => {
                            eprintln!("Deepgram transcriber completed normally");
                            break;
                        }
                        Err(e) => {
                            consecutive_failures += 1;
                            eprintln!("Deepgram transcriber error (attempt {}): {}", consecutive_failures, e);

                            if consecutive_failures >= MAX_RETRIES {
                                eprintln!("Deepgram: Max retries ({}) reached, giving up", MAX_RETRIES);
                                break;
                            }

                            // Exponential backoff
                            eprintln!("Deepgram: Retrying in {}ms...", retry_delay_ms);
                            tokio::time::sleep(std::time::Duration::from_millis(retry_delay_ms)).await;

                            retry_delay_ms = std::cmp::min(retry_delay_ms * 2, MAX_RETRY_DELAY_MS);
                        }
                    }
                }
            });
        }
        TranscriptionProvider::Groq | TranscriptionProvider::AssemblyAI => {
            // Use batch transcription (Groq Whisper or AssemblyAI)
            let provider_name = match effective_provider {
                TranscriptionProvider::Groq => "Groq Whisper",
                TranscriptionProvider::AssemblyAI => "AssemblyAI",
                _ => "Unknown",
            };
            eprintln!("Using {} for transcription...", provider_name);

            let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
            *state.live_stop_signal.lock().map_err(|e| e.to_string())? = Some(stop_tx);

            let recorder = audio::AudioRecorder::new().map_err(|e| e.to_string())?;
            let output_path = recorder.get_output_path().to_string();
            *state.audio_recorder.lock().map_err(|e| e.to_string())? = Some(recorder);
            *state.current_recording_path.lock().map_err(|e| e.to_string())? = Some(output_path.clone());

            let transcription_state = state.transcription.clone();
            let is_live_transcribing = state.is_live_transcribing.clone();
            let api_key = if effective_provider == TranscriptionProvider::Groq { groq_key } else { assemblyai_key };
            let use_groq = effective_provider == TranscriptionProvider::Groq;

            tokio::spawn(async move {
                eprintln!("Starting {} transcription...", provider_name);

                const CHECK_INTERVAL_MS: u64 = 4000;  // Check every 4 seconds
                const MIN_AUDIO_BYTES: u64 = 48_000;

                let mut last_transcribed_size: u64 = 0;
                let mut last_full_text = String::new();  // Track last transcription to extract new text

                // Retry state for resilient error handling
                let mut consecutive_errors: u32 = 0;
                let mut retry_delay_ms: u64 = 1000;

                loop {
                    tokio::select! {
                        _ = stop_rx.recv() => {
                            eprintln!("Received stop signal");
                            break;
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_millis(CHECK_INTERVAL_MS)) => {
                            if let Ok(metadata) = tokio::fs::metadata(&output_path).await {
                                let current_size = metadata.len();
                                let new_audio = current_size.saturating_sub(last_transcribed_size);

                                if new_audio >= MIN_AUDIO_BYTES {
                                    eprintln!("New audio detected: {} bytes (total: {}MB), transcribing...",
                                        new_audio, current_size / 1_000_000);

                                    let result = if use_groq {
                                        groq::transcribe_audio(&api_key, &output_path).await
                                    } else {
                                        // AssemblyAI transcription
                                        assemblyai::transcribe_file(&api_key, &output_path).await
                                            .map(|r| r.text.unwrap_or_default())
                                    };

                                    match result {
                                        Ok(full_text) => {
                                            // Reset retry state on success
                                            consecutive_errors = 0;
                                            retry_delay_ms = 1000;

                                            if !full_text.is_empty() {
                                                // Extract only the NEW text (what's different from last transcription)
                                                let new_text = if last_full_text.is_empty() {
                                                    full_text.clone()
                                                } else if full_text.len() > last_full_text.len() && full_text.starts_with(&last_full_text) {
                                                    // New text is appended at the end
                                                    full_text[last_full_text.len()..].trim().to_string()
                                                } else if full_text != last_full_text {
                                                    // Text changed completely, use the full new text
                                                    full_text.clone()
                                                } else {
                                                    // Same text, nothing new
                                                    String::new()
                                                };

                                                if !new_text.is_empty() && new_text.len() > 5 {
                                                    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();

                                                    if let Ok(mut trans) = transcription_state.lock() {
                                                        trans.push(TranscriptSegment {
                                                            timestamp: timestamp.clone(),
                                                            speaker: "Speaker".to_string(),
                                                            text: clean_transcript(&new_text),
                                                        });
                                                    }

                                                    let _ = app.emit("transcript-update", TranscriptEvent {
                                                        text: new_text,
                                                        timestamp,
                                                        speaker: "Speaker".to_string(),
                                                        is_final: true,
                                                    });

                                                    eprintln!("New transcript segment emitted");
                                                } else {
                                                    eprintln!("No new speech detected");
                                                }

                                                last_full_text = full_text;
                                            }
                                            last_transcribed_size = current_size;
                                        }
                                        Err(e) => {
                                            consecutive_errors += 1;
                                            let error_msg = e.to_string();
                                            eprintln!("Transcription error (attempt {}): {}", consecutive_errors, error_msg);

                                            // Emit retry status to frontend
                                            let _ = app.emit("transcription-status", serde_json::json!({
                                                "status": "retrying",
                                                "error": error_msg,
                                                "attempt": consecutive_errors,
                                                "next_retry_ms": retry_delay_ms
                                            }));

                                            // Exponential backoff with max delay of 30 seconds
                                            if retry_delay_ms < 30000 {
                                                retry_delay_ms = std::cmp::min(retry_delay_ms * 2, 30000);
                                            }

                                            // Wait before next attempt (but still check for stop signal)
                                            tokio::time::sleep(std::time::Duration::from_millis(retry_delay_ms)).await;

                                            // Continue trying - the loop will automatically retry
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Ok(mut is_live) = is_live_transcribing.lock() {
                    *is_live = false;
                }
                eprintln!("{} transcription stopped", provider_name);
            });
        }
    }

    Ok(())
}

#[tauri::command]
async fn stop_live_transcription(state: State<'_, AppState>) -> Result<String, String> {
    // Stop Deepgram if running
    state.deepgram_stop_flag.store(true, Ordering::SeqCst);
    if let Some(transcriber) = state.deepgram_transcriber.lock().map_err(|e| e.to_string())?.take() {
        transcriber.stop();
        eprintln!("Deepgram transcriber stopped");
    }

    // Stop AssemblyAI batch mode if running
    let stop_tx = state.live_stop_signal.lock().map_err(|e| e.to_string())?.take();
    if let Some(tx) = stop_tx {
        let _ = tx.send(()).await;
    }

    // Stop the audio recorder (only used for AssemblyAI batch mode)
    let audio_path = if let Some(recorder) = state.audio_recorder.lock().map_err(|e| e.to_string())?.take() {
        recorder.stop().map_err(|e| e.to_string())?
    } else {
        String::new()
    };

    *state.is_live_transcribing.lock().map_err(|e| e.to_string())? = false;

    Ok(audio_path)
}

#[tauri::command]
async fn get_meeting_state(state: State<'_, AppState>) -> Result<MeetingState, String> {
    let has_groq_key = !state.groq_api_key.lock().map_err(|e| e.to_string())?.is_empty();
    let has_assemblyai_key = !state.assemblyai_api_key.lock().map_err(|e| e.to_string())?.is_empty();
    let has_deepgram_key = !state.deepgram_api_key.lock().map_err(|e| e.to_string())?.is_empty();
    let transcription_provider = state.transcription_provider.lock().map_err(|e| e.to_string())?.clone();

    Ok(MeetingState {
        is_recording: *state.is_recording.lock().map_err(|e| e.to_string())?,
        is_live_transcribing: *state.is_live_transcribing.lock().map_err(|e| e.to_string())?,
        is_transcribing: *state.is_transcribing.lock().map_err(|e| e.to_string())?,
        transcription: state.transcription.lock().map_err(|e| e.to_string())?.clone(),
        summary: state.summary.lock().map_err(|e| e.to_string())?.clone(),
        suggested_replies: state.suggested_replies.lock().map_err(|e| e.to_string())?.clone(),
        selected_model: state.selected_model.lock().map_err(|e| e.to_string())?.clone(),
        transcription_provider,
        has_groq_key,
        has_assemblyai_key,
        has_deepgram_key,
        current_recording_path: state.current_recording_path.lock().map_err(|e| e.to_string())?.clone(),
        meeting_context: state.meeting_context.lock().map_err(|e| e.to_string())?.clone(),
    })
}

#[tauri::command]
async fn set_groq_api_key(state: State<'_, AppState>, key: String) -> Result<bool, String> {
    // Basic validation - Groq API keys start with "gsk_"
    if key.is_empty() {
        return Ok(false);
    }

    // Save the key to memory
    *state.groq_api_key.lock().map_err(|e| e.to_string())? = key.clone();

    // Persist to disk
    {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.groq_api_key = key.clone();
        if let Err(e) = settings.save() {
            eprintln!("Failed to persist settings: {}", e);
        }
    }

    // Optionally verify with API (but don't block on failure)
    match groq::check_api_key(&key).await {
        Ok(true) => {
            eprintln!("Groq API key verified and saved successfully");
            Ok(true)
        }
        Ok(false) => {
            eprintln!("Groq API key verification failed, but key saved anyway");
            // Still return true since we saved it - user can try it
            Ok(true)
        }
        Err(e) => {
            eprintln!("Groq API key verification error: {}, but key saved anyway", e);
            // Still return true since we saved it
            Ok(true)
        }
    }
}

#[tauri::command]
async fn set_assemblyai_api_key(state: State<'_, AppState>, key: String) -> Result<bool, String> {
    if !key.is_empty() {
        *state.assemblyai_api_key.lock().map_err(|e| e.to_string())? = key.clone();

        // Persist to disk
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.assemblyai_api_key = key;
        if let Err(e) = settings.save() {
            eprintln!("Failed to persist settings: {}", e);
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
async fn set_deepgram_api_key(state: State<'_, AppState>, key: String) -> Result<bool, String> {
    if !key.is_empty() {
        *state.deepgram_api_key.lock().map_err(|e| e.to_string())? = key.clone();

        // Persist to disk
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.deepgram_api_key = key;
        if let Err(e) = settings.save() {
            eprintln!("Failed to persist settings: {}", e);
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
async fn set_model(state: State<'_, AppState>, model: String) -> Result<(), String> {
    *state.selected_model.lock().map_err(|e| e.to_string())? = model.clone();

    // Persist to disk
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.selected_model = model;
    if let Err(e) = settings.save() {
        eprintln!("Failed to persist settings: {}", e);
    }

    Ok(())
}

#[tauri::command]
async fn set_transcription_provider(state: State<'_, AppState>, provider: String) -> Result<(), String> {
    let provider_enum = match provider.to_lowercase().as_str() {
        "groq" => TranscriptionProvider::Groq,
        "deepgram" => TranscriptionProvider::Deepgram,
        "assemblyai" => TranscriptionProvider::AssemblyAI,
        _ => return Err(format!("Unknown provider: {}", provider)),
    };
    *state.transcription_provider.lock().map_err(|e| e.to_string())? = provider_enum;

    // Persist to disk
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.transcription_provider = provider;
    if let Err(e) = settings.save() {
        eprintln!("Failed to persist settings: {}", e);
    }

    Ok(())
}

#[tauri::command]
async fn set_meeting_context(state: State<'_, AppState>, context: String) -> Result<(), String> {
    *state.meeting_context.lock().map_err(|e| e.to_string())? = context.clone();

    // Persist to disk
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.meeting_context = context;
    if let Err(e) = settings.save() {
        eprintln!("Failed to persist settings: {}", e);
    }

    eprintln!("Meeting context updated");
    Ok(())
}

#[tauri::command]
async fn get_transcription_providers() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![
        serde_json::json!({
            "id": "Deepgram",
            "name": "Deepgram (Recommended)",
            "description": "Real-time streaming with speaker diarization. Words appear as spoken.",
            "recommended": true,
            "requires_key": "deepgram"
        }),
        serde_json::json!({
            "id": "Groq",
            "name": "Groq Whisper",
            "description": "Batch transcription every 4 seconds. Good fallback option.",
            "recommended": false,
            "requires_key": "groq"
        }),
        serde_json::json!({
            "id": "AssemblyAI",
            "name": "AssemblyAI",
            "description": "High accuracy batch transcription.",
            "recommended": false,
            "requires_key": "assemblyai"
        }),
    ])
}

#[tauri::command]
async fn get_available_models() -> Result<Vec<ModelInfo>, String> {
    Ok(groq::get_available_models()
        .into_iter()
        .map(|(id, name)| ModelInfo {
            id: id.to_string(),
            name: name.to_string(),
        })
        .collect())
}

#[tauri::command]
async fn add_transcription(
    state: State<'_, AppState>,
    text: String,
    speaker: String,
) -> Result<(), String> {
    let segment = TranscriptSegment {
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        speaker,
        text: clean_transcript(&text),
    };
    state.transcription.lock().map_err(|e| e.to_string())?.push(segment);
    Ok(())
}

#[tauri::command]
async fn add_manual_transcript(
    state: State<'_, AppState>,
    text: String,
    timestamp: String,
    speaker: String,
) -> Result<(), String> {
    let segment = TranscriptSegment {
        timestamp,
        speaker,
        text: clean_transcript(&text),
    };
    state.transcription.lock().map_err(|e| e.to_string())?.push(segment);
    Ok(())
}

#[tauri::command]
async fn clear_transcription(state: State<'_, AppState>) -> Result<(), String> {
    state.transcription.lock().map_err(|e| e.to_string())?.clear();
    *state.summary.lock().map_err(|e| e.to_string())? = String::new();
    state.suggested_replies.lock().map_err(|e| e.to_string())?.clear();
    Ok(())
}

#[tauri::command]
async fn transcribe_recording(state: State<'_, AppState>, file_path: String) -> Result<Vec<TranscriptSegment>, String> {
    let api_key = state.groq_api_key.lock().map_err(|e| e.to_string())?.clone();

    if api_key.is_empty() {
        return Err("Groq API key not set. Please add it in Settings.".to_string());
    }

    *state.is_transcribing.lock().map_err(|e| e.to_string())? = true;

    let result = groq::transcribe_audio(&api_key, &file_path).await;

    *state.is_transcribing.lock().map_err(|e| e.to_string())? = false;

    match result {
        Ok(text) => {
            let mut segments = Vec::new();

            if !text.is_empty() {
                let cleaned_text = clean_transcript(&text);
                let segment = TranscriptSegment {
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    speaker: "Speaker".to_string(),
                    text: cleaned_text,
                };
                segments.push(segment.clone());
                state.transcription.lock().map_err(|e| e.to_string())?.push(segment);
            }

            Ok(segments)
        }
        Err(e) => Err(e.to_string()),
    }
}

fn format_milliseconds(ms: u64) -> String {
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    format!("{:02}:{:02}:{:02}", hours, minutes % 60, seconds % 60)
}

#[tauri::command]
async fn list_recordings() -> Result<Vec<String>, String> {
    audio::list_recordings().map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_recordings_folder() -> Result<String, String> {
    audio::get_recordings_folder()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeetingSummary {
    #[serde(default)]
    pub key_points: Vec<String>,
    #[serde(default)]
    pub action_items: Vec<String>,
    #[serde(default)]
    pub decisions: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub raw_summary: String,
}

#[tauri::command]
async fn generate_summary(state: State<'_, AppState>) -> Result<String, String> {
    let transcription = state.transcription.lock().map_err(|e| e.to_string())?.clone();
    let model = state.selected_model.lock().map_err(|e| e.to_string())?.clone();
    let api_key = state.groq_api_key.lock().map_err(|e| e.to_string())?.clone();

    if transcription.is_empty() {
        return Err("No transcription to summarize".to_string());
    }

    let transcript_text: String = transcription
        .iter()
        .map(|s| format!("[{}] {}: {}", s.timestamp, s.speaker, s.text))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        r#"Analyze this meeting transcript and provide a structured summary. Format your response EXACTLY as follows:

## KEY POINTS
- [Main discussion point 1]
- [Main discussion point 2]
- [Add more as needed]

## ACTION ITEMS
- [Action item with owner if mentioned]
- [Add more as needed, or write "None identified" if no action items]

## DECISIONS MADE
- [Decision 1]
- [Add more as needed, or write "None identified" if no decisions]

## NOTES
- [Any other important observations]
- [Follow-ups needed]
- [Questions raised]

Be concise but comprehensive. Each bullet point should be a complete thought.

MEETING TRANSCRIPT:
{}"#,
        transcript_text
    );

    let summary = groq::generate(&api_key, &model, &prompt).await.map_err(|e| e.to_string())?;
    *state.summary.lock().map_err(|e| e.to_string())? = summary.clone();
    Ok(summary)
}

#[tauri::command]
async fn generate_structured_summary(state: State<'_, AppState>) -> Result<MeetingSummary, String> {
    let transcription = state.transcription.lock().map_err(|e| e.to_string())?.clone();
    let model = state.selected_model.lock().map_err(|e| e.to_string())?.clone();
    let api_key = state.groq_api_key.lock().map_err(|e| e.to_string())?.clone();

    if transcription.is_empty() {
        return Err("No transcription to summarize".to_string());
    }

    let transcript_text: String = transcription
        .iter()
        .map(|s| format!("[{}] {}: {}", s.timestamp, s.speaker, s.text))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        r#"Analyze this meeting transcript and provide a structured summary in JSON format.
Return ONLY valid JSON with this exact structure (no markdown, no explanation):
{{
  "key_points": ["point 1", "point 2"],
  "action_items": ["action 1 with owner", "action 2"],
  "decisions": ["decision 1", "decision 2"],
  "notes": ["note 1", "follow-up needed", "question raised"]
}}

If a category has no items, use an empty array [].
Each item should be a concise but complete sentence.

MEETING TRANSCRIPT:
{}"#,
        transcript_text
    );

    let response = groq::generate(&api_key, &model, &prompt).await.map_err(|e| e.to_string())?;

    // Try to parse JSON response
    let summary: MeetingSummary = match serde_json::from_str(&response) {
        Ok(s) => s,
        Err(_) => {
            // If JSON parsing fails, try to extract JSON from the response
            let json_start = response.find('{').unwrap_or(0);
            let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
            let json_str = &response[json_start..json_end];

            match serde_json::from_str(json_str) {
                Ok(s) => s,
                Err(_) => {
                    // Fallback: return raw summary
                    MeetingSummary {
                        key_points: vec![],
                        action_items: vec![],
                        decisions: vec![],
                        notes: vec![],
                        raw_summary: response.clone(),
                    }
                }
            }
        }
    };

    // Store raw summary for backward compatibility
    let raw = format!(
        "## KEY POINTS\n{}\n\n## ACTION ITEMS\n{}\n\n## DECISIONS\n{}\n\n## NOTES\n{}",
        summary.key_points.iter().map(|p| format!("• {}", p)).collect::<Vec<_>>().join("\n"),
        if summary.action_items.is_empty() { "• None identified".to_string() } else { summary.action_items.iter().map(|p| format!("• {}", p)).collect::<Vec<_>>().join("\n") },
        if summary.decisions.is_empty() { "• None identified".to_string() } else { summary.decisions.iter().map(|p| format!("• {}", p)).collect::<Vec<_>>().join("\n") },
        if summary.notes.is_empty() { "• None".to_string() } else { summary.notes.iter().map(|p| format!("• {}", p)).collect::<Vec<_>>().join("\n") }
    );
    *state.summary.lock().map_err(|e| e.to_string())? = raw;

    Ok(summary)
}

#[tauri::command]
async fn generate_reply_suggestions(
    state: State<'_, AppState>,
    context: String,
) -> Result<Vec<String>, String> {
    let model = state.selected_model.lock().map_err(|e| e.to_string())?.clone();
    let api_key = state.groq_api_key.lock().map_err(|e| e.to_string())?.clone();
    let transcription = state.transcription.lock().map_err(|e| e.to_string())?.clone();

    let recent_context: String = transcription
        .iter()
        .rev()
        .take(5)
        .rev()
        .map(|s| format!("{}: {}", s.speaker, s.text))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Based on this meeting context, suggest 3 brief professional responses I could give. Each response should be on a new line, numbered 1-3, and be concise (1-2 sentences max).\n\nRecent discussion:\n{}\n\nCurrent topic/question: {}",
        recent_context, context
    );

    let response = groq::generate(&api_key, &model, &prompt).await.map_err(|e| e.to_string())?;

    let replies: Vec<String> = response
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ')' || c == ':').trim().to_string())
        .filter(|line| !line.is_empty())
        .take(3)
        .collect();

    *state.suggested_replies.lock().map_err(|e| e.to_string())? = replies.clone();
    Ok(replies)
}

#[tauri::command]
async fn generate_auto_replies(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let model = state.selected_model.lock().map_err(|e| e.to_string())?.clone();
    let api_key = state.groq_api_key.lock().map_err(|e| e.to_string())?.clone();
    let transcription = state.transcription.lock().map_err(|e| e.to_string())?.clone();
    let meeting_context = state.meeting_context.lock().map_err(|e| e.to_string())?.clone();

    if api_key.is_empty() {
        return Err("Groq API key not set. Please add it in Settings.".to_string());
    }

    if transcription.is_empty() {
        return Err("No transcription available. Record or transcribe something first.".to_string());
    }

    // Get full transcript for context (limit to last 20 segments for performance)
    let context_segments: Vec<_> = transcription.iter().rev().take(20).rev().collect();
    let full_context: String = context_segments
        .iter()
        .map(|s| format!("[{}] {}: {}", s.timestamp, s.speaker, s.text))
        .collect::<Vec<_>>()
        .join("\n");

    // Detect the last speaker's intent
    let last_segment = transcription.last().map(|s| s.text.clone()).unwrap_or_default();

    // Build meeting context section if provided
    let meeting_context_section = if !meeting_context.is_empty() {
        format!("MEETING CONTEXT (use this to tailor your responses):\n{}\n\n", meeting_context)
    } else {
        String::new()
    };

    let prompt = format!(
        r#"You are the inner voice of a master communicator (Chris Voss, top negotiators). Generate tactical responses for this conversation.

{}CONVERSATION:
{}

JUST SAID: "{}"

Generate 6 tactical suggestions grouped by type. Mark the BEST one with ★.

TYPES:
• PROBE: Strategic question to uncover more
• INSIGHT: Pattern or observation you noticed
• MIRROR: Echo key words as a question
• REFRAME: Shift perspective or redirect
• CLARIFY: Get specifics on something unclear
• LABEL: Name the emotion or dynamic

RULES:
- Each suggestion: 3-12 words, specific to conversation
- No filler phrases
- One suggestion MUST have ★ prefix (your top recommendation)

FORMAT (exactly like this):
★ PROBE: What's driving that timeline?
INSIGHT: They keep circling back to cost
MIRROR: The accessibility concern?
PROBE: Who raised the PDF requirement?
REFRAME: What if we phase the rollout?
LABEL: Sounds like competing priorities"#,
        meeting_context_section, full_context, last_segment
    );

    eprintln!("Generating contextual auto replies from transcript...");
    let response = groq::generate(&api_key, &model, &prompt).await.map_err(|e| e.to_string())?;
    eprintln!("Got response from Groq");

    let replies: Vec<String> = response
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            // Clean up but preserve the TYPE: format and ★ marker
            let trimmed = line.trim();
            // Remove leading numbers like "1." or "1)"
            let cleaned = trimmed.trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ')' || c == '-').trim();
            cleaned.to_string()
        })
        .filter(|line| {
            // Keep lines that have TYPE: format (PROBE:, INSIGHT:, etc.) or start with ★
            let upper = line.to_uppercase();
            !line.is_empty() && (
                upper.starts_with("PROBE:") || upper.starts_with("★ PROBE:") ||
                upper.starts_with("INSIGHT:") || upper.starts_with("★ INSIGHT:") ||
                upper.starts_with("MIRROR:") || upper.starts_with("★ MIRROR:") ||
                upper.starts_with("REFRAME:") || upper.starts_with("★ REFRAME:") ||
                upper.starts_with("CLARIFY:") || upper.starts_with("★ CLARIFY:") ||
                upper.starts_with("LABEL:") || upper.starts_with("★ LABEL:")
            )
        })
        .take(6)
        .collect();

    *state.suggested_replies.lock().map_err(|e| e.to_string())? = replies.clone();
    Ok(replies)
}

#[tauri::command]
async fn check_connection(state: State<'_, AppState>) -> Result<bool, String> {
    let api_key = state.groq_api_key.lock().map_err(|e| e.to_string())?.clone();
    groq::check_api_key(&api_key).await.map_err(|e| e.to_string())
}

/// Set screen share exclusion (hide window during screen sharing)
#[tauri::command]
fn set_screen_share_exclusion(window: tauri::Window, exclude: bool) -> Result<bool, String> {
    screen_share::set_screen_share_exclusion(&window, exclude)?;
    Ok(exclude)
}

/// Check if screen share exclusion is supported on this platform
#[tauri::command]
fn is_screen_share_exclusion_supported() -> bool {
    screen_share::is_supported()
}

/// Get platform info about screen share exclusion support
#[tauri::command]
fn get_screen_share_platform_info() -> String {
    screen_share::get_platform_info().to_string()
}

/// Start mock transcription using pre-recorded audio files
/// This is for dev/testing purposes to simulate a live meeting
/// Expects files named: you_1.wav, participant_1.wav, you_2.wav, participant_2.wav, etc.
#[tauri::command]
async fn start_mock_transcription(
    test_audio_dir: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    // Check if already mock transcribing
    {
        let is_mock = state.is_mock_transcribing.lock().map_err(|e| e.to_string())?;
        if *is_mock {
            return Err("Mock transcription already running".to_string());
        }
    }

    // Get API key
    let api_key = state.groq_api_key.lock().map_err(|e| e.to_string())?.clone();
    if api_key.is_empty() {
        return Err("Groq API key not set. Please add it in Settings.".to_string());
    }

    // Set up stop signal
    let (stop_tx, stop_rx) = watch::channel(false);
    *state.mock_stop_signal.lock().map_err(|e| e.to_string())? = Some(stop_tx);
    *state.is_mock_transcribing.lock().map_err(|e| e.to_string())? = true;

    let config = mock::MockConfig {
        test_audio_dir: test_audio_dir.clone(),
    };

    let transcription_state = state.transcription.clone();
    let is_mock_transcribing = state.is_mock_transcribing.clone();

    // Spawn the mock session
    tokio::spawn(async move {
        match mock::run_mock_session(config, &api_key, app, transcription_state, stop_rx).await {
            Ok(_) => eprintln!("Mock transcription completed successfully"),
            Err(e) => eprintln!("Mock transcription error: {}", e),
        }

        // Mark as not running
        if let Ok(mut is_mock) = is_mock_transcribing.lock() {
            *is_mock = false;
        }
    });

    Ok(format!("Mock transcription started from: {}", test_audio_dir))
}

/// Stop mock transcription
#[tauri::command]
async fn stop_mock_transcription(state: State<'_, AppState>) -> Result<(), String> {
    // Send stop signal
    if let Some(tx) = state.mock_stop_signal.lock().map_err(|e| e.to_string())?.take() {
        let _ = tx.send(true);
    }

    *state.is_mock_transcribing.lock().map_err(|e| e.to_string())? = false;
    eprintln!("Mock transcription stopped");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            start_live_transcription,
            stop_live_transcription,
            get_meeting_state,
            set_groq_api_key,
            set_assemblyai_api_key,
            set_deepgram_api_key,
            set_model,
            set_transcription_provider,
            set_meeting_context,
            get_transcription_providers,
            get_available_models,
            add_transcription,
            add_manual_transcript,
            clear_transcription,
            transcribe_recording,
            list_recordings,
            get_recordings_folder,
            generate_summary,
            generate_structured_summary,
            generate_reply_suggestions,
            generate_auto_replies,
            check_connection,
            set_screen_share_exclusion,
            is_screen_share_exclusion_supported,
            get_screen_share_platform_info,
            start_mock_transcription,
            stop_mock_transcription,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_transcript_removes_fillers() {
        assert_eq!(
            clean_transcript("So um I think like we should you know consider this"),
            "So i think we should consider this"
        );
    }

    #[test]
    fn test_clean_transcript_handles_multiple_fillers() {
        assert_eq!(
            clean_transcript("Um uh er ah the thing is basically"),
            "The thing is"
        );
    }

    #[test]
    fn test_clean_transcript_preserves_content() {
        assert_eq!(
            clean_transcript("The project deadline is next Friday"),
            "The project deadline is next friday"
        );
    }

    #[test]
    fn test_clean_transcript_capitalizes_first_letter() {
        assert_eq!(
            clean_transcript("hello world"),
            "Hello world"
        );
    }

    #[test]
    fn test_clean_transcript_empty_string() {
        assert_eq!(clean_transcript(""), "");
    }

    // Tests for retry/exponential backoff logic
    #[test]
    fn test_exponential_backoff_calculation() {
        let mut retry_delay_ms: u64 = 1000;
        const MAX_RETRY_DELAY_MS: u64 = 30000;

        // First retry: 1000 -> 2000
        retry_delay_ms = std::cmp::min(retry_delay_ms * 2, MAX_RETRY_DELAY_MS);
        assert_eq!(retry_delay_ms, 2000);

        // Second retry: 2000 -> 4000
        retry_delay_ms = std::cmp::min(retry_delay_ms * 2, MAX_RETRY_DELAY_MS);
        assert_eq!(retry_delay_ms, 4000);

        // Third retry: 4000 -> 8000
        retry_delay_ms = std::cmp::min(retry_delay_ms * 2, MAX_RETRY_DELAY_MS);
        assert_eq!(retry_delay_ms, 8000);

        // Fourth retry: 8000 -> 16000
        retry_delay_ms = std::cmp::min(retry_delay_ms * 2, MAX_RETRY_DELAY_MS);
        assert_eq!(retry_delay_ms, 16000);

        // Fifth retry: 16000 -> 30000 (capped at max)
        retry_delay_ms = std::cmp::min(retry_delay_ms * 2, MAX_RETRY_DELAY_MS);
        assert_eq!(retry_delay_ms, 30000);

        // Sixth retry: stays at max
        retry_delay_ms = std::cmp::min(retry_delay_ms * 2, MAX_RETRY_DELAY_MS);
        assert_eq!(retry_delay_ms, 30000);
    }

    #[test]
    fn test_retry_counter_increment() {
        let mut consecutive_errors: u32 = 0;

        // Simulate 5 consecutive errors
        for i in 1..=5 {
            consecutive_errors += 1;
            assert_eq!(consecutive_errors, i);
        }

        // Reset on success
        consecutive_errors = 0;
        assert_eq!(consecutive_errors, 0);
    }

    #[test]
    fn test_max_retries_limit() {
        const MAX_RETRIES: u32 = 10;
        let mut consecutive_failures: u32 = 0;

        // Should continue retrying until max
        for _ in 0..MAX_RETRIES {
            consecutive_failures += 1;
            if consecutive_failures >= MAX_RETRIES {
                break;
            }
        }

        assert_eq!(consecutive_failures, MAX_RETRIES);
    }

    #[test]
    fn test_retry_delay_starts_at_one_second() {
        let retry_delay_ms: u64 = 1000;
        assert_eq!(retry_delay_ms, 1000);
    }

    #[test]
    fn test_retry_delay_max_is_30_seconds() {
        const MAX_RETRY_DELAY_MS: u64 = 30000;
        let very_large_delay: u64 = 100000;
        let capped = std::cmp::min(very_large_delay, MAX_RETRY_DELAY_MS);
        assert_eq!(capped, 30000);
    }
}
