use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
}

impl AppSettings {
    /// Get the path to the settings file
    fn get_settings_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("wiselymeet");
            path.push("settings.json");
            path
        })
    }

    /// Load settings from disk
    pub fn load() -> Self {
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
