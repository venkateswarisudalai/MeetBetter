use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

const OLLAMA_BASE_URL: &str = "http://localhost:11434";

#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    name: String,
}

/// Check if Ollama is running and accessible
pub async fn check_connection() -> Result<bool> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/tags", OLLAMA_BASE_URL))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;

    match response {
        Ok(res) => Ok(res.status().is_success()),
        Err(_) => Ok(false),
    }
}

/// List available models from Ollama
pub async fn list_models() -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/tags", OLLAMA_BASE_URL))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch models: {}",
            response.status()
        ));
    }

    let models: ModelsResponse = response.json().await?;
    Ok(models.models.into_iter().map(|m| m.name).collect())
}

/// Generate a response using the specified model
pub async fn generate(model: &str, prompt: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let request = GenerateRequest {
        model: model.to_string(),
        prompt: prompt.to_string(),
        stream: false,
    };

    let response = client
        .post(format!("{}/api/generate", OLLAMA_BASE_URL))
        .json(&request)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Ollama API error: {}", error_text));
    }

    let result: GenerateResponse = response.json().await?;
    Ok(result.response)
}

/// Generate a chat completion with context
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

pub async fn chat(model: &str, messages: Vec<ChatMessage>) -> Result<String> {
    let client = reqwest::Client::new();

    let request = ChatRequest {
        model: model.to_string(),
        messages,
        stream: false,
    };

    let response = client
        .post(format!("{}/api/chat", OLLAMA_BASE_URL))
        .json(&request)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Ollama API error: {}", error_text));
    }

    let result: ChatResponse = response.json().await?;
    Ok(result.message.content)
}

/// Pull a model from Ollama
pub async fn pull_model(model: &str) -> Result<()> {
    let client = reqwest::Client::new();

    #[derive(Serialize)]
    struct PullRequest {
        name: String,
    }

    let request = PullRequest {
        name: model.to_string(),
    };

    let response = client
        .post(format!("{}/api/pull", OLLAMA_BASE_URL))
        .json(&request)
        .timeout(std::time::Duration::from_secs(600)) // 10 minutes for large models
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Failed to pull model: {}", error_text));
    }

    Ok(())
}
