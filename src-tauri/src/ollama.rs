use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

const OLLAMA_BASE_URL: &str = "http://localhost:11434";

/// Best first: qwen3:14b writes much better notes on 16 GB+ Macs; llama3.2 (2 GB) runs anywhere.
const PREFERRED_MODELS: &[&str] = &["qwen3:14b", "qwen3:8b", "gemma3:12b", "llama3.1:8b", "llama3.2"];

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
    /// Thinking models (qwen3, deepseek-r1) otherwise reason for ~40 s first; non-thinking models accept it.
    think: bool,
    options: ChatOptions,
}

#[derive(Debug, Serialize)]
struct ChatOptions {
    temperature: f32,
    /// Ollama's default context (4K tokens) silently cuts long meetings.
    num_ctx: u32,
}

/// Enough context for the prompt plus a reply, in steps that keep memory use predictable.
fn context_window(prompt_chars: usize) -> u32 {
    let needed = (prompt_chars / 3 + 2_048) as u32;
    [8_192, 16_384, 32_768].into_iter().find(|&n| n >= needed).unwrap_or(32_768)
}

/// The saved model if it's installed, otherwise the best installed one.
pub fn pick_model(installed: &[String], saved: &str) -> Option<String> {
    let has = |name: &str| installed.iter().any(|m| m == name || m.strip_suffix(":latest") == Some(name));
    if !saved.is_empty() && has(saved) {
        return Some(saved.to_string());
    }
    PREFERRED_MODELS.iter().find(|m| has(m)).map(|m| m.to_string()).or_else(|| installed.first().cloned())
}

/// A chat reply with a system prompt, for summaries and suggestions.
pub async fn chat_with_system(model: &str, system: &str, prompt: &str) -> Result<String> {
    chat(model, vec![
        ChatMessage { role: "system".to_string(), content: system.to_string() },
        ChatMessage { role: "user".to_string(), content: prompt.to_string() },
    ])
    .await
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

    let prompt_chars = messages.iter().map(|m| m.content.len()).sum();
    let request = ChatRequest {
        model: model.to_string(),
        messages,
        stream: false,
        think: false,
        options: ChatOptions { temperature: 0.3, num_ctx: context_window(prompt_chars) },
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

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn picks_saved_model_when_installed() {
        assert_eq!(pick_model(&names(&["llama3.2:latest", "qwen3:14b"]), "llama3.2"), Some("llama3.2".to_string()));
    }

    #[test]
    fn picks_best_installed_when_saved_is_missing_or_empty() {
        assert_eq!(pick_model(&names(&["llama3.2:latest", "qwen3:14b"]), ""), Some("qwen3:14b".to_string()));
        assert_eq!(pick_model(&names(&["llama3.2:latest"]), "qwen3:14b"), Some("llama3.2".to_string()));
        assert_eq!(pick_model(&names(&["mistral:7b"]), ""), Some("mistral:7b".to_string()));
        assert_eq!(pick_model(&[], ""), None);
    }

    #[test]
    fn chat_request_turns_off_thinking_and_sizes_context() {
        let req = ChatRequest {
            model: "qwen3:14b".into(),
            messages: vec![],
            stream: false,
            think: false,
            options: ChatOptions { temperature: 0.3, num_ctx: context_window(90_000) },
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"think\":false"));
        assert!(json.contains("\"num_ctx\":32768"));
        assert_eq!(context_window(1_000), 8_192);
    }

    /// Needs Ollama running locally: `cargo test --lib ollama -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn live_summary_from_local_model() {
        let installed = list_models().await.expect("Ollama is running");
        let model = pick_model(&installed, "").expect("a model is installed");
        let started = std::time::Instant::now();
        let reply = chat_with_system(
            &model,
            "You are a helpful meeting assistant. Be concise and professional.",
            "Summarize with ## KEY POINTS and ## ACTION ITEMS.\n\nMEETING TRANSCRIPT:\n[00:01] Participant: We launch Thursday unless the Safari bug slips.\n[00:09] You: I'll tell marketing about Friday.\n[00:15] Participant: Sam, can you own the rollback plan by Wednesday?",
        )
        .await
        .expect("a reply");
        println!("model={} secs={:.1}\n{}", model, started.elapsed().as_secs_f32(), reply);
        assert!(reply.contains("ACTION ITEMS"), "follows the format");
        assert!(!reply.contains("<think>"), "no reasoning in the reply");
    }
}
