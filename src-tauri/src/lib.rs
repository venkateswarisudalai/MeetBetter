use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

mod assemblyai;
mod audio;
mod deepgram;
mod groq;
mod realtime;
mod screen_share;

use deepgram::DeepgramTranscriber;

/// Transcription provider options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TranscriptionProvider {
    Groq,       // Recommended - uses Whisper, good quality
    Deepgram,   // Real-time streaming, fast
    AssemblyAI, // High accuracy, batch processing
}

impl Default for TranscriptionProvider {
    fn default() -> Self {
        TranscriptionProvider::Groq
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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            is_recording: Arc::new(Mutex::new(false)),
            is_live_transcribing: Arc::new(Mutex::new(false)),
            transcription: Arc::new(Mutex::new(Vec::new())),
            summary: Arc::new(Mutex::new(String::new())),
            suggested_replies: Arc::new(Mutex::new(Vec::new())),
            selected_model: Arc::new(Mutex::new("llama-3.1-8b-instant".to_string())),
            transcription_provider: Arc::new(Mutex::new(TranscriptionProvider::default())),
            groq_api_key: Arc::new(Mutex::new(String::new())),
            assemblyai_api_key: Arc::new(Mutex::new(String::new())),
            deepgram_api_key: Arc::new(Mutex::new(String::new())),
            audio_recorder: Arc::new(Mutex::new(None)),
            current_recording_path: Arc::new(Mutex::new(None)),
            is_transcribing: Arc::new(Mutex::new(false)),
            live_stop_signal: Arc::new(Mutex::new(None)),
            deepgram_transcriber: Arc::new(Mutex::new(None)),
            deepgram_stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub timestamp: String,
    pub speaker: String,
    pub text: String,
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

    // Check if the required API key is set for the selected provider
    match provider {
        TranscriptionProvider::Groq => {
            if groq_key.is_empty() {
                return Err("Please set your Groq API key in Settings".to_string());
            }
        }
        TranscriptionProvider::Deepgram => {
            if deepgram_key.is_empty() {
                return Err("Please set your Deepgram API key in Settings".to_string());
            }
        }
        TranscriptionProvider::AssemblyAI => {
            if assemblyai_key.is_empty() {
                return Err("Please set your AssemblyAI API key in Settings".to_string());
            }
        }
    }

    {
        let mut is_live = state.is_live_transcribing.lock().map_err(|e| e.to_string())?;
        if *is_live {
            return Err("Already transcribing".to_string());
        }
        *is_live = true;
    }

    match provider {
        TranscriptionProvider::Deepgram => {
            // Use Deepgram real-time streaming
            eprintln!("Using Deepgram for real-time transcription...");
            state.deepgram_stop_flag.store(false, Ordering::SeqCst);

            // Create channel for receiving transcripts
            let (tx, mut rx) = mpsc::channel::<String>(100);
            let transcriber = DeepgramTranscriber::new(tx);

            let app_clone = app.clone();
            let transcription_state = state.transcription.clone();

            // Spawn task to handle incoming transcripts
            tokio::spawn(async move {
                while let Some(text) = rx.recv().await {
                    if !text.is_empty() {
                        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();

                        if let Ok(mut trans) = transcription_state.lock() {
                            trans.push(TranscriptSegment {
                                timestamp: timestamp.clone(),
                                speaker: "Speaker".to_string(),
                                text: text.clone(),
                            });
                        }

                        let _ = app_clone.emit("transcript-update", TranscriptEvent {
                            text,
                            timestamp,
                            speaker: "Speaker".to_string(),
                        });
                    }
                }
            });

            // Start the transcriber
            let api_key = deepgram_key.clone();
            tokio::spawn(async move {
                if let Err(e) = transcriber.start(&api_key).await {
                    eprintln!("Deepgram transcriber error: {}", e);
                }
            });
        }
        TranscriptionProvider::Groq | TranscriptionProvider::AssemblyAI => {
            // Use batch transcription (Groq Whisper or AssemblyAI)
            let provider_name = match provider {
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
            let api_key = if provider == TranscriptionProvider::Groq { groq_key } else { assemblyai_key };
            let use_groq = provider == TranscriptionProvider::Groq;

            tokio::spawn(async move {
                eprintln!("Starting {} transcription...", provider_name);

                const CHECK_INTERVAL_MS: u64 = 4000;  // Check every 4 seconds
                const MIN_AUDIO_BYTES: u64 = 48_000;

                let mut last_transcribed_size: u64 = 0;
                let mut last_full_text: String = String::new();

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
                                    eprintln!("New audio detected: {} bytes, transcribing...", new_audio);

                                    let result = if use_groq {
                                        groq::transcribe_audio(&api_key, &output_path).await
                                    } else {
                                        // AssemblyAI transcription
                                        assemblyai::transcribe_file(&api_key, &output_path).await
                                            .map(|r| r.text.unwrap_or_default())
                                    };

                                    match result {
                                        Ok(full_text) => {
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
                                                            text: new_text.clone(),
                                                        });
                                                    }

                                                    let _ = app.emit("transcript-update", TranscriptEvent {
                                                        text: new_text,
                                                        timestamp,
                                                        speaker: "Speaker".to_string(),
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
                                            eprintln!("Transcription error: {}", e);
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
    })
}

#[tauri::command]
async fn set_groq_api_key(state: State<'_, AppState>, key: String) -> Result<bool, String> {
    // Basic validation - Groq API keys start with "gsk_"
    if key.is_empty() {
        return Ok(false);
    }

    // Save the key first
    *state.groq_api_key.lock().map_err(|e| e.to_string())? = key.clone();

    // Optionally verify with API (but don't block on failure)
    match groq::check_api_key(&key).await {
        Ok(true) => {
            eprintln!("Groq API key verified successfully");
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
        *state.assemblyai_api_key.lock().map_err(|e| e.to_string())? = key;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
async fn set_deepgram_api_key(state: State<'_, AppState>, key: String) -> Result<bool, String> {
    if !key.is_empty() {
        *state.deepgram_api_key.lock().map_err(|e| e.to_string())? = key;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
async fn set_model(state: State<'_, AppState>, model: String) -> Result<(), String> {
    *state.selected_model.lock().map_err(|e| e.to_string())? = model;
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
    Ok(())
}

#[tauri::command]
async fn get_transcription_providers() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![
        serde_json::json!({
            "id": "Groq",
            "name": "Groq Whisper (Recommended)",
            "description": "Free, fast, and accurate. Uses Whisper model for transcription.",
            "recommended": true,
            "requires_key": "groq"
        }),
        serde_json::json!({
            "id": "Deepgram",
            "name": "Deepgram",
            "description": "Real-time streaming transcription. Very fast response time.",
            "recommended": false,
            "requires_key": "deepgram"
        }),
        serde_json::json!({
            "id": "AssemblyAI",
            "name": "AssemblyAI",
            "description": "High accuracy transcription with speaker detection.",
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
        text,
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
        text,
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
                let segment = TranscriptSegment {
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    speaker: "Speaker".to_string(),
                    text,
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

    // Get the most recent statements (last 5) to focus replies on current topic
    let recent_statements: String = transcription
        .iter()
        .rev()
        .take(5)
        .rev()
        .map(|s| format!("{}: {}", s.speaker, s.text))
        .collect::<Vec<_>>()
        .join("\n");

    // Detect the last speaker's intent
    let last_segment = transcription.last().map(|s| s.text.clone()).unwrap_or_default();

    let prompt = format!(
        r#"You are an AI assistant helping someone participate in a real-time meeting. Generate smart, contextual reply suggestions instantly.

CONVERSATION CONTEXT:
{}

MOST RECENT (what was just said):
{}

LAST STATEMENT: "{}"

Generate 4 quick reply suggestions the user can say RIGHT NOW. Requirements:
1. Be IMMEDIATELY relevant to what was just said
2. Reference specific details from the conversation
3. Sound natural and professional
4. Keep it SHORT (1 sentence preferred, 2 max)

Reply types (pick the most appropriate for the situation):
• Answer if a question was asked
• Ask a clarifying question about something specific
• Agree with reasoning OR respectfully disagree with alternative
• Suggest next step or action
• Offer help or take ownership of something
• Add relevant information or perspective

Format: Return exactly 4 replies, numbered 1-4, one per line.
DO NOT use generic filler like "Great point" - be specific and actionable."#,
        full_context, recent_statements, last_segment
    );

    eprintln!("Generating contextual auto replies from transcript...");
    let response = groq::generate(&api_key, &model, &prompt).await.map_err(|e| e.to_string())?;
    eprintln!("Got response from Groq");

    let replies: Vec<String> = response
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ')' || c == ':' || c == '-').trim().to_string())
        .filter(|line| !line.is_empty() && line.len() > 10)
        .take(4)
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
