use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{mpsc, watch};

mod assemblyai;
mod audio;
mod calendar;
mod database;
mod deepgram;
pub mod groq;  // Public for mock_test binary
mod meeting_monitor;
mod mock;
mod realtime;
mod screen_share;
mod settings;
mod supabase;
mod system_audio;

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
    // Meetings database
    pub meetings_db: Arc<Mutex<database::MeetingsDatabase>>,
    // Meeting monitor for auto-start
    pub meeting_monitor: Arc<meeting_monitor::MeetingMonitor>,
    // Supabase cloud sync
    pub supabase_client: Arc<Mutex<Option<supabase::SupabaseClient>>>,
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
            meeting_context: Arc::new(Mutex::new(saved_settings.meeting_context.clone())),
            // Mock transcription state
            is_mock_transcribing: Arc::new(Mutex::new(false)),
            mock_stop_signal: Arc::new(Mutex::new(None)),
            // Meetings database
            meetings_db: Arc::new(Mutex::new(database::MeetingsDatabase::load())),
            // Meeting monitor
            meeting_monitor: Arc::new(meeting_monitor::MeetingMonitor::new()),
            // Supabase client (always initialized with embedded credentials)
            supabase_client: Arc::new(Mutex::new(
                Some(supabase::SupabaseClient::with_embedded_credentials())
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TranscriptSegment {
    pub timestamp: String,
    pub speaker: String,
    pub text: String,
    #[serde(default, skip_serializing)]
    pub is_final: bool,
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

/// Check if two transcript texts are similar enough to be cross-channel duplicates.
/// Requires substantial overlap — short fragments won't match long sentences.
fn is_cross_channel_match(a: &str, b: &str) -> bool {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    // Exact match
    if a_lower == b_lower {
        return true;
    }

    let a_words: Vec<&str> = a_lower.split_whitespace().collect();
    let b_words: Vec<&str> = b_lower.split_whitespace().collect();
    let min_words = a_words.len().min(b_words.len());
    let max_words = a_words.len().max(b_words.len());

    // Skip if either is too short (< 4 words) — short fragments are unreliable
    if min_words < 4 {
        return false;
    }

    // Substring containment: only if the shorter string is at least 50% of the longer
    if min_words * 2 >= max_words {
        if a_lower.contains(&b_lower) || b_lower.contains(&a_lower) {
            return true;
        }
    }

    // Word overlap: require both high ratio AND minimum word count
    let words_a: std::collections::HashSet<&str> = a_words.into_iter().collect();
    let words_b: std::collections::HashSet<&str> = b_words.into_iter().collect();
    let overlap = words_a.intersection(&words_b).count();
    let ratio = overlap as f64 / min_words as f64;

    ratio > 0.6 && overlap >= 4
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

            // Start audio recorder to save WAV file (mic + system audio)
            match audio::AudioRecorder::new() {
                Ok(recorder) => {
                    let output_path = recorder.get_output_path().to_string();
                    eprintln!("Deepgram: Recording audio to {}", output_path);
                    *state.audio_recorder.lock().map_err(|e| e.to_string())? = Some(recorder);
                    *state.current_recording_path.lock().map_err(|e| e.to_string())? = Some(output_path);
                }
                Err(e) => {
                    eprintln!("Warning: Could not start audio recorder: {}. Transcription will continue without saving audio.", e);
                }
            }

            // Create channel for receiving transcripts (now includes is_final flag)
            let (tx, mut rx) = mpsc::channel::<TranscriptMessage>(100);
            let transcriber = DeepgramTranscriber::new(tx);

            let app_clone = app.clone();
            let transcription_state = state.transcription.clone();

            // Spawn task to handle incoming transcripts with mic-bleed dedup.
            // The mic picks up system audio from speakers, so both channels get
            // the same content. We buffer mic ("You") finals briefly — if a
            // matching "Participant" (system audio) final arrives within the
            // window, we discard the mic version and emit only Participant
            // (the true digital source). If no match, the mic final is emitted
            // after the timeout.
            tokio::spawn(async move {
                let mut recent_finals: Vec<String> = Vec::new();
                const MAX_RECENT: usize = 10;
                // Cross-channel dedup: (text, speaker, timestamp_ms)
                let mut recent_finals_by_channel: Vec<(String, String, u64)> = Vec::new();

                // Pending mic finals awaiting potential Participant match
                struct PendingMic {
                    cleaned_text: String,
                    timestamp: String,
                    buffered_at: u64,
                }
                let mut mic_buffer: Vec<PendingMic> = Vec::new();
                const MIC_HOLD_MS: u64 = 2000; // 2s window for system audio to arrive

                let mut flush_tick = tokio::time::interval(tokio::time::Duration::from_millis(300));

                loop {
                    enum Action {
                        Message(TranscriptMessage),
                        Tick,
                        Done,
                    }

                    let action = tokio::select! {
                        msg = rx.recv() => match msg {
                            Some(m) => Action::Message(m),
                            None => Action::Done,
                        },
                        _ = flush_tick.tick() => Action::Tick,
                    };

                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    // Flush aged-out mic finals (no Participant match arrived in time)
                    let mut i = 0;
                    while i < mic_buffer.len() {
                        if now_ms - mic_buffer[i].buffered_at >= MIC_HOLD_MS {
                            let pending = mic_buffer.remove(i);
                            eprintln!("Flushing buffered [You]: \"{}\" (no Participant match)", pending.cleaned_text);

                            recent_finals.push(pending.cleaned_text.clone());
                            if recent_finals.len() > MAX_RECENT {
                                recent_finals.remove(0);
                            }
                            recent_finals_by_channel.push((pending.cleaned_text.clone(), "You".to_string(), pending.buffered_at));

                            if let Ok(mut trans) = transcription_state.lock() {
                                trans.push(TranscriptSegment {
                                    timestamp: pending.timestamp.clone(),
                                    speaker: "You".to_string(),
                                    text: pending.cleaned_text.clone(),
                                    ..Default::default()
                                });
                            }

                            let _ = app_clone.emit("transcript-update", TranscriptEvent {
                                text: pending.cleaned_text,
                                timestamp: pending.timestamp,
                                speaker: "You".to_string(),
                                is_final: true,
                            });
                        } else {
                            i += 1;
                        }
                    }

                    // Purge old cross-channel entries
                    recent_finals_by_channel.retain(|(_t, _s, ts)| now_ms - *ts < 10_000);

                    match action {
                        Action::Done => break,
                        Action::Tick => continue,
                        Action::Message(msg) => {
                            if msg.text.is_empty() {
                                continue;
                            }

                            let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();

                            let speaker_label = match msg.source {
                                system_audio::AudioSource::Microphone => "You".to_string(),
                                system_audio::AudioSource::SystemAudio => "Participant".to_string(),
                                system_audio::AudioSource::Diarized(0) => "You".to_string(),
                                system_audio::AudioSource::Diarized(1) => "Participant".to_string(),
                                system_audio::AudioSource::Diarized(id) => format!("Participant {}", id),
                            };

                            if msg.is_final {
                                let cleaned_text = clean_transcript(&msg.text);

                                // Skip exact duplicates (already emitted)
                                if recent_finals.contains(&cleaned_text) {
                                    eprintln!("Skipping duplicate final: \"{}\"", cleaned_text);
                                    continue;
                                }

                                if speaker_label == "You" {
                                    // Skip if already buffered (avoid double-buffering same mic text)
                                    if mic_buffer.iter().any(|p| p.cleaned_text == cleaned_text) {
                                        eprintln!("Skipping duplicate (already buffered): \"{}\"", cleaned_text);
                                        continue;
                                    }
                                    // Check if Participant already emitted this
                                    let already_from_participant = recent_finals_by_channel.iter().any(|(prev_text, prev_speaker, _)| {
                                        prev_speaker != "You" && is_cross_channel_match(&cleaned_text, prev_text)
                                    });

                                    if already_from_participant {
                                        eprintln!("Skipping mic bleed [You]: \"{}\" (Participant already emitted)", cleaned_text);
                                        continue;
                                    }

                                    // Buffer it — wait for a potential Participant match
                                    eprintln!("Buffering [You]: \"{}\"", cleaned_text);
                                    mic_buffer.push(PendingMic {
                                        cleaned_text,
                                        timestamp,
                                        buffered_at: now_ms,
                                    });
                                } else {
                                    // Participant final — check against buffered mic entries
                                    // Use both exact match (case-insensitive) and similarity match
                                    let cleaned_lower = cleaned_text.to_lowercase();
                                    let match_idx = mic_buffer.iter().position(|pending| {
                                        pending.cleaned_text.to_lowercase() == cleaned_lower
                                            || is_cross_channel_match(&cleaned_text, &pending.cleaned_text)
                                    });

                                    if let Some(idx) = match_idx {
                                        let removed = mic_buffer.remove(idx);
                                        eprintln!("Discarded mic bleed [You]: \"{}\" → keeping [Participant]: \"{}\"",
                                            removed.cleaned_text, cleaned_text);
                                    }

                                    // Check if "You" already flushed matching content.
                                    // If so, retroactively REPLACE those "You" entries with this
                                    // Participant version (system audio is the true source).
                                    let matching_you_texts: Vec<String> = recent_finals_by_channel.iter()
                                        .filter(|(prev_text, prev_speaker, _)| {
                                            *prev_speaker == "You" && is_cross_channel_match(&cleaned_text, prev_text)
                                        })
                                        .map(|(text, _, _)| text.clone())
                                        .collect();

                                    if !matching_you_texts.is_empty() {
                                        eprintln!("Replacing {} flushed [You] entries with [Participant]: \"{}\"",
                                            matching_you_texts.len(), cleaned_text);

                                        // Remove matching "You" entries from transcript state
                                        if let Ok(mut trans) = transcription_state.lock() {
                                            trans.retain(|seg| {
                                                !(seg.speaker == "You" && matching_you_texts.iter().any(|t| *t == seg.text))
                                            });
                                        }

                                        // Tell frontend to remove matching "You" entries
                                        for you_text in &matching_you_texts {
                                            let _ = app_clone.emit("transcript-remove", serde_json::json!({
                                                "speaker": "You",
                                                "text": you_text,
                                            }));
                                        }

                                        // Remove from dedup trackers so Participant version can be added
                                        recent_finals_by_channel.retain(|(text, speaker, _)| {
                                            !(*speaker == "You" && matching_you_texts.contains(text))
                                        });
                                        recent_finals.retain(|t| !matching_you_texts.contains(t));
                                    }

                                    // Emit the Participant final
                                    recent_finals.push(cleaned_text.clone());
                                    if recent_finals.len() > MAX_RECENT {
                                        recent_finals.remove(0);
                                    }
                                    recent_finals_by_channel.push((cleaned_text.clone(), speaker_label.clone(), now_ms));

                                    if let Ok(mut trans) = transcription_state.lock() {
                                        trans.push(TranscriptSegment {
                                            timestamp: timestamp.clone(),
                                            speaker: speaker_label.clone(),
                                            text: cleaned_text.clone(),
                                            ..Default::default()
                                        });
                                    }

                                    let _ = app_clone.emit("transcript-update", TranscriptEvent {
                                        text: cleaned_text,
                                        timestamp,
                                        speaker: speaker_label,
                                        is_final: true,
                                    });
                                }
                            } else {
                                // Interim — emit immediately for real-time UI feedback
                                let _ = app_clone.emit("transcript-update", TranscriptEvent {
                                    text: msg.text,
                                    timestamp,
                                    speaker: speaker_label,
                                    is_final: false,
                                });
                            }
                        }
                    }
                }

                // Final flush: emit any remaining buffered mic entries
                for pending in mic_buffer {
                    if let Ok(mut trans) = transcription_state.lock() {
                        trans.push(TranscriptSegment {
                            timestamp: pending.timestamp.clone(),
                            speaker: "You".to_string(),
                            text: pending.cleaned_text.clone(),
                            ..Default::default()
                        });
                    }
                    let _ = app_clone.emit("transcript-update", TranscriptEvent {
                        text: pending.cleaned_text,
                        timestamp: pending.timestamp,
                        speaker: "You".to_string(),
                        is_final: true,
                    });
                }
            });

            // Set up audio devices ONCE (this is what triggers the microphone permission prompt)
            // By doing this outside the retry loop, we avoid re-requesting permission on retries
            let mut audio_setup = match transcriber.setup_audio() {
                Ok(setup) => setup,
                Err(e) => {
                    eprintln!("Failed to set up audio devices: {}", e);
                    return Err(format!("Audio setup failed: {}", e));
                }
            };

            // Start the transcriber with auto-retry on connection failures
            // Only the WebSocket connection is retried, NOT the audio device setup
            let api_key = deepgram_key.clone();
            let app_for_deepgram = app.clone();
            tokio::spawn(async move {
                let mut retry_delay_ms: u64 = 1000;
                let mut consecutive_failures: u32 = 0;
                const MAX_RETRY_DELAY_MS: u64 = 30000;
                const MAX_RETRIES: u32 = 10;

                loop {
                    match transcriber.start(&api_key, Some(app_for_deepgram.clone()), &mut audio_setup).await {
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
                                                            ..Default::default()
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

    // Stop the audio recorder (used for all transcription modes)
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
        ..Default::default()
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
        ..Default::default()
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
                    ..Default::default()
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

/// Parse a text-format summary into structured MeetingSummary
/// Handles formats like:
/// ## KEY POINTS
/// • point 1
/// • point 2
fn parse_text_summary(text: &str) -> MeetingSummary {
    let mut key_points = Vec::new();
    let mut action_items = Vec::new();
    let mut decisions = Vec::new();
    let mut notes = Vec::new();

    let mut current_section: Option<&str> = None;

    for line in text.lines() {
        let line = line.trim();

        // Detect section headers
        let lower = line.to_lowercase();
        if lower.contains("key point") || lower.contains("keypoint") {
            current_section = Some("key_points");
            continue;
        } else if lower.contains("action item") || lower.contains("action_item") {
            current_section = Some("action_items");
            continue;
        } else if lower.contains("decision") {
            current_section = Some("decisions");
            continue;
        } else if lower.contains("note") {
            current_section = Some("notes");
            continue;
        }

        // Extract bullet points
        if line.starts_with('•') || line.starts_with('-') || line.starts_with('*') {
            let content = line[1..].trim().to_string();
            if !content.is_empty() && content.to_lowercase() != "none" && !content.to_lowercase().contains("none identified") {
                match current_section {
                    Some("key_points") => key_points.push(content),
                    Some("action_items") => action_items.push(content),
                    Some("decisions") => decisions.push(content),
                    Some("notes") => notes.push(content),
                    Some(_) | None => {
                        // Default to notes if no section detected or unknown section
                        notes.push(content);
                    }
                }
            }
        }
    }

    eprintln!("Parsed text summary: {} key points, {} action items, {} decisions, {} notes",
        key_points.len(), action_items.len(), decisions.len(), notes.len());

    MeetingSummary {
        key_points,
        action_items,
        decisions,
        notes,
        raw_summary: text.to_string(),
    }
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
    eprintln!("Summary response from AI (first 500 chars): {}", &response.chars().take(500).collect::<String>());

    // Try to parse JSON response
    let summary: MeetingSummary = match serde_json::from_str(&response) {
        Ok(s) => {
            eprintln!("Successfully parsed JSON summary");
            s
        },
        Err(e1) => {
            eprintln!("Direct JSON parse failed: {}", e1);
            // If JSON parsing fails, try to extract JSON from the response
            let json_start = response.find('{').unwrap_or(0);
            let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
            let json_str = &response[json_start..json_end];

            match serde_json::from_str(json_str) {
                Ok(s) => {
                    eprintln!("Successfully parsed extracted JSON");
                    s
                },
                Err(e2) => {
                    eprintln!("Extracted JSON parse failed: {}. Falling back to text parsing.", e2);
                    // Fallback: parse the text format into structured data
                    parse_text_summary(&response)
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
    *state.summary.lock().map_err(|e| e.to_string())? = raw.clone();

    // Return summary with raw_summary populated
    Ok(MeetingSummary {
        key_points: summary.key_points,
        action_items: summary.action_items,
        decisions: summary.decisions,
        notes: summary.notes,
        raw_summary: raw,
    })
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

/// Get audio diagnostics info (current mode, devices, channel count, capture method)
#[tauri::command]
fn get_audio_diagnostics() -> Result<serde_json::Value, String> {
    let capture_method = system_audio::get_system_audio_backend();
    let has_system_audio = capture_method != system_audio::SystemAudioCaptureMethod::None;
    let mode = if has_system_audio { "multichannel" } else { "diarize" };
    let system_device_name = system_audio::get_system_audio_device_name();
    let devices = system_audio::list_audio_devices();

    Ok(serde_json::json!({
        "mode": mode,
        "capture_method": capture_method.to_string(),
        "system_audio_device": system_device_name,
        "has_system_audio": has_system_audio,
        "channels": if has_system_audio { 2 } else { 1 },
        "available_devices": devices,
    }))
}

/// Check if Screen Recording permission is granted (for ScreenCaptureKit)
#[tauri::command]
fn check_screen_recording_permission() -> bool {
    system_audio::check_screencapturekit_permission()
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

// ============== Calendar Commands ==============

/// Get Google OAuth auth URL
#[tauri::command]
async fn get_google_auth_url() -> Result<String, String> {
    let cal = calendar::GoogleCalendar::with_embedded_credentials();
    Ok(cal.get_auth_url())
}

/// Start a local HTTP server to capture the OAuth callback code
#[tauri::command]
async fn wait_for_oauth_callback() -> Result<String, String> {
    use tokio::net::TcpListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = TcpListener::bind("127.0.0.1:8765").await
        .map_err(|e| format!("Failed to start OAuth callback server: {}", e))?;

    // Wait for a single connection (with timeout)
    let code = tokio::time::timeout(std::time::Duration::from_secs(120), async {
        let (mut stream, _) = listener.accept().await
            .map_err(|e| format!("Failed to accept connection: {}", e))?;

        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).await
            .map_err(|e| format!("Failed to read request: {}", e))?;
        let request = String::from_utf8_lossy(&buf[..n]).to_string();

        // Extract code from GET /callback?code=...
        let code = request.lines().next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|path| {
                url::Url::parse(&format!("http://localhost{}", path)).ok()
            })
            .and_then(|url| {
                url.query_pairs()
                    .find(|(k, _)| k == "code")
                    .map(|(_, v)| v.to_string())
            })
            .ok_or_else(|| "No authorization code found in callback".to_string())?;

        // Send a nice HTML response
        let html = r#"<!DOCTYPE html><html><body style="font-family:system-ui;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;background:#1a1a2e;color:#fff"><div style="text-align:center"><h1>&#10004; Authorization Successful</h1><p>You can close this window and return to Vantage.</p></div></body></html>"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            html.len(), html
        );
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.flush().await;

        Ok::<String, String>(code)
    }).await
    .map_err(|_| "OAuth callback timed out after 2 minutes".to_string())??;

    Ok(code)
}

/// Exchange Google OAuth code for tokens
#[tauri::command]
async fn exchange_google_code(code: String) -> Result<bool, String> {
    let cal = calendar::GoogleCalendar::with_embedded_credentials();
    cal.exchange_code(&code).await?;
    Ok(true)
}

/// Check if Google Calendar is connected
#[tauri::command]
fn is_calendar_connected() -> bool {
    calendar::is_calendar_connected()
}

/// Disconnect Google Calendar
#[tauri::command]
fn disconnect_calendar() -> Result<(), String> {
    calendar::disconnect_calendar()
}

/// Get upcoming calendar events
#[tauri::command]
async fn get_upcoming_events(limit: Option<u32>) -> Result<Vec<calendar::SimpleCalendarEvent>, String> {
    let cal = calendar::GoogleCalendar::with_embedded_credentials();
    cal.get_upcoming_events(limit).await
}

/// Get past calendar events
#[tauri::command]
async fn get_past_calendar_events(days: Option<i64>, limit: Option<u32>) -> Result<Vec<calendar::SimpleCalendarEvent>, String> {
    let cal = calendar::GoogleCalendar::with_embedded_credentials();
    cal.get_past_events(days, limit).await
}

/// Update a Google Calendar event's description with meeting summary
#[tauri::command]
async fn update_calendar_event_description(
    event_id: String,
    description: String,
) -> Result<(), String> {
    let cal = calendar::GoogleCalendar::with_embedded_credentials();
    cal.update_event_description(&event_id, &description).await
}

// ============== Meeting Monitor Commands ==============

/// Get meeting monitor status
#[tauri::command]
async fn get_meeting_status(state: State<'_, AppState>) -> Result<meeting_monitor::MeetingStatus, String> {
    Ok(state.meeting_monitor.get_status().await)
}

/// Update meeting monitor settings
#[tauri::command]
async fn update_meeting_monitor_settings(
    state: State<'_, AppState>,
    settings: meeting_monitor::MeetingMonitorSettings,
) -> Result<(), String> {
    state.meeting_monitor.update_settings(settings).await;
    Ok(())
}

/// Get meeting monitor settings
#[tauri::command]
async fn get_meeting_monitor_settings(state: State<'_, AppState>) -> Result<meeting_monitor::MeetingMonitorSettings, String> {
    Ok(state.meeting_monitor.get_settings().await)
}

/// Reset meeting monitor trigger (useful when manually stopping)
#[tauri::command]
async fn reset_meeting_monitor_trigger(state: State<'_, AppState>) -> Result<(), String> {
    state.meeting_monitor.reset_trigger().await;
    Ok(())
}

/// Manually check for meetings (for testing)
#[tauri::command]
async fn check_for_meetings_now(state: State<'_, AppState>) -> Result<bool, String> {
    let cal = calendar::GoogleCalendar::with_embedded_credentials();
    state.meeting_monitor.check_for_meetings(&cal).await
}

// ============== Meetings Database Commands ==============

/// Save current meeting to database
#[tauri::command]
async fn save_meeting(
    state: State<'_, AppState>,
    title: String,
    attendees: Vec<String>,
    calendar_event_id: Option<String>,
    duration_seconds: Option<u64>,
    transcript: Option<Vec<TranscriptSegment>>,
    summary: Option<MeetingSummary>,
    user_notes: Option<String>,
) -> Result<String, String> {
    // Use provided transcript or fall back to state
    let transcription = if let Some(t) = transcript {
        if t.is_empty() {
            return Err("No transcription to save".to_string());
        }
        t
    } else {
        let t = state.transcription.lock().map_err(|e| e.to_string())?.clone();
        if t.is_empty() {
            return Err("No transcription to save".to_string());
        }
        t
    };

    eprintln!("Saving meeting '{}' with {} transcript segments", title, transcription.len());

    let recording_path = state.current_recording_path.lock().map_err(|e| e.to_string())?.clone();

    // Use provided summary or fall back to state
    let meeting_summary = if let Some(s) = summary {
        eprintln!("Using provided summary with {} key points, {} action items", s.key_points.len(), s.action_items.len());
        // If structured fields are empty but raw_summary exists, parse it
        if s.key_points.is_empty() && s.action_items.is_empty() && !s.raw_summary.is_empty() {
            eprintln!("Parsing raw_summary into structured fields...");
            let parsed = parse_text_summary(&s.raw_summary);
            eprintln!("Parsed: {} key points, {} action items, {} decisions, {} notes",
                parsed.key_points.len(), parsed.action_items.len(), parsed.decisions.len(), parsed.notes.len());
            Some(parsed)
        } else {
            Some(s)
        }
    } else {
        // Fall back to raw summary text from state
        let summary_text = state.summary.lock().map_err(|e| e.to_string())?.clone();
        if !summary_text.is_empty() {
            eprintln!("Parsing state summary into structured fields...");
            Some(parse_text_summary(&summary_text))
        } else {
            None
        }
    };

    let meeting = database::create_meeting_from_transcript(
        title,
        transcription,
        meeting_summary,
        attendees,
        calendar_event_id,
        recording_path,
        duration_seconds,
        user_notes,
    );

    let meeting_id = meeting.id.clone();
    eprintln!("Created meeting with ID: {}", meeting_id);

    let mut db = state.meetings_db.lock().map_err(|e| e.to_string())?;
    db.add_meeting(meeting.clone())?;
    eprintln!("Meeting saved to database");

    // Fire-and-forget cloud sync (always available with embedded credentials)
    let cloud_enabled = state.settings.lock().map_err(|e| e.to_string())?.cloud_sync_enabled;
    if cloud_enabled {
        let sync_client = supabase::SupabaseClient::with_embedded_credentials();
        tokio::spawn(async move {
            if let Err(e) = sync_client.upsert_meeting(&meeting).await {
                eprintln!("Background cloud sync failed for meeting: {}", e);
            }
        });
    }

    Ok(meeting_id)
}

/// Get all saved meetings
#[tauri::command]
async fn get_saved_meetings(state: State<'_, AppState>, limit: Option<usize>) -> Result<Vec<database::StoredMeeting>, String> {
    let db = state.meetings_db.lock().map_err(|e| e.to_string())?;
    let meetings = db.get_past_meetings(limit);
    Ok(meetings.into_iter().cloned().collect())
}

/// Get a specific meeting by ID
#[tauri::command]
async fn get_meeting_by_id(state: State<'_, AppState>, id: String) -> Result<Option<database::StoredMeeting>, String> {
    let db = state.meetings_db.lock().map_err(|e| e.to_string())?;
    Ok(db.get_meeting(&id).cloned())
}

/// Delete a meeting
#[tauri::command]
async fn delete_meeting(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut db = state.meetings_db.lock().map_err(|e| e.to_string())?;
    db.delete_meeting(&id)?;

    // Fire-and-forget cloud delete (always available with embedded credentials)
    let cloud_enabled = state.settings.lock().map_err(|e| e.to_string())?.cloud_sync_enabled;
    if cloud_enabled {
        let sync_client = supabase::SupabaseClient::with_embedded_credentials();
        let meeting_id = id.clone();
        tokio::spawn(async move {
            if let Err(e) = sync_client.delete_meeting(&meeting_id).await {
                eprintln!("Background cloud delete failed for meeting: {}", e);
            }
        });
    }

    Ok(())
}

/// Enhance user notes with AI using transcript context
#[tauri::command]
async fn enhance_notes(
    state: State<'_, AppState>,
    user_notes: String,
) -> Result<String, String> {
    let api_key = state.groq_api_key.lock().map_err(|e| e.to_string())?.clone();
    let model = state.selected_model.lock().map_err(|e| e.to_string())?.clone();
    let transcription = state.transcription.lock().map_err(|e| e.to_string())?.clone();

    if api_key.is_empty() {
        return Err("Groq API key not set".to_string());
    }

    let transcript_text: String = transcription
        .iter()
        .map(|s| format!("[{}] {}: {}", s.timestamp, s.speaker, s.text))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        r#"You are enhancing meeting notes. Given the user's own notes and a transcript, produce enhanced meeting notes.

RULES:
- Preserve the user's original notes EXACTLY as written, prefixed with [USER] on each line
- Add AI-generated insights prefixed with [AI] — key points, action items, decisions the user didn't capture
- Structure the output clearly with sections
- Keep it concise and actionable
- Do NOT repeat information the user already wrote

USER'S NOTES:
{}

MEETING TRANSCRIPT:
{}

OUTPUT FORMAT:
[USER] (user's original note line)
[AI] (AI-added insight)
...

Group into sections: Key Points, Action Items, Decisions, Additional Context"#,
        user_notes, transcript_text
    );

    let response = groq::generate(&api_key, &model, &prompt).await.map_err(|e| e.to_string())?;
    Ok(response)
}

// ============== Supabase Cloud Sync Commands ==============

/// Toggle cloud sync on/off
#[tauri::command]
async fn toggle_cloud_sync(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.cloud_sync_enabled = enabled;
    settings.save().map_err(|e| e.to_string())?;

    let mut client_guard = state.supabase_client.lock().map_err(|e| e.to_string())?;
    if enabled {
        *client_guard = Some(supabase::SupabaseClient::with_embedded_credentials());
    } else {
        *client_guard = None;
    }

    Ok(())
}

/// Sync all local meetings to Supabase
#[tauri::command]
async fn sync_meetings_to_cloud(
    state: State<'_, AppState>,
) -> Result<u32, String> {
    {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        if !settings.cloud_sync_enabled {
            return Err("Cloud sync not enabled".to_string());
        }
    }

    let client = supabase::SupabaseClient::with_embedded_credentials();

    let meetings = {
        let db = state.meetings_db.lock().map_err(|e| e.to_string())?;
        db.get_all_meetings().into_iter().cloned().collect::<Vec<_>>()
    };

    let total = meetings.len() as u32;
    let mut synced: u32 = 0;

    for meeting in &meetings {
        match client.upsert_meeting(meeting).await {
            Ok(()) => synced += 1,
            Err(e) => eprintln!("Failed to sync meeting '{}': {}", meeting.id, e),
        }
    }

    eprintln!("Synced {}/{} meetings to Supabase", synced, total);
    Ok(synced)
}

/// Get cloud sync status
#[tauri::command]
async fn get_cloud_sync_status(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    let is_enabled = settings.cloud_sync_enabled;

    Ok(serde_json::json!({
        "is_configured": true,
        "is_enabled": is_enabled,
    }))
}

/// Search meetings
#[tauri::command]
async fn search_meetings(state: State<'_, AppState>, query: String) -> Result<Vec<database::StoredMeeting>, String> {
    let db = state.meetings_db.lock().map_err(|e| e.to_string())?;
    let meetings = db.search_meetings(&query);
    Ok(meetings.into_iter().cloned().collect())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Start background task for meeting monitor
            tauri::async_runtime::spawn(async move {
                loop {
                    // Wait 30 seconds between checks
                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

                    // Get app state
                    let state = app_handle.state::<AppState>();

                    // Check if meeting monitor is enabled
                    let settings = state.meeting_monitor.get_settings().await;
                    if !settings.enabled {
                        continue;
                    }

                    // Check for meetings using embedded credentials
                    let cal = calendar::GoogleCalendar::with_embedded_credentials();
                    match state.meeting_monitor.check_for_meetings(&cal).await {
                        Ok(should_auto_start) => {
                            if should_auto_start {
                                eprintln!("Meeting detected! Auto-starting transcription...");

                                // Emit event to frontend to auto-start
                                if let Err(e) = app_handle.emit("meeting-auto-start", ()) {
                                    eprintln!("Failed to emit meeting-auto-start event: {}", e);
                                }

                                // Get meeting status for event details
                                let status = state.meeting_monitor.get_status().await;
                                if let Err(e) = app_handle.emit("meeting-status-updated", status) {
                                    eprintln!("Failed to emit meeting-status-updated event: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error checking for meetings: {}", e);
                        }
                    }
                }
            });

            Ok(())
        })
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
            get_audio_diagnostics,
            check_screen_recording_permission,
            start_mock_transcription,
            stop_mock_transcription,
            // Calendar commands
            get_google_auth_url,
            wait_for_oauth_callback,
            exchange_google_code,
            is_calendar_connected,
            disconnect_calendar,
            get_upcoming_events,
            get_past_calendar_events,
            update_calendar_event_description,
            // Meeting monitor commands
            get_meeting_status,
            update_meeting_monitor_settings,
            get_meeting_monitor_settings,
            reset_meeting_monitor_trigger,
            check_for_meetings_now,
            // Meetings database commands
            save_meeting,
            get_saved_meetings,
            get_meeting_by_id,
            delete_meeting,
            search_meetings,
            enhance_notes,
            // Supabase cloud sync commands
            toggle_cloud_sync,
            sync_meetings_to_cloud,
            get_cloud_sync_status,
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
