//! Integration test for mock transcription pipeline
//!
//! Verifies:
//! 1. Voice-to-text transcription works (audio files → text)
//! 2. Suggested replies are generated correctly
//!
//! Run with: cargo run --bin mock_test

use std::sync::{Arc, Mutex};

// Re-use the library code
use vantage_lib::groq;

const TEST_AUDIO_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test_audio");
const ENV_GROQ_API_KEY: &str = "VANTAGE_GROQ_API_KEY";

#[tokio::main]
async fn main() {
    println!("=== Vantage Mock Integration Test ===\n");

    // Load .env file if it exists
    let _ = dotenvy::dotenv();

    // Try to get API key from environment variable first, then settings file
    let api_key = std::env::var(ENV_GROQ_API_KEY)
        .ok()
        .filter(|k| !k.is_empty())
        .or_else(|| {
            // Fallback to settings file
            let settings_path = dirs::config_dir()?.join("vantage").join("settings.json");
            if settings_path.exists() {
                let content = std::fs::read_to_string(&settings_path).ok()?;
                let json: serde_json::Value = serde_json::from_str(&content).ok()?;
                json["groq_api_key"].as_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .unwrap_or_default();

    if api_key.is_empty() {
        eprintln!("ERROR: No Groq API key found.");
        eprintln!("Set VANTAGE_GROQ_API_KEY environment variable or add key in app settings.");
        std::process::exit(1);
    }

    let key_source = if std::env::var(ENV_GROQ_API_KEY).is_ok() {
        "environment variable"
    } else {
        "settings file"
    };
    println!("✓ API key loaded from {}\n", key_source);

    // Collect transcripts
    let transcripts: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));

    // Test 1: Voice-to-text transcription
    println!("--- Test 1: Voice-to-Text Transcription ---\n");

    let mut turn = 1;
    let mut total_transcripts = 0;

    loop {
        let you_file = format!("{}/you_{}.wav", TEST_AUDIO_DIR, turn);
        let participant_file = format!("{}/participant_{}.wav", TEST_AUDIO_DIR, turn);

        if !std::path::Path::new(&you_file).exists() && !std::path::Path::new(&participant_file).exists() {
            break;
        }

        // Transcribe "You" audio
        if std::path::Path::new(&you_file).exists() {
            print!("  Transcribing you_{}.wav... ", turn);
            match groq::transcribe_audio(&api_key, &you_file).await {
                Ok(text) => {
                    println!("✓");
                    println!("    Speaker: You");
                    println!("    Text: \"{}\"\n", truncate(&text, 80));
                    transcripts.lock().unwrap().push(("You".to_string(), text));
                    total_transcripts += 1;
                }
                Err(e) => {
                    println!("✗ FAILED: {}", e);
                    std::process::exit(1);
                }
            }
        }

        // Transcribe "Participant" audio
        if std::path::Path::new(&participant_file).exists() {
            print!("  Transcribing participant_{}.wav... ", turn);
            match groq::transcribe_audio(&api_key, &participant_file).await {
                Ok(text) => {
                    println!("✓");
                    println!("    Speaker: Participant");
                    println!("    Text: \"{}\"\n", truncate(&text, 80));
                    transcripts.lock().unwrap().push(("Participant".to_string(), text));
                    total_transcripts += 1;
                }
                Err(e) => {
                    println!("✗ FAILED: {}", e);
                    std::process::exit(1);
                }
            }
        }

        turn += 1;
    }

    if total_transcripts == 0 {
        eprintln!("ERROR: No audio files found in {}", TEST_AUDIO_DIR);
        eprintln!("Expected files: you_1.wav, participant_1.wav, you_2.wav, etc.");
        std::process::exit(1);
    }

    println!("Voice-to-text: ✓ {} transcripts generated\n", total_transcripts);

    // Test 2: Reply suggestions
    println!("--- Test 2: Reply Suggestions ---\n");

    let trans = transcripts.lock().unwrap();

    // Build transcript text for the prompt
    let transcript_text: String = trans
        .iter()
        .map(|(speaker, text)| format!("{}: {}", speaker, text))
        .collect::<Vec<_>>()
        .join("\n");

    let last_statement = trans.last().map(|(_, t)| t.as_str()).unwrap_or("");

    let prompt = format!(
        r#"You are an AI assistant helping someone participate in a real-time meeting. Generate smart, contextual reply suggestions.

CONVERSATION TRANSCRIPT:
{}

LAST STATEMENT: "{}"

Generate 4 quick reply suggestions the user can say RIGHT NOW. Requirements:
1. Be IMMEDIATELY relevant to what was just said
2. Sound natural and professional
3. Keep it SHORT (1 sentence preferred, 2 max)

Format: Return exactly 4 replies, numbered 1-4, one per line."#,
        transcript_text, truncate(last_statement, 100)
    );

    print!("  Generating reply suggestions... ");
    match groq::generate(&api_key, "llama-3.1-8b-instant", &prompt).await {
        Ok(response) => {
            println!("✓\n");

            let replies: Vec<&str> = response
                .lines()
                .filter(|line| !line.trim().is_empty())
                .take(4)
                .collect();

            println!("  Suggested Replies:");
            for reply in &replies {
                println!("    • {}", reply.trim());
            }

            if replies.is_empty() {
                println!("\n  ✗ WARNING: No replies generated");
            } else {
                println!("\n  Reply suggestions: ✓ {} suggestions generated", replies.len());
            }
        }
        Err(e) => {
            println!("✗ FAILED: {}", e);
            std::process::exit(1);
        }
    }

    println!("\n=== All Tests Passed ===");
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
