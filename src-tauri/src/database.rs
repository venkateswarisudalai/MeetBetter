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
    pub user_notes: Option<String>,
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a minimal `StoredMeeting` with the given `id` and `title`.
    fn make_meeting(id: &str, title: &str) -> StoredMeeting {
        let now = chrono::Utc::now().to_rfc3339();
        StoredMeeting {
            id: id.to_string(),
            title: title.to_string(),
            date: now.clone(),
            duration_seconds: Some(600),
            transcript: vec![],
            summary: None,
            attendees: vec![],
            calendar_event_id: None,
            recording_path: None,
            user_notes: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    // -- Pure in-memory tests (no filesystem side-effects) --------------------

    #[test]
    fn test_add_then_get_meeting_in_memory() {
        let mut db = MeetingsDatabase::default();
        let meeting = make_meeting("m1", "Standup");
        db.meetings.push(meeting);

        let fetched = db.get_meeting("m1");
        assert!(fetched.is_some(), "Meeting should be retrievable after insertion");
        assert_eq!(fetched.unwrap().title, "Standup");
    }

    #[test]
    fn test_delete_meeting_removes_from_vec() {
        let mut db = MeetingsDatabase::default();
        db.meetings.push(make_meeting("m1", "Standup"));
        db.meetings.push(make_meeting("m2", "Retro"));
        assert_eq!(db.meetings.len(), 2);

        // Remove meeting by retaining all others
        db.meetings.retain(|m| m.id != "m1");

        assert_eq!(db.meetings.len(), 1);
        assert!(db.get_meeting("m1").is_none(), "Deleted meeting should not be found");
        assert!(db.get_meeting("m2").is_some(), "Other meeting should still exist");
    }

    #[test]
    fn test_delete_nonexistent_meeting_leaves_vec_unchanged() {
        let mut db = MeetingsDatabase::default();
        db.meetings.push(make_meeting("m1", "Standup"));
        let original_len = db.meetings.len();

        // Trying to find a non-existent meeting yields None
        let pos = db.meetings.iter().position(|m| m.id == "does_not_exist");
        assert!(pos.is_none(), "Non-existent meeting should not be found");

        // Vec is unchanged
        assert_eq!(db.meetings.len(), original_len);
    }

    #[test]
    fn test_get_all_meetings_after_deletion() {
        let mut db = MeetingsDatabase::default();
        db.meetings.push(make_meeting("m1", "Standup"));
        db.meetings.push(make_meeting("m2", "Retro"));
        db.meetings.push(make_meeting("m3", "Planning"));
        assert_eq!(db.get_all_meetings().len(), 3);

        db.meetings.retain(|m| m.id != "m2");

        let all = db.get_all_meetings();
        assert_eq!(all.len(), 2, "Should have 2 meetings after deleting one");
        assert!(all.iter().all(|m| m.id != "m2"), "Deleted meeting must not appear in listing");
    }

    #[test]
    fn test_delete_only_meeting_leaves_empty_db() {
        let mut db = MeetingsDatabase::default();
        db.meetings.push(make_meeting("m1", "Solo Meeting"));
        assert_eq!(db.meetings.len(), 1);

        db.meetings.retain(|m| m.id != "m1");

        assert!(db.meetings.is_empty(), "Database should be empty after deleting the only meeting");
        assert!(db.get_meeting("m1").is_none());
        assert_eq!(db.get_all_meetings().len(), 0);
    }

    #[test]
    fn test_delete_meeting_with_duplicate_ids_removes_first_occurrence() {
        // Edge case: if two meetings accidentally share an ID, the position-based
        // removal in delete_meeting only removes the first one.
        let mut db = MeetingsDatabase::default();
        db.meetings.push(make_meeting("dup", "First"));
        db.meetings.push(make_meeting("dup", "Second"));
        assert_eq!(db.meetings.len(), 2);

        // Simulate the same logic as delete_meeting (position-based remove)
        if let Some(pos) = db.meetings.iter().position(|m| m.id == "dup") {
            db.meetings.remove(pos);
        }

        assert_eq!(db.meetings.len(), 1);
        assert_eq!(db.meetings[0].title, "Second", "Only the first occurrence should be removed");
    }

    #[test]
    fn test_search_meetings_excludes_deleted() {
        let mut db = MeetingsDatabase::default();
        let mut m1 = make_meeting("m1", "Budget Review");
        m1.transcript.push(TranscriptSegment {
            timestamp: "00:01".to_string(),
            speaker: "Alice".to_string(),
            text: "Let us discuss the budget".to_string(),
        });
        db.meetings.push(m1);
        db.meetings.push(make_meeting("m2", "Budget Planning"));

        // Both should appear in search before deletion
        assert_eq!(db.search_meetings("budget").len(), 2);

        // Delete one
        db.meetings.retain(|m| m.id != "m1");

        // Only one should remain in search
        let results = db.search_meetings("budget");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "m2");
    }

    // -- Tests that exercise the actual delete_meeting method (hits filesystem) -

    #[test]
    fn test_delete_meeting_method_returns_ok_for_existing() {
        let mut db = MeetingsDatabase::default();
        db.meetings.push(make_meeting("m1", "Standup"));

        // delete_meeting calls save() which writes to disk -- acceptable in tests
        let result = db.delete_meeting("m1");
        assert!(result.is_ok(), "Deleting an existing meeting should succeed");
        assert!(db.get_meeting("m1").is_none(), "Meeting should be gone after deletion");
    }

    #[test]
    fn test_delete_meeting_method_returns_err_for_nonexistent() {
        let mut db = MeetingsDatabase::default();

        let result = db.delete_meeting("nonexistent_id");
        assert!(result.is_err(), "Deleting a non-existent meeting should return an error");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("Meeting not found"),
            "Error message should indicate meeting was not found, got: {}",
            err_msg
        );
        assert!(
            err_msg.contains("nonexistent_id"),
            "Error message should contain the requested ID, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_delete_meeting_method_correct_count_after_multiple_operations() {
        let mut db = MeetingsDatabase::default();
        db.meetings.push(make_meeting("m1", "Meeting 1"));
        db.meetings.push(make_meeting("m2", "Meeting 2"));
        db.meetings.push(make_meeting("m3", "Meeting 3"));
        assert_eq!(db.meetings.len(), 3);

        // Delete the middle meeting
        let result = db.delete_meeting("m2");
        assert!(result.is_ok());
        assert_eq!(db.meetings.len(), 2);

        // Delete the first meeting
        let result = db.delete_meeting("m1");
        assert!(result.is_ok());
        assert_eq!(db.meetings.len(), 1);

        // The remaining meeting should be m3
        assert!(db.get_meeting("m3").is_some());
        assert_eq!(db.get_all_meetings().len(), 1);
        assert_eq!(db.get_all_meetings()[0].id, "m3");
    }

    #[test]
    fn test_delete_meeting_then_re_add_same_id() {
        let mut db = MeetingsDatabase::default();
        db.meetings.push(make_meeting("m1", "Original"));
        assert_eq!(db.get_meeting("m1").unwrap().title, "Original");

        let _ = db.delete_meeting("m1");
        assert!(db.get_meeting("m1").is_none());

        // Re-add with the same ID but different title
        db.meetings.push(make_meeting("m1", "Re-created"));
        // save is not called here but that is fine for the in-memory assertion
        assert_eq!(db.get_meeting("m1").unwrap().title, "Re-created");
    }

    #[test]
    fn test_delete_meeting_idempotency_second_call_errors() {
        let mut db = MeetingsDatabase::default();
        db.meetings.push(make_meeting("m1", "Standup"));

        let first = db.delete_meeting("m1");
        assert!(first.is_ok(), "First deletion should succeed");

        let second = db.delete_meeting("m1");
        assert!(second.is_err(), "Second deletion of same ID should fail");
    }

    #[test]
    fn test_get_past_meetings_excludes_deleted() {
        let mut db = MeetingsDatabase::default();

        // m1 has a transcript -> qualifies as a "past" meeting
        let mut m1 = make_meeting("m1", "Past Meeting");
        m1.transcript.push(TranscriptSegment {
            timestamp: "00:00".to_string(),
            speaker: "Bob".to_string(),
            text: "Hello".to_string(),
        });
        db.meetings.push(m1);

        // m2 also has a transcript
        let mut m2 = make_meeting("m2", "Another Past Meeting");
        m2.transcript.push(TranscriptSegment {
            timestamp: "00:00".to_string(),
            speaker: "Eve".to_string(),
            text: "Hi there".to_string(),
        });
        db.meetings.push(m2);

        assert_eq!(db.get_past_meetings(None).len(), 2);

        let _ = db.delete_meeting("m1");

        assert_eq!(
            db.get_past_meetings(None).len(),
            1,
            "Past meetings list should reflect deletion"
        );
        assert_eq!(db.get_past_meetings(None)[0].id, "m2");
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
    user_notes: Option<String>,
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
        user_notes,
        created_at: now.clone(),
        updated_at: now,
    }
}
