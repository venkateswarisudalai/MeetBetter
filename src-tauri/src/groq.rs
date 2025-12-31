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

/// Generate a response using Groq API with automatic rate limit retry
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

    // Retry with exponential backoff for rate limits
    const MAX_RETRIES: u32 = 5;
    let mut retry_delay_ms: u64 = 1000;  // Start with 1 second

    for attempt in 0..MAX_RETRIES {
        let response = client
            .post(GROQ_API_URL)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await?;

        let status = response.status();

        // Handle rate limit (429) with exponential backoff
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            // Try to get retry-after header
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(|s| s * 1000)  // Convert seconds to ms
                .unwrap_or(retry_delay_ms);

            let wait_time = retry_after.max(retry_delay_ms);
            eprintln!(
                "Rate limited (attempt {}/{}), waiting {}ms before retry...",
                attempt + 1, MAX_RETRIES, wait_time
            );

            tokio::time::sleep(std::time::Duration::from_millis(wait_time)).await;
            retry_delay_ms = (retry_delay_ms * 2).min(30000);  // Double up to 30s max
            continue;
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Groq API error ({}): {}", status, error_text));
        }

        let result: ChatResponse = response.json().await?;
        return result
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No response from Groq"));
    }

    Err(anyhow!("Rate limit exceeded after {} retries", MAX_RETRIES))
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

/// Maximum file size for Groq Whisper API (25MB, use 15MB to be safe)
const MAX_WHISPER_FILE_SIZE: u64 = 15_000_000;

/// WAV file header size (44 bytes standard)
const WAV_HEADER_SIZE: usize = 44;

/// Extract the most recent portion of a WAV file for transcription
/// Creates a new valid WAV with proper headers containing only the last `max_size` bytes of audio
async fn extract_recent_audio(file_path: &str, max_size: usize) -> Result<Vec<u8>> {
    use tokio::io::{AsyncReadExt, AsyncSeekExt};
    use std::io::SeekFrom;

    let mut file = tokio::fs::File::open(file_path).await?;
    let file_size = file.metadata().await?.len() as usize;

    // Read the original WAV header (first 44 bytes)
    let mut header = vec![0u8; WAV_HEADER_SIZE];
    file.read_exact(&mut header).await?;

    // Calculate how much audio data to read (excluding header)
    let total_audio_data = file_size - WAV_HEADER_SIZE;
    let audio_to_read = (max_size - WAV_HEADER_SIZE).min(total_audio_data);

    // Seek to the position where we want to start reading
    let start_pos = file_size - audio_to_read;
    file.seek(SeekFrom::Start(start_pos as u64)).await?;

    // Read the audio data
    let mut audio_data = vec![0u8; audio_to_read];
    file.read_exact(&mut audio_data).await?;

    // Update the WAV header with correct sizes
    // Bytes 4-7: File size - 8 (little endian)
    let new_file_size = (WAV_HEADER_SIZE + audio_to_read - 8) as u32;
    header[4..8].copy_from_slice(&new_file_size.to_le_bytes());

    // Bytes 40-43: Data chunk size (little endian)
    let data_size = audio_to_read as u32;
    header[40..44].copy_from_slice(&data_size.to_le_bytes());

    // Combine header and audio data
    let mut result = header;
    result.extend(audio_data);

    eprintln!("Extracted {}MB of recent audio from {}MB file",
        result.len() / 1_000_000, file_size / 1_000_000);

    Ok(result)
}

/// Transcribe audio file using Groq's Whisper API
/// For files larger than MAX_WHISPER_FILE_SIZE, only transcribes the last portion
pub async fn transcribe_audio(api_key: &str, file_path: &str) -> Result<String> {
    if api_key.is_empty() {
        return Err(anyhow!("Groq API key not set"));
    }

    let path = Path::new(file_path);
    if !path.exists() {
        return Err(anyhow!("Audio file not found: {}", file_path));
    }

    let metadata = tokio::fs::metadata(file_path).await?;
    let file_size = metadata.len();

    // If file is small enough, read the whole thing
    let file_bytes = if file_size <= MAX_WHISPER_FILE_SIZE {
        tokio::fs::read(file_path).await?
    } else {
        // File too large - extract only the last portion
        eprintln!("Large file detected ({}MB), extracting last {}MB for transcription",
            file_size / 1_000_000, MAX_WHISPER_FILE_SIZE / 1_000_000);
        extract_recent_audio(file_path, MAX_WHISPER_FILE_SIZE as usize).await?
    };

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
