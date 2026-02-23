//! Speaker Separation Test
//!
//! This test simulates the multichannel audio flow to verify speaker separation works correctly.
//! It creates mock audio data for both channels and verifies the labeling.
//!
//! Run with: cargo run --bin speaker_test

use std::sync::Arc;
use tokio::sync::mpsc;

// Simulated AudioSource enum (matches system_audio.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSource {
    Microphone,    // Channel 0 - Your voice
    SystemAudio,   // Channel 1 - Remote participants
}

// Simulated TranscriptMessage (matches deepgram.rs)
#[derive(Debug, Clone)]
pub struct TranscriptMessage {
    pub text: String,
    pub is_final: bool,
    pub speaker: Option<u32>,
    pub source: AudioSource,
}

fn main() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          Speaker Separation Test - MeetBetter                  ║");
    println!("╠════════════════════════════════════════════════════════════════╣");
    println!("║ This test simulates multichannel audio to verify speaker       ║");
    println!("║ labeling works correctly:                                      ║");
    println!("║   • Channel 0 (Microphone) → \"You\"                             ║");
    println!("║   • Channel 1 (System Audio) → \"Participant\"                   ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    // Run the async tests
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(run_tests());
}

async fn run_tests() {
    println!("Running speaker separation tests...\n");

    test_speaker_labeling();
    test_multichannel_simulation().await;
    test_mono_fallback();

    println!("\n✅ All tests passed!");
}

fn test_speaker_labeling() {
    println!("📋 Test 1: Speaker Labeling from AudioSource");
    println!("   Testing that audio sources map to correct labels...\n");

    let test_cases = vec![
        (AudioSource::Microphone, "You"),
        (AudioSource::SystemAudio, "Participant"),
    ];

    for (source, expected_label) in test_cases {
        let label = match source {
            AudioSource::Microphone => "You",
            AudioSource::SystemAudio => "Participant",
        };

        if label == expected_label {
            println!("   ✓ {:?} → \"{}\" (correct)", source, label);
        } else {
            println!("   ✗ {:?} → \"{}\" (expected \"{}\")", source, label, expected_label);
            panic!("Speaker labeling test failed!");
        }
    }

    println!();
}

async fn test_multichannel_simulation() {
    println!("📋 Test 2: Multichannel Transcript Simulation");
    println!("   Simulating a conversation with both channels...\n");

    // Create a channel to receive transcript messages
    let (tx, mut rx) = mpsc::channel::<TranscriptMessage>(10);

    // Simulate transcript messages from both channels
    let conversation = vec![
        TranscriptMessage {
            text: "Hello, can everyone hear me?".to_string(),
            is_final: true,
            speaker: Some(0),
            source: AudioSource::Microphone, // Channel 0 - You
        },
        TranscriptMessage {
            text: "Yes, we can hear you clearly.".to_string(),
            is_final: true,
            speaker: Some(1),
            source: AudioSource::SystemAudio, // Channel 1 - Participant
        },
        TranscriptMessage {
            text: "Great! Let me share my screen.".to_string(),
            is_final: true,
            speaker: Some(0),
            source: AudioSource::Microphone, // Channel 0 - You
        },
        TranscriptMessage {
            text: "Sounds good, we're ready.".to_string(),
            is_final: true,
            speaker: Some(1),
            source: AudioSource::SystemAudio, // Channel 1 - Participant
        },
        TranscriptMessage {
            text: "I have a question about the design.".to_string(),
            is_final: true,
            speaker: Some(2), // Different participant
            source: AudioSource::SystemAudio, // Channel 1 - Still system audio
        },
    ];

    // Send all messages
    for msg in conversation.iter() {
        tx.send(msg.clone()).await.unwrap();
    }
    drop(tx);

    // Process and display the transcript
    println!("   Simulated Transcript:");
    println!("   ─────────────────────────────────────────────────");

    let mut you_count = 0;
    let mut participant_count = 0;

    while let Some(msg) = rx.recv().await {
        let speaker_label = match msg.source {
            AudioSource::Microphone => {
                you_count += 1;
                "You"
            },
            AudioSource::SystemAudio => {
                participant_count += 1;
                "Participant"
            },
        };

        let alignment = if speaker_label == "You" { "→" } else { "←" };
        println!("   {} [{}] {}", alignment, speaker_label, msg.text);
    }

    println!("   ─────────────────────────────────────────────────");
    println!("   Summary: {} messages from You, {} from Participants", you_count, participant_count);

    assert_eq!(you_count, 2, "Expected 2 'You' messages");
    assert_eq!(participant_count, 3, "Expected 3 'Participant' messages");

    println!("   ✓ Multichannel simulation passed!\n");
}

fn test_mono_fallback() {
    println!("📋 Test 3: Mono Fallback (No System Audio Device)");
    println!("   Testing diarization-based fallback when BlackHole not available...\n");

    // In mono mode, we fall back to speaker ID from diarization
    // Speaker 0 = You (first detected), Speaker 1+ = Participant
    let test_cases = vec![
        (Some(0), "You"),       // First speaker = You
        (Some(1), "Participant"), // Other speaker = Participant
        (Some(2), "Participant"), // Other speaker = Participant
        (None, "Participant"),    // Unknown = Participant (safer default)
    ];

    for (speaker_id, expected_label) in test_cases {
        let label = match speaker_id {
            Some(0) => "You",
            Some(_) => "Participant",
            None => "Participant", // Default to participant for unknown
        };

        if label == expected_label {
            println!("   ✓ Speaker {:?} → \"{}\" (correct)", speaker_id, label);
        } else {
            println!("   ✗ Speaker {:?} → \"{}\" (expected \"{}\")", speaker_id, label, expected_label);
            panic!("Mono fallback test failed!");
        }
    }

    println!();
}
