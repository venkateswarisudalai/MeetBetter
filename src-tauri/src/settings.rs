use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Environment variable names for API keys
/// These take priority over settings file
pub const ENV_GROQ_API_KEY: &str = "VANTAGE_GROQ_API_KEY";
pub const ENV_DEEPGRAM_API_KEY: &str = "VANTAGE_DEEPGRAM_API_KEY";
pub const ENV_ASSEMBLYAI_API_KEY: &str = "VANTAGE_ASSEMBLYAI_API_KEY";
pub const ENV_PROXY_URL: &str = "VANTAGE_PROXY_URL";

/// Default proxy URL baked into the build.
/// Change this to your deployed Cloudflare Worker URL before building for distribution.
/// Set to "" to disable demo mode by default.
pub const DEFAULT_PROXY_URL: &str = "https://vantage-api-proxy.venkateswari1095.workers.dev";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub groq_api_key: String,
    #[serde(default)]
    pub assemblyai_api_key: String,
    #[serde(default)]
    pub deepgram_api_key: String,
    #[serde(default)]
    pub selected_model: String,
    #[serde(default)]
    pub transcription_provider: String,
    #[serde(default)]
    pub meeting_context: String,
    #[serde(default)]
    pub cloud_sync_enabled: bool,
    /// Proxy URL for demo mode (e.g. "https://vantage-api-proxy.your-subdomain.workers.dev")
    /// When set and personal API keys are empty, the app routes through this proxy.
    #[serde(default = "default_proxy_url")]
    pub proxy_url: String,
}

fn default_proxy_url() -> String {
    DEFAULT_PROXY_URL.to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            groq_api_key: String::new(),
            assemblyai_api_key: String::new(),
            deepgram_api_key: String::new(),
            selected_model: String::new(),
            transcription_provider: String::new(),
            meeting_context: String::new(),
            cloud_sync_enabled: false,
            proxy_url: DEFAULT_PROXY_URL.to_string(),
        }
    }
}

