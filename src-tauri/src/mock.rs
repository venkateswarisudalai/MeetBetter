use anyhow::Result;
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;

use crate::groq;
use crate::TranscriptSegment;

#[derive(Clone)]
struct TranscriptEvent {
    text: String,
    timestamp: String,
    speaker: String,
}

impl serde::Serialize for TranscriptEvent {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("TranscriptEvent", 3)?;
        state.serialize_field("text", &self.text)?;
        state.serialize_field("timestamp", &self.timestamp)?;
        state.serialize_field("speaker", &self.speaker)?;
        state.end()
    }
}

/// Configuration for mock transcription session
pub struct MockConfig {
    pub test_audio_dir: String,
}

/// Run a mock transcription session using pre-recorded audio files.
/// Looks for files named: you_1.wav, participant_1.wav, you_2.wav, participant_2.wav, etc.
/// This simulates a realistic back-and-forth meeting conversation.
pub async fn run_mock_session(
    config: MockConfig,
    api_key: &str,
    app: AppHandle,
    transcription_state: std::sync::Arc<std::sync::Mutex<Vec<TranscriptSegment>>>,
    stop_signal: watch::Receiver<bool>,
) -> Result<()> {
    eprintln!("Starting mock transcription session...");
    eprintln!("  Test audio dir: {}", config.test_audio_dir);

    let dir = std::path::Path::new(&config.test_audio_dir);
    if !dir.exists() {
        return Err(anyhow::anyhow!("Test audio directory not found: {}", config.test_audio_dir));
    }

    // Find all conversation turns (you_1.wav, participant_1.wav, you_2.wav, etc.)
    let mut turn = 1;
    loop {
        if *stop_signal.borrow() {
            eprintln!("Mock session stopped by signal");
            break;
        }

        let you_file = dir.join(format!("you_{}.wav", turn));
        let participant_file = dir.join(format!("participant_{}.wav", turn));

        // Check if we have more turns
        if !you_file.exists() && !participant_file.exists() {
            if turn == 1 {
                return Err(anyhow::anyhow!(
                    "No conversation files found. Expected you_1.wav and/or participant_1.wav in {}",
                    config.test_audio_dir
                ));
            }
            eprintln!("No more conversation turns found after turn {}", turn - 1);
            break;
        }

        // Transcribe "You" turn if exists
        if you_file.exists() {
            eprintln!("Transcribing turn {} (You): {:?}", turn, you_file);
            match groq::transcribe_audio(api_key, you_file.to_str().unwrap()).await {
                Ok(text) if !text.is_empty() => {
                    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();

                    if let Ok(mut trans) = transcription_state.lock() {
                        trans.push(TranscriptSegment {
                            timestamp: timestamp.clone(),
                            speaker: "You".to_string(),
                            text: text.clone(),
                        });
                    }

                    let _ = app.emit("transcript-update", TranscriptEvent {
                        text,
                        timestamp,
                        speaker: "You".to_string(),
                    });
                }
                Ok(_) => eprintln!("Empty transcription for {:?}", you_file),
                Err(e) => eprintln!("Failed to transcribe {:?}: {}", you_file, e),
            }

            // Delay between turns
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        }

        if *stop_signal.borrow() {
            break;
        }

        // Transcribe "Participant" turn if exists
        if participant_file.exists() {
            eprintln!("Transcribing turn {} (Participant): {:?}", turn, participant_file);
            match groq::transcribe_audio(api_key, participant_file.to_str().unwrap()).await {
                Ok(text) if !text.is_empty() => {
                    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();

                    if let Ok(mut trans) = transcription_state.lock() {
                        trans.push(TranscriptSegment {
                            timestamp: timestamp.clone(),
                            speaker: "Participant".to_string(),
                            text: text.clone(),
                        });
                    }

                    let _ = app.emit("transcript-update", TranscriptEvent {
                        text,
                        timestamp,
                        speaker: "Participant".to_string(),
                    });
                }
                Ok(_) => eprintln!("Empty transcription for {:?}", participant_file),
                Err(e) => eprintln!("Failed to transcribe {:?}: {}", participant_file, e),
            }

            // Delay between turns
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        }

        turn += 1;
    }

    eprintln!("Mock transcription session completed ({} turns)", turn - 1);
    Ok(())
}
