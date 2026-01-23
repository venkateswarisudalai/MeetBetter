use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use std::process::Command;

use crate::calendar::{GoogleCalendar, SimpleCalendarEvent};

/// Meeting monitor settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingMonitorSettings {
    /// Enable auto-start when meeting begins
    pub enabled: bool,
    /// Minutes before meeting start to begin monitoring
    pub start_buffer_minutes: i64,
    /// Detect meeting apps (Zoom, Teams, Meet)
    pub detect_meeting_apps: bool,
    /// Auto-start based on calendar time
    pub auto_start_on_time: bool,
}

impl Default for MeetingMonitorSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            start_buffer_minutes: 2,
            detect_meeting_apps: true,
            auto_start_on_time: true,
        }
    }
}

/// Current meeting status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingStatus {
    pub is_meeting_detected: bool,
    pub meeting_app_running: Option<String>,
    pub upcoming_meeting: Option<SimpleCalendarEvent>,
    pub minutes_until_meeting: Option<i64>,
    pub auto_start_triggered: bool,
}

/// Meeting monitor state
pub struct MeetingMonitor {
    settings: Arc<RwLock<MeetingMonitorSettings>>,
    status: Arc<RwLock<MeetingStatus>>,
    last_triggered_event_id: Arc<RwLock<Option<String>>>,
}

impl MeetingMonitor {
    pub fn new() -> Self {
        Self {
            settings: Arc::new(RwLock::new(MeetingMonitorSettings::default())),
            status: Arc::new(RwLock::new(MeetingStatus {
                is_meeting_detected: false,
                meeting_app_running: None,
                upcoming_meeting: None,
                minutes_until_meeting: None,
                auto_start_triggered: false,
            })),
            last_triggered_event_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Update monitor settings
    pub async fn update_settings(&self, settings: MeetingMonitorSettings) {
        let mut current_settings = self.settings.write().await;
        *current_settings = settings;
    }

    /// Get current settings
    pub async fn get_settings(&self) -> MeetingMonitorSettings {
        self.settings.read().await.clone()
    }

    /// Get current meeting status
    pub async fn get_status(&self) -> MeetingStatus {
        self.status.read().await.clone()
    }

    /// Check if any meeting apps are running
    fn detect_meeting_apps() -> Option<String> {
        let meeting_apps = vec![
            ("zoom.us", "Zoom"),
            ("Microsoft Teams", "Teams"),
            ("Google Meet", "Google Meet"),
            ("meet.google.com", "Google Meet"),
            ("Webex", "Webex"),
            ("Slack", "Slack Call"),
        ];

        for (process_name, display_name) in meeting_apps {
            if Self::is_process_running(process_name) {
                return Some(display_name.to_string());
            }
        }
        None
    }

    /// Check if a process is running (macOS)
    #[cfg(target_os = "macos")]
    fn is_process_running(process_name: &str) -> bool {
        let output = Command::new("pgrep")
            .arg("-f")
            .arg(process_name)
            .output();

        match output {
            Ok(result) => !result.stdout.is_empty(),
            Err(_) => false,
        }
    }

    /// Check if a process is running (Windows)
    #[cfg(target_os = "windows")]
    fn is_process_running(process_name: &str) -> bool {
        let output = Command::new("tasklist")
            .args(&["/FI", &format!("IMAGENAME eq {}", process_name)])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                stdout.contains(process_name)
            }
            Err(_) => false,
        }
    }

    /// Check if a process is running (Linux)
    #[cfg(target_os = "linux")]
    fn is_process_running(process_name: &str) -> bool {
        let output = Command::new("pgrep")
            .arg("-f")
            .arg(process_name)
            .output();

        match output {
            Ok(result) => !result.stdout.is_empty(),
            Err(_) => false,
        }
    }

    /// Check for upcoming meetings and detect if meeting should start
    pub async fn check_for_meetings(
        &self,
        calendar: &GoogleCalendar,
    ) -> Result<bool, String> {
        let settings = self.settings.read().await;

        if !settings.enabled {
            return Ok(false);
        }

        // Get upcoming events from calendar
        let events = calendar.get_upcoming_events(Some(10)).await?;

        let now = Utc::now();
        let mut should_auto_start = false;

        // Find the next upcoming meeting
        let next_meeting = events.iter()
            .filter(|e| !e.is_past)
            .min_by_key(|e| &e.start_time);

        if let Some(meeting) = next_meeting {
            // Parse meeting start time
            if let Ok(start_time) = meeting.start_time.parse::<DateTime<Utc>>() {
                let time_until_meeting = start_time - now;
                let minutes_until = time_until_meeting.num_minutes();

                // Check if meeting is starting soon
                let is_starting_soon = minutes_until <= settings.start_buffer_minutes && minutes_until >= -5;

                // Check if this event was already triggered
                let last_triggered = self.last_triggered_event_id.read().await;
                let already_triggered = last_triggered.as_ref() == Some(&meeting.id);

                // Detect meeting apps if enabled
                let meeting_app_detected = if settings.detect_meeting_apps {
                    Self::detect_meeting_apps()
                } else {
                    None
                };

                // Determine if we should auto-start
                should_auto_start = !already_triggered && (
                    (settings.auto_start_on_time && is_starting_soon) ||
                    (settings.detect_meeting_apps && meeting_app_detected.is_some() && minutes_until <= 10)
                );

                // Update status
                let mut status = self.status.write().await;
                status.is_meeting_detected = is_starting_soon || meeting_app_detected.is_some();
                status.meeting_app_running = meeting_app_detected;
                status.upcoming_meeting = Some(meeting.clone());
                status.minutes_until_meeting = Some(minutes_until);
                status.auto_start_triggered = should_auto_start;

                // Mark event as triggered if auto-starting
                if should_auto_start {
                    let mut last_triggered = self.last_triggered_event_id.write().await;
                    *last_triggered = Some(meeting.id.clone());
                }
            }
        } else {
            // No upcoming meetings
            let mut status = self.status.write().await;
            status.is_meeting_detected = false;
            status.meeting_app_running = None;
            status.upcoming_meeting = None;
            status.minutes_until_meeting = None;
            status.auto_start_triggered = false;
        }

        Ok(should_auto_start)
    }

    /// Reset the last triggered event (useful when user manually stops)
    pub async fn reset_trigger(&self) {
        let mut last_triggered = self.last_triggered_event_id.write().await;
        *last_triggered = None;

        let mut status = self.status.write().await;
        status.auto_start_triggered = false;
    }
}
