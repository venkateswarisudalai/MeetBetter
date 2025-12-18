use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

const ASSEMBLYAI_API_URL: &str = "https://api.assemblyai.com/v2";

#[derive(Debug, Serialize)]
struct TranscriptRequest {
    audio_url: String,
    speaker_labels: bool,
}

#[derive(Debug, Deserialize)]
pub struct TranscriptResponse {
    pub id: String,
    pub status: String,
    pub text: Option<String>,
    pub utterances: Option<Vec<Utterance>>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Utterance {
    pub speaker: String,
    pub text: String,
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Serialize)]
struct UploadResponse {
    upload_url: String,
}

/// Check if AssemblyAI API key is valid
pub async fn check_api_key(api_key: &str) -> Result<bool> {
    if api_key.is_empty() {
        return Ok(false);
    }

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/transcript", ASSEMBLYAI_API_URL))
        .header("Authorization", api_key)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    match response {
        Ok(res) => Ok(res.status().is_success() || res.status().as_u16() == 200),
        Err(_) => Ok(false),
    }
}

/// Upload audio file to AssemblyAI
pub async fn upload_audio(api_key: &str, file_path: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let file_content = std::fs::read(file_path)?;

    let response = client
        .post(format!("{}/upload", ASSEMBLYAI_API_URL))
        .header("Authorization", api_key)
        .header("Content-Type", "application/octet-stream")
        .body(file_content)
        .timeout(std::time::Duration::from_secs(300))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Failed to upload audio: {}", error_text));
    }

    #[derive(Deserialize)]
    struct UploadResp {
        upload_url: String,
    }

    let result: UploadResp = response.json().await?;
    Ok(result.upload_url)
}

/// Start transcription job
pub async fn start_transcription(api_key: &str, audio_url: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let request = TranscriptRequest {
        audio_url: audio_url.to_string(),
        speaker_labels: true,
    };

    let response = client
        .post(format!("{}/transcript", ASSEMBLYAI_API_URL))
        .header("Authorization", api_key)
        .header("Content-Type", "application/json")
        .json(&request)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Failed to start transcription: {}", error_text));
    }

    let result: TranscriptResponse = response.json().await?;
    Ok(result.id)
}

/// Get transcription result
pub async fn get_transcription(api_key: &str, transcript_id: &str) -> Result<TranscriptResponse> {
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/transcript/{}", ASSEMBLYAI_API_URL, transcript_id))
        .header("Authorization", api_key)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Failed to get transcription: {}", error_text));
    }

    let result: TranscriptResponse = response.json().await?;
    Ok(result)
}

/// Poll for transcription completion
pub async fn wait_for_transcription(api_key: &str, transcript_id: &str) -> Result<TranscriptResponse> {
    loop {
        let result = get_transcription(api_key, transcript_id).await?;

        match result.status.as_str() {
            "completed" => return Ok(result),
            "error" => {
                return Err(anyhow!(
                    "Transcription failed: {}",
                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                ));
            }
            _ => {
                // Still processing, wait and retry
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }
    }
}

/// Transcribe an audio file (upload, start, and wait for result)
pub async fn transcribe_file(api_key: &str, file_path: &str) -> Result<TranscriptResponse> {
    // Upload the file
    let upload_url = upload_audio(api_key, file_path).await?;

    // Start transcription
    let transcript_id = start_transcription(api_key, &upload_url).await?;

    // Wait for completion
    wait_for_transcription(api_key, &transcript_id).await
}

/// Poll for transcription completion with configurable poll interval
async fn wait_for_transcription_fast(api_key: &str, transcript_id: &str, poll_interval_ms: u64) -> Result<TranscriptResponse> {
    loop {
        let result = get_transcription(api_key, transcript_id).await?;

        match result.status.as_str() {
            "completed" => return Ok(result),
            "error" => {
                return Err(anyhow!(
                    "Transcription failed: {}",
                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                ));
            }
            _ => {
                // Still processing, wait and retry with configurable interval
                tokio::time::sleep(std::time::Duration::from_millis(poll_interval_ms)).await;
            }
        }
    }
}

/// Transcribe an audio file with faster polling for lower latency
pub async fn transcribe_file_fast(api_key: &str, file_path: &str, poll_interval_ms: u64) -> Result<TranscriptResponse> {
    // Upload the file
    let upload_url = upload_audio(api_key, file_path).await?;

    // Start transcription (without speaker labels for faster processing)
    let client = reqwest::Client::new();

    #[derive(serde::Serialize)]
    struct FastTranscriptRequest {
        audio_url: String,
        speaker_labels: bool,
    }

    let request = FastTranscriptRequest {
        audio_url: upload_url,
        speaker_labels: false, // Disable for faster processing
    };

    let response = client
        .post(format!("{}/transcript", ASSEMBLYAI_API_URL))
        .header("Authorization", api_key)
        .header("Content-Type", "application/json")
        .json(&request)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Failed to start transcription: {}", error_text));
    }

    let result: TranscriptResponse = response.json().await?;

    // Wait for completion with faster polling
    wait_for_transcription_fast(api_key, &result.id, poll_interval_ms).await
}
