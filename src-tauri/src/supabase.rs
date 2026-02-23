use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::database::StoredMeeting;

/// Embedded app-level Supabase credentials (baked in at compile time)
pub const SUPABASE_URL: &str = env!("SUPABASE_URL");
pub const SUPABASE_ANON_KEY: &str = env!("SUPABASE_ANON_KEY");

/// Supabase client for cloud sync
pub struct SupabaseClient {
    client: Client,
    url: String,
    anon_key: String,
}

/// Response from Supabase when fetching meetings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupabaseMeeting {
    pub id: String,
    pub title: String,
    pub date: String,
    pub duration_seconds: Option<i64>,
    pub transcript: serde_json::Value,
    pub summary: Option<serde_json::Value>,
    pub attendees: Vec<String>,
    pub calendar_event_id: Option<String>,
    pub recording_path: Option<String>,
    pub recording_url: Option<String>,
    pub user_notes: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl SupabaseClient {
    /// Create a new Supabase client
    pub fn new(url: String, anon_key: String) -> Self {
        Self {
            client: Client::new(),
            url: url.trim_end_matches('/').to_string(),
            anon_key,
        }
    }

    pub fn with_embedded_credentials() -> Self {
        Self::new(SUPABASE_URL.to_string(), SUPABASE_ANON_KEY.to_string())
    }

    /// Check if the client is properly configured
    pub fn is_configured(&self) -> bool {
        !self.url.is_empty() && !self.anon_key.is_empty()
    }

    /// Upsert a meeting (insert or update on conflict)
    pub async fn upsert_meeting(&self, meeting: &StoredMeeting) -> Result<(), String> {
        let url = format!("{}/rest/v1/meetings", self.url);

        let transcript_json = serde_json::to_value(&meeting.transcript)
            .map_err(|e| format!("Failed to serialize transcript: {}", e))?;
        let summary_json = meeting.summary.as_ref()
            .map(|s| serde_json::to_value(s).ok())
            .flatten();

        let body = serde_json::json!({
            "id": meeting.id,
            "title": meeting.title,
            "date": meeting.date,
            "duration_seconds": meeting.duration_seconds,
            "transcript": transcript_json,
            "summary": summary_json,
            "attendees": meeting.attendees,
            "calendar_event_id": meeting.calendar_event_id,
            "recording_path": meeting.recording_path,
            "user_notes": meeting.user_notes,
            "created_at": meeting.created_at,
            "updated_at": meeting.updated_at,
        });

        let response = self.client
            .post(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", &self.anon_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "resolution=merge-duplicates")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Failed to upsert meeting to Supabase: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Supabase upsert failed ({}): {}", status, error_text));
        }

        eprintln!("Meeting '{}' synced to Supabase", meeting.id);
        Ok(())
    }

    /// Delete a meeting from Supabase
    pub async fn delete_meeting(&self, meeting_id: &str) -> Result<(), String> {
        let url = format!("{}/rest/v1/meetings?id=eq.{}", self.url, meeting_id);

        let response = self.client
            .delete(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", &self.anon_key))
            .send()
            .await
            .map_err(|e| format!("Failed to delete meeting from Supabase: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Supabase delete failed ({}): {}", status, error_text));
        }

        eprintln!("Meeting '{}' deleted from Supabase", meeting_id);
        Ok(())
    }

    /// Fetch all meetings from Supabase
    pub async fn fetch_all_meetings(&self) -> Result<Vec<SupabaseMeeting>, String> {
        let url = format!("{}/rest/v1/meetings?order=date.desc", self.url);

        let response = self.client
            .get(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", &self.anon_key))
            .send()
            .await
            .map_err(|e| format!("Failed to fetch meetings from Supabase: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Supabase fetch failed ({}): {}", status, error_text));
        }

        let meetings: Vec<SupabaseMeeting> = response.json().await
            .map_err(|e| format!("Failed to parse Supabase response: {}", e))?;

        Ok(meetings)
    }
}