impl AppSettings {
    /// Get the path to the settings file
    fn get_settings_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("vantage");
            path.push("settings.json");
            path
        })
    }

    /// Load settings from disk, with environment variables taking priority
    pub fn load() -> Self {
        // First, load from config file
        let mut settings = Self::load_from_file();

        // Then override with environment variables (if set)
        settings.apply_env_overrides();

        settings
    }

    /// Load settings from config file only
    fn load_from_file() -> Self {
        let Some(path) = Self::get_settings_path() else {
            eprintln!("Could not determine config directory");
            return Self::default();
        };

        if !path.exists() {
            eprintln!("Settings file does not exist, using defaults");
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str(&content) {
                    Ok(settings) => {
                        eprintln!("Settings loaded from {:?}", path);
                        settings
                    }
                    Err(e) => {
                        eprintln!("Failed to parse settings: {}", e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read settings file: {}", e);
                Self::default()
            }
        }
    }

    /// Apply environment variable overrides for API keys
    /// Environment variables take priority over config file
    fn apply_env_overrides(&mut self) {
        // Groq API key
        if let Ok(key) = std::env::var(ENV_GROQ_API_KEY) {
            if !key.is_empty() {
                eprintln!("Using Groq API key from environment variable");
                self.groq_api_key = key;
            }
        }

        // Deepgram API key
        if let Ok(key) = std::env::var(ENV_DEEPGRAM_API_KEY) {
            if !key.is_empty() {
                eprintln!("Using Deepgram API key from environment variable");
                self.deepgram_api_key = key;
            }
        }

        // AssemblyAI API key
        if let Ok(key) = std::env::var(ENV_ASSEMBLYAI_API_KEY) {
            if !key.is_empty() {
                eprintln!("Using AssemblyAI API key from environment variable");
                self.assemblyai_api_key = key;
            }
        }

        // Proxy URL
        if let Ok(url) = std::env::var(ENV_PROXY_URL) {
            if !url.is_empty() {
                eprintln!("Using proxy URL from environment variable");
                self.proxy_url = url;
            }
        }
    }

    /// Save settings to disk
    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_settings_path()
            .ok_or_else(|| "Could not determine config directory".to_string())?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(&path, content)
            .map_err(|e| format!("Failed to write settings file: {}", e))?;

        eprintln!("Settings saved to {:?}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings_empty_api_keys() {
        let settings = AppSettings::default();
        assert!(settings.groq_api_key.is_empty());
        assert!(settings.assemblyai_api_key.is_empty());
        assert!(settings.deepgram_api_key.is_empty());
    }

    #[test]
    fn test_default_settings_empty_model() {
        let settings = AppSettings::default();
        assert!(settings.selected_model.is_empty());
        assert!(settings.transcription_provider.is_empty());
    }

    #[test]
    fn test_default_settings_cloud_sync_disabled() {
        let settings = AppSettings::default();
        assert!(!settings.cloud_sync_enabled);
    }

    #[test]
    fn test_default_proxy_url_populated() {
        let settings = AppSettings::default();
        assert_eq!(settings.proxy_url, DEFAULT_PROXY_URL);
        assert!(!settings.proxy_url.is_empty());
    }

    #[test]
    fn test_settings_serialize_deserialize_roundtrip() {
        let settings = AppSettings {
            groq_api_key: "test-groq-key".to_string(),
            assemblyai_api_key: "test-aai-key".to_string(),
            deepgram_api_key: "test-dg-key".to_string(),
            selected_model: "llama-3.3-70b-versatile".to_string(),
            transcription_provider: "deepgram".to_string(),
            meeting_context: "Weekly standup".to_string(),
            cloud_sync_enabled: true,
            proxy_url: "https://example.com".to_string(),
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.groq_api_key, "test-groq-key");
        assert_eq!(deserialized.assemblyai_api_key, "test-aai-key");
        assert_eq!(deserialized.deepgram_api_key, "test-dg-key");
        assert_eq!(deserialized.selected_model, "llama-3.3-70b-versatile");
        assert_eq!(deserialized.transcription_provider, "deepgram");
        assert_eq!(deserialized.meeting_context, "Weekly standup");
        assert!(deserialized.cloud_sync_enabled);
        assert_eq!(deserialized.proxy_url, "https://example.com");
    }

    #[test]
    fn test_settings_deserialize_missing_fields_uses_defaults() {
        // Simulate an older settings file that doesn't have all fields
        let json = r#"{"groq_api_key": "my-key"}"#;
        let settings: AppSettings = serde_json::from_str(json).unwrap();

        assert_eq!(settings.groq_api_key, "my-key");
        assert!(settings.assemblyai_api_key.is_empty());
        assert!(settings.deepgram_api_key.is_empty());
        assert!(!settings.cloud_sync_enabled);
        assert_eq!(settings.proxy_url, DEFAULT_PROXY_URL);
    }

    #[test]
    fn test_settings_deserialize_empty_json() {
        let json = "{}";
        let settings: AppSettings = serde_json::from_str(json).unwrap();

        assert!(settings.groq_api_key.is_empty());
        assert_eq!(settings.proxy_url, DEFAULT_PROXY_URL);
    }

    #[test]
    fn test_env_override_groq_key() {
        let mut settings = AppSettings::default();
        assert!(settings.groq_api_key.is_empty());

        // Simulate env override
        std::env::set_var(ENV_GROQ_API_KEY, "env-groq-key");
        settings.apply_env_overrides();
        assert_eq!(settings.groq_api_key, "env-groq-key");

        // Clean up
        std::env::remove_var(ENV_GROQ_API_KEY);
    }

    #[test]
    fn test_env_override_empty_value_no_change() {
        let mut settings = AppSettings {
            groq_api_key: "file-key".to_string(),
            ..AppSettings::default()
        };

        std::env::set_var(ENV_GROQ_API_KEY, "");
        settings.apply_env_overrides();
        // Empty env var should NOT override
        assert_eq!(settings.groq_api_key, "file-key");

        std::env::remove_var(ENV_GROQ_API_KEY);
    }

    #[test]
    fn test_env_constants_defined() {
        assert_eq!(ENV_GROQ_API_KEY, "VANTAGE_GROQ_API_KEY");
        assert_eq!(ENV_DEEPGRAM_API_KEY, "VANTAGE_DEEPGRAM_API_KEY");
        assert_eq!(ENV_ASSEMBLYAI_API_KEY, "VANTAGE_ASSEMBLYAI_API_KEY");
        assert_eq!(ENV_PROXY_URL, "VANTAGE_PROXY_URL");
    }

    #[test]
    fn test_settings_path_ends_with_expected() {
        if let Some(path) = AppSettings::get_settings_path() {
            assert!(path.ends_with("vantage/settings.json"));
        }
        // If config_dir() returns None (unlikely on macOS), test just passes
    }
}
