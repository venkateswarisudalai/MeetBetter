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

/// Available Groq models (current as of 2025)
pub fn get_available_models() -> Vec<(&'static str, &'static str)> {
    vec![
        ("llama-3.3-70b-versatile", "Llama 3.3 70B (Best)"),
        ("llama-3.1-8b-instant", "Llama 3.1 8B (Fast)"),
        ("llama-3.3-70b-specdec", "Llama 3.3 70B SpecDec"),
        ("mixtral-8x7b-32768", "Mixtral 8x7B"),
        ("gemma2-9b-it", "Gemma 2 9B"),
    ]
}

/// Generate a response using Groq API with automatic rate limit retry.
/// If `proxy_url` is provided, routes through the proxy (demo mode).
pub async fn generate(api_key: &str, model: &str, prompt: &str) -> Result<String> {
    generate_with_proxy(api_key, model, prompt, None).await
}

/// Generate a response, optionally routing through a proxy.
pub async fn generate_with_proxy(api_key: &str, model: &str, prompt: &str, proxy_url: Option<&str>) -> Result<String> {
    generate_with_system(
        api_key,
        model,
        "You are a helpful meeting assistant. Be concise and professional.",
        prompt,
        proxy_url,
    )
    .await
}

/// Generate a response with a caller-supplied system prompt.
///
/// The default assistant system prompt pushes every answer short, which fights
/// tasks that need full spoken sentences (e.g. live reply suggestions).
pub async fn generate_with_system(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    prompt: &str,
    proxy_url: Option<&str>,
) -> Result<String> {
    let use_proxy = proxy_url.is_some() && !proxy_url.unwrap().is_empty();

    if api_key.is_empty() && !use_proxy {
        return Err(anyhow!("Groq API key not set. Get one free at console.groq.com"));
    }

    let client = reqwest::Client::new();

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
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

    // Determine endpoint and auth
    let (url, auth_header) = if use_proxy {
        let proxy = proxy_url.unwrap().trim_end_matches('/');
        eprintln!("Using proxy for Groq: {}/api/groq/chat", proxy);
        (format!("{}/api/groq/chat", proxy), None)
    } else {
        (GROQ_API_URL.to_string(), Some(format!("Bearer {}", api_key)))
    };

    // Retry with exponential backoff for rate limits
    const MAX_RETRIES: u32 = 5;
    let mut retry_delay_ms: u64 = 1000;

    for attempt in 0..MAX_RETRIES {
        let mut req = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(std::time::Duration::from_secs(60));

        if let Some(ref auth) = auth_header {
            req = req.header("Authorization", auth);
        }

        let response = req.send().await?;
        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(|s| s * 1000)
                .unwrap_or(retry_delay_ms);

            let wait_time = retry_after.max(retry_delay_ms);
            eprintln!(
                "Rate limited (attempt {}/{}), waiting {}ms before retry...",
                attempt + 1, MAX_RETRIES, wait_time
            );

            tokio::time::sleep(std::time::Duration::from_millis(wait_time)).await;
            retry_delay_ms = (retry_delay_ms * 2).min(30000);
            continue;
        }

        if status == reqwest::StatusCode::FORBIDDEN {
            let error_text = response.text().await.unwrap_or_default();
            eprintln!("Groq 403 Forbidden with model '{}': {}", request.model, error_text);
            return Err(anyhow!(
                "Groq API key rejected (403 Forbidden). Your API key may be invalid or expired. \
                Please go to Settings and re-enter a valid key from console.groq.com/keys"
            ));
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
        model: "llama-3.3-70b-versatile".to_string(),
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

/// Transcribe audio file using Groq's Whisper API.
/// For files larger than MAX_WHISPER_FILE_SIZE, only transcribes the last portion.
/// If `proxy_url` is provided, routes through the proxy (demo mode).
pub async fn transcribe_audio(api_key: &str, file_path: &str) -> Result<String> {
    transcribe_audio_with_proxy(api_key, file_path, None).await
}

pub async fn transcribe_audio_with_proxy(api_key: &str, file_path: &str, proxy_url: Option<&str>) -> Result<String> {
    let use_proxy = proxy_url.is_some() && !proxy_url.unwrap().is_empty();

    if api_key.is_empty() && !use_proxy {
        return Err(anyhow!("Groq API key not set"));
    }

    let path = Path::new(file_path);
    if !path.exists() {
        return Err(anyhow!("Audio file not found: {}", file_path));
    }

    let metadata = tokio::fs::metadata(file_path).await?;
    let file_size = metadata.len();

    let file_bytes = if file_size <= MAX_WHISPER_FILE_SIZE {
        tokio::fs::read(file_path).await?
    } else {
        eprintln!("Large file detected ({}MB), extracting last {}MB for transcription",
            file_size / 1_000_000, MAX_WHISPER_FILE_SIZE / 1_000_000);
        extract_recent_audio(file_path, MAX_WHISPER_FILE_SIZE as usize).await?
    };

    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.wav")
        .to_string();

    let client = reqwest::Client::new();

    let file_part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name)
        .mime_str("audio/wav")?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", "whisper-large-v3-turbo")
        .text("response_format", "json")
        .text("language", "en");

    let (url, auth_header) = if use_proxy {
        let proxy = proxy_url.unwrap().trim_end_matches('/');
        (format!("{}/api/groq/whisper", proxy), None)
    } else {
        (GROQ_WHISPER_URL.to_string(), Some(format!("Bearer {}", api_key)))
    };

    let mut req = client
        .post(&url)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(120));

    if let Some(auth) = auth_header {
        req = req.header("Authorization", auth);
    }

    let response = req.send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Groq Whisper API error ({}): {}", status, error_text));
    }

    let result: WhisperResponse = response.json().await?;
    Ok(result.text)
}

