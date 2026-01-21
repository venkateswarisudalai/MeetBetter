use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

/// Stored meeting with transcript and summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMeeting {
    pub id: String,
    pub title: String,
    pub date: String,
    pub duration_seconds: Option<u64>,
    pub transcript: Vec<TranscriptSegment>,
    pub summary: Option<MeetingSummary>,
    pub attendees: Vec<String>,
    pub calendar_event_id: Option<String>,
    pub recording_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub timestamp: String,
    pub speaker: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingSummary {
    pub key_points: Vec<String>,
    pub action_items: Vec<String>,
    pub decisions: Vec<String>,
    pub notes: Vec<String>,
    pub raw_summary: Option<String>,
}

/// Database for storing meetings (JSON file-based for simplicity)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeetingsDatabase {
    pub meetings: Vec<StoredMeeting>,
}

impl MeetingsDatabase {
    fn get_db_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("vantage");
            path.push("meetings.json");
            path
        })
    }

    pub fn load() -> Self {
        let Some(path) = Self::get_db_path() else {
            eprintln!("Could not determine config directory for meetings database");
            return Self::default();
        };

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str(&content) {
                    Ok(db) => db,
                    Err(e) => {
                        eprintln!("Failed to parse meetings database: {}", e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read meetings database: {}", e);
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_db_path()
            .ok_or_else(|| "Could not determine config directory".to_string())?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize meetings: {}", e))?;

        fs::write(&path, content)
            .map_err(|e| format!("Failed to write meetings file: {}", e))?;

        Ok(())
    }

    /// Add a new meeting
    pub fn add_meeting(&mut self, meeting: StoredMeeting) -> Result<(), String> {
        self.meetings.push(meeting);
        self.save()
    }

    /// Update an existing meeting
    pub fn update_meeting(&mut self, id: &str, meeting: StoredMeeting) -> Result<(), String> {
        if let Some(pos) = self.meetings.iter().position(|m| m.id == id) {
            self.meetings[pos] = meeting;
            self.save()
        } else {
            Err(format!("Meeting not found: {}", id))
        }
    }

    /// Delete a meeting
    pub fn delete_meeting(&mut self, id: &str) -> Result<(), String> {
        if let Some(pos) = self.meetings.iter().position(|m| m.id == id) {
            self.meetings.remove(pos);
            self.save()
        } else {
            Err(format!("Meeting not found: {}", id))
        }
    }

    /// Get a meeting by ID
    pub fn get_meeting(&self, id: &str) -> Option<&StoredMeeting> {
        self.meetings.iter().find(|m| m.id == id)
    }

    /// Get all meetings sorted by date (newest first)
    pub fn get_all_meetings(&self) -> Vec<&StoredMeeting> {
        let mut meetings: Vec<_> = self.meetings.iter().collect();
        meetings.sort_by(|a, b| b.date.cmp(&a.date));
        meetings
    }

    /// Get past meetings (meetings with transcripts)
    pub fn get_past_meetings(&self, limit: Option<usize>) -> Vec<&StoredMeeting> {
        let mut meetings: Vec<_> = self.meetings.iter()
            .filter(|m| !m.transcript.is_empty())
            .collect();
        meetings.sort_by(|a, b| b.date.cmp(&a.date));

        if let Some(limit) = limit {
            meetings.truncate(limit);
        }

        meetings
    }

    /// Search meetings by title or transcript content
    pub fn search_meetings(&self, query: &str) -> Vec<&StoredMeeting> {
        let query_lower = query.to_lowercase();
        self.meetings.iter()
            .filter(|m| {
                m.title.to_lowercase().contains(&query_lower) ||
                m.transcript.iter().any(|t| t.text.to_lowercase().contains(&query_lower))
            })
            .collect()
    }
}

/// Generate a unique meeting ID
pub fn generate_meeting_id() -> String {
    let now = Utc::now();
    format!("meeting_{}", now.format("%Y%m%d_%H%M%S_%3f"))
}

/// Create a new meeting from current transcript
pub fn create_meeting_from_transcript(
    title: String,
    transcript: Vec<crate::TranscriptSegment>,
    summary: Option<crate::MeetingSummary>,
    attendees: Vec<String>,
    calendar_event_id: Option<String>,
    recording_path: Option<String>,
    duration_seconds: Option<u64>,
) -> StoredMeeting {
    let now = Utc::now().to_rfc3339();

    // Convert transcript segments
    let db_transcript: Vec<TranscriptSegment> = transcript
        .into_iter()
        .map(|s| TranscriptSegment {
            timestamp: s.timestamp,
            speaker: s.speaker,
            text: s.text,
        })
        .collect();

    // Convert summary
    let db_summary = summary.map(|s| MeetingSummary {
        key_points: s.key_points,
        action_items: s.action_items,
        decisions: s.decisions,
        notes: s.notes,
        raw_summary: Some(s.raw_summary),
    });

    StoredMeeting {
        id: generate_meeting_id(),
        title,
        date: now.clone(),
        duration_seconds,
        transcript: db_transcript,
        summary: db_summary,
        attendees,
        calendar_event_id,
        recording_path,
        created_at: now.clone(),
        updated_at: now,
    }
}
