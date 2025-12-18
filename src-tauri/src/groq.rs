use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const GROQ_WHISPER_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

/// Available Groq models (all open-source)
pub fn get_available_models() -> Vec<(&'static str, &'static str)> {
    vec![
        ("llama-3.2-90b-vision-preview", "Llama 3.2 90B (Best)"),
        ("llama-3.2-11b-vision-preview", "Llama 3.2 11B"),
        ("llama-3.1-70b-versatile", "Llama 3.1 70B"),
        ("llama-3.1-8b-instant", "Llama 3.1 8B (Fast)"),
        ("mixtral-8x7b-32768", "Mixtral 8x7B"),
        ("gemma2-9b-it", "Gemma 2 9B"),
    ]
}

/// Generate a response using Groq API
pub async fn generate(api_key: &str, model: &str, prompt: &str) -> Result<String> {
    if api_key.is_empty() {
        return Err(anyhow!("Groq API key not set. Get one free at console.groq.com"));
    }

    let client = reqwest::Client::new();

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are a helpful meeting assistant. Be concise and professional.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        },
    ];

    let request = ChatRequest {
        model: model.to_string(),
        messages,
        temperature: 0.7,
        max_tokens: 1024,
    };

    let response = client
        .post(GROQ_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Groq API error ({}): {}", status, error_text));
    }

    let result: ChatResponse = response.json().await?;

    result
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| anyhow!("No response from Groq"))
}

/// Check if API key is valid
pub async fn check_api_key(api_key: &str) -> Result<bool> {
    if api_key.is_empty() {
        return Ok(false);
    }

    let client = reqwest::Client::new();

    let request = ChatRequest {
        model: "llama-3.1-8b-instant".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Hi".to_string(),
        }],
        temperature: 0.1,
        max_tokens: 5,
    };

    let response = client
        .post(GROQ_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    match response {
        Ok(res) => Ok(res.status().is_success()),
        Err(_) => Ok(false),
    }
}

/// Whisper transcription response
#[derive(Debug, Deserialize)]
pub struct WhisperResponse {
    pub text: String,
}

/// Transcribe audio file using Groq's Whisper API
pub async fn transcribe_audio(api_key: &str, file_path: &str) -> Result<String> {
    if api_key.is_empty() {
        return Err(anyhow!("Groq API key not set"));
    }

    let path = Path::new(file_path);
    if !path.exists() {
        return Err(anyhow!("Audio file not found: {}", file_path));
    }

    // Read the audio file
    let file_bytes = tokio::fs::read(file_path).await?;
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.wav")
        .to_string();

    let client = reqwest::Client::new();

    // Create multipart form
    let file_part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name)
        .mime_str("audio/wav")?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", "whisper-large-v3-turbo")
        .text("response_format", "json")
        .text("language", "en");

    let response = client
        .post(GROQ_WHISPER_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Groq Whisper API error ({}): {}", status, error_text));
    }

    let result: WhisperResponse = response.json().await?;
    Ok(result.text)
}

/// Transcribe audio bytes directly (for real-time chunks)
pub async fn transcribe_audio_bytes(api_key: &str, audio_bytes: Vec<u8>, file_name: &str) -> Result<String> {
    if api_key.is_empty() {
        return Err(anyhow!("Groq API key not set"));
    }

    if audio_bytes.len() < 1000 {
        return Err(anyhow!("Audio too short for transcription"));
    }

    let client = reqwest::Client::new();

    // Create multipart form
    let file_part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(file_name.to_string())
        .mime_str("audio/wav")?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", "whisper-large-v3-turbo")
        .text("response_format", "json")
        .text("language", "en");

    let response = client
        .post(GROQ_WHISPER_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Groq Whisper API error ({}): {}", status, error_text));
    }

    let result: WhisperResponse = response.json().await?;
    Ok(result.text)
}