/// Transcribe audio bytes directly (for real-time chunks).
/// If `proxy_url` is provided, routes through the proxy (demo mode).
pub async fn transcribe_audio_bytes(api_key: &str, audio_bytes: Vec<u8>, file_name: &str) -> Result<String> {
    transcribe_audio_bytes_with_proxy(api_key, audio_bytes, file_name, None).await
}

pub async fn transcribe_audio_bytes_with_proxy(api_key: &str, audio_bytes: Vec<u8>, file_name: &str, proxy_url: Option<&str>) -> Result<String> {
    let use_proxy = proxy_url.is_some() && !proxy_url.unwrap().is_empty();

    if api_key.is_empty() && !use_proxy {
        return Err(anyhow!("Groq API key not set"));
    }

    if audio_bytes.len() < 1000 {
        return Err(anyhow!("Audio too short for transcription"));
    }

    let client = reqwest::Client::new();

    let file_part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(file_name.to_string())
        .mime_str("audio/wav")?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", "whisper-large-v3-turbo")
        .text("response_format", "json")
        .text("language", "en");

    let (url, auth_header) = if use_proxy {
        let proxy = proxy_url.unwrap().trim_end_matches('/');
        (format!("{}/api/groq/whisper", proxy), None)
    } else {
        (GROQ_WHISPER_URL.to_string(), Some(format!("Bearer {}", api_key)))
    };

    let mut req = client
        .post(&url)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(60));

    if let Some(auth) = auth_header {
        req = req.header("Authorization", auth);
    }

    let response = req.send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Groq Whisper API error ({}): {}", status, error_text));
    }

    let result: WhisperResponse = response.json().await?;
    Ok(result.text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_available_models_not_empty() {
        let models = get_available_models();
        assert!(!models.is_empty());
    }

    #[test]
    fn test_get_available_models_has_default() {
        let models = get_available_models();
        let has_default = models.iter().any(|(id, _)| *id == "llama-3.3-70b-versatile");
        assert!(has_default, "Default model llama-3.3-70b-versatile should be available");
    }

    #[test]
    fn test_get_available_models_all_have_labels() {
        let models = get_available_models();
        for (id, label) in &models {
            assert!(!id.is_empty(), "Model ID should not be empty");
            assert!(!label.is_empty(), "Model label should not be empty");
        }
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_chat_message_deserialization() {
        let json = r#"{"role":"assistant","content":"Hi there"}"#;
        let msg: ChatMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "Hi there");
    }

    #[test]
    fn test_chat_request_serialization() {
        let req = ChatRequest {
            model: "llama-3.3-70b-versatile".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            temperature: 0.7,
            max_tokens: 1024,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("llama-3.3-70b-versatile"));
        assert!(json.contains("\"max_tokens\":1024"));
    }

    #[test]
    fn test_whisper_response_deserialization() {
        let json = r#"{"text":"Hello world, this is a test."}"#;
        let resp: WhisperResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.text, "Hello world, this is a test.");
    }

    #[test]
    fn test_whisper_response_empty_text() {
        let json = r#"{"text":""}"#;
        let resp: WhisperResponse = serde_json::from_str(json).unwrap();
        assert!(resp.text.is_empty());
    }

    #[tokio::test]
    async fn test_generate_empty_key_no_proxy_errors() {
        let result = generate("", "llama-3.3-70b-versatile", "Hello").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("API key not set"), "Expected API key error, got: {}", err);
    }

    #[tokio::test]
    async fn test_transcribe_audio_bytes_too_short() {
        let short_bytes = vec![0u8; 100]; // Way too short
        let result = transcribe_audio_bytes("fake-key", short_bytes, "test.wav").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("too short"), "Expected 'too short' error, got: {}", err);
    }

    #[tokio::test]
    async fn test_transcribe_audio_empty_key_no_proxy_errors() {
        let result = transcribe_audio_bytes("", vec![0u8; 2000], "test.wav").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("API key not set"), "Expected API key error, got: {}", err);
    }

    #[tokio::test]
    async fn test_transcribe_file_not_found() {
        let result = transcribe_audio("fake-key", "/nonexistent/path/audio.wav").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"), "Expected not found error, got: {}", err);
    }

    #[test]
    fn test_max_whisper_file_size_reasonable() {
        assert_eq!(MAX_WHISPER_FILE_SIZE, 15_000_000);
        assert!(MAX_WHISPER_FILE_SIZE < 25_000_000, "Should be under Groq's 25MB limit");
    }
}
