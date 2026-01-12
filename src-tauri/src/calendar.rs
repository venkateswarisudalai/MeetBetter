use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use reqwest::Client;
use chrono::{DateTime, Utc, Duration};

/// Google OAuth2 configuration
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_CALENDAR_API: &str = "https://www.googleapis.com/calendar/v3";
const SCOPES: &str = "https://www.googleapis.com/auth/calendar.readonly";

/// Calendar event from Google Calendar API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start: EventDateTime,
    pub end: EventDateTime,
    pub attendees: Option<Vec<Attendee>>,
    pub html_link: Option<String>,
    pub conference_data: Option<ConferenceData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDateTime {
    #[serde(rename = "dateTime")]
    pub date_time: Option<String>,
    pub date: Option<String>,
    #[serde(rename = "timeZone")]
    pub time_zone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendee {
    pub email: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "responseStatus")]
    pub response_status: Option<String>,
    #[serde(rename = "self")]
    pub is_self: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConferenceData {
    #[serde(rename = "entryPoints")]
    pub entry_points: Option<Vec<EntryPoint>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    #[serde(rename = "entryPointType")]
    pub entry_point_type: Option<String>,
    pub uri: Option<String>,
    pub label: Option<String>,
}

/// Simplified calendar event for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleCalendarEvent {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub attendees: Vec<String>,
    pub meeting_link: Option<String>,
    pub is_today: bool,
    pub is_past: bool,
}

/// Google Calendar API response for events list
#[derive(Debug, Deserialize)]
struct EventsListResponse {
    items: Option<Vec<CalendarEvent>>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

/// OAuth2 tokens
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}

impl GoogleTokens {
    fn get_tokens_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("vantage");
            path.push("google_tokens.json");
            path
        })
    }

    pub fn load() -> Option<Self> {
        let path = Self::get_tokens_path()?;
        if !path.exists() {
            return None;
        }
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_tokens_path()
            .ok_or_else(|| "Could not determine config directory".to_string())?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize tokens: {}", e))?;

        fs::write(&path, content)
            .map_err(|e| format!("Failed to write tokens file: {}", e))?;

        Ok(())
    }

    pub fn delete() -> Result<(), String> {
        if let Some(path) = Self::get_tokens_path() {
            if path.exists() {
                fs::remove_file(&path)
                    .map_err(|e| format!("Failed to delete tokens file: {}", e))?;
            }
        }
        Ok(())
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = Utc::now().timestamp();
            now >= expires_at - 60 // Refresh 1 minute before expiry
        } else {
            true
        }
    }
}

/// Google Calendar client
pub struct GoogleCalendar {
    client: Client,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl GoogleCalendar {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client: Client::new(),
            client_id,
            client_secret,
            redirect_uri: "http://localhost:8765/callback".to_string(),
        }
    }

    /// Generate the OAuth2 authorization URL
    pub fn get_auth_url(&self) -> String {
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
            GOOGLE_AUTH_URL,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.redirect_uri),
            urlencoding::encode(SCOPES)
        )
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(&self, code: &str) -> Result<GoogleTokens, String> {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", self.redirect_uri.as_str()),
        ];

        let response = self.client
            .post(GOOGLE_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Failed to exchange code: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Token exchange failed: {}", error_text));
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            refresh_token: Option<String>,
            expires_in: Option<i64>,
        }

        let token_response: TokenResponse = response.json().await
            .map_err(|e| format!("Failed to parse token response: {}", e))?;

        let expires_at = token_response.expires_in
            .map(|exp| Utc::now().timestamp() + exp);

        let tokens = GoogleTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at,
        };

        tokens.save()?;
        Ok(tokens)
    }

    /// Refresh access token using refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<GoogleTokens, String> {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let response = self.client
            .post(GOOGLE_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Failed to refresh token: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Token refresh failed: {}", error_text));
        }

        #[derive(Deserialize)]
        struct RefreshResponse {
            access_token: String,
            expires_in: Option<i64>,
        }

        let refresh_response: RefreshResponse = response.json().await
            .map_err(|e| format!("Failed to parse refresh response: {}", e))?;

        let expires_at = refresh_response.expires_in
            .map(|exp| Utc::now().timestamp() + exp);

        let tokens = GoogleTokens {
            access_token: refresh_response.access_token,
            refresh_token: Some(refresh_token.to_string()),
            expires_at,
        };

        tokens.save()?;
        Ok(tokens)
    }

    /// Get valid access token (refresh if needed)
    pub async fn get_valid_token(&self) -> Result<String, String> {
        let tokens = GoogleTokens::load()
            .ok_or_else(|| "Not authenticated with Google. Please connect your calendar.".to_string())?;

        if tokens.is_expired() {
            if let Some(refresh_token) = &tokens.refresh_token {
                let new_tokens = self.refresh_token(refresh_token).await?;
                return Ok(new_tokens.access_token);
            } else {
                return Err("Token expired and no refresh token available".to_string());
            }
        }

        Ok(tokens.access_token)
    }

    /// Fetch calendar events
    pub async fn get_events(
        &self,
        time_min: Option<DateTime<Utc>>,
        time_max: Option<DateTime<Utc>>,
        max_results: Option<u32>,
    ) -> Result<Vec<SimpleCalendarEvent>, String> {
        let access_token = self.get_valid_token().await?;

        let now = Utc::now();
        let time_min = time_min.unwrap_or_else(|| now - Duration::days(30));
        let time_max = time_max.unwrap_or_else(|| now + Duration::days(7));
        let max_results = max_results.unwrap_or(50);

        let url = format!(
            "{}/calendars/primary/events?timeMin={}&timeMax={}&maxResults={}&singleEvents=true&orderBy=startTime",
            GOOGLE_CALENDAR_API,
            urlencoding::encode(&time_min.to_rfc3339()),
            urlencoding::encode(&time_max.to_rfc3339()),
            max_results
        );

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| format!("Failed to fetch events: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Failed to fetch events: {}", error_text));
        }

        let events_response: EventsListResponse = response.json().await
            .map_err(|e| format!("Failed to parse events: {}", e))?;

        let events = events_response.items.unwrap_or_default();
        let today = Utc::now().date_naive();

        let simple_events: Vec<SimpleCalendarEvent> = events
            .into_iter()
            .map(|e| {
                let start_time = e.start.date_time.clone()
                    .or(e.start.date.clone())
                    .unwrap_or_default();
                let end_time = e.end.date_time.clone()
                    .or(e.end.date.clone())
                    .unwrap_or_default();

                // Parse start time to check if today/past
                let is_today = start_time.parse::<DateTime<Utc>>()
                    .map(|dt| dt.date_naive() == today)
                    .unwrap_or(false);
                let is_past = start_time.parse::<DateTime<Utc>>()
                    .map(|dt| dt < now)
                    .unwrap_or(false);

                // Extract meeting link
                let meeting_link = e.conference_data
                    .and_then(|cd| cd.entry_points)
                    .and_then(|eps| eps.into_iter()
                        .find(|ep| ep.entry_point_type.as_deref() == Some("video"))
                        .and_then(|ep| ep.uri));

                // Extract attendees
                let attendees = e.attendees
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|a| a.is_self != Some(true))
                    .filter_map(|a| a.display_name.or(a.email))
                    .collect();

                SimpleCalendarEvent {
                    id: e.id,
                    title: e.summary.unwrap_or_else(|| "(No title)".to_string()),
                    description: e.description,
                    start_time,
                    end_time,
                    attendees,
                    meeting_link,
                    is_today,
                    is_past,
                }
            })
            .collect();

        Ok(simple_events)
    }

    /// Get upcoming events (today and future)
    pub async fn get_upcoming_events(&self, limit: Option<u32>) -> Result<Vec<SimpleCalendarEvent>, String> {
        let now = Utc::now();
        let events = self.get_events(
            Some(now),
            Some(now + Duration::days(30)),
            limit,
        ).await?;
        Ok(events)
    }

    /// Get past events
    pub async fn get_past_events(&self, days: Option<i64>, limit: Option<u32>) -> Result<Vec<SimpleCalendarEvent>, String> {
        let now = Utc::now();
        let days = days.unwrap_or(30);
        let events = self.get_events(
            Some(now - Duration::days(days)),
            Some(now),
            limit,
        ).await?;

        // Return in reverse chronological order (most recent first)
        let mut past_events: Vec<_> = events.into_iter().filter(|e| e.is_past).collect();
        past_events.reverse();
        Ok(past_events)
    }
}

/// Check if Google Calendar is connected
pub fn is_calendar_connected() -> bool {
    GoogleTokens::load().is_some()
}

/// Disconnect Google Calendar
pub fn disconnect_calendar() -> Result<(), String> {
    GoogleTokens::delete()
}
