import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import "./App.css";

interface TranscriptSegment {
  timestamp: string;
  speaker: string;
  text: string;
  is_final?: boolean;
}

interface MeetingSummary {
  key_points: string[];
  action_items: string[];
  decisions: string[];
  notes: string[];
  raw_summary: string;
}

interface CalendarEvent {
  id: string;
  title: string;
  description: string | null;
  start_time: string;
  end_time: string;
  attendees: string[];
  meeting_link: string | null;
  is_today: boolean;
  is_past: boolean;
}

interface MeetingMonitorSettings {
  enabled: boolean;
  start_buffer_minutes: number;
  detect_meeting_apps: boolean;
  auto_start_on_time: boolean;
}

interface MeetingStatus {
  is_meeting_detected: boolean;
  meeting_app_running: string | null;
  upcoming_meeting: CalendarEvent | null;
  minutes_until_meeting: number | null;
  auto_start_triggered: boolean;
}

interface StoredMeeting {
  id: string;
  title: string;
  date: string;
  duration_seconds: number | null;
  transcript: TranscriptSegment[];
  summary: MeetingSummary | null;
  attendees: string[];
  calendar_event_id: string | null;
  recording_path: string | null;
  created_at: string;
  updated_at: string;
}

type ViewMode = 'home' | 'meeting-detail' | 'transcript-view';

function App() {
  // Core state
  const [isLiveTranscribing, setIsLiveTranscribing] = useState(false);
  const [isRecordingOnly] = useState(false);
  const [transcription, setTranscription] = useState<TranscriptSegment[]>([]);
  const [, setSummary] = useState("");
  const [structuredSummary, setStructuredSummary] = useState<MeetingSummary | null>(null);
  const [savedRecordingPath, setSavedRecordingPath] = useState<string | null>(null);
  const [isMockTranscribing, setIsMockTranscribing] = useState(false);

  // API keys state
  const [hasGroqKey, setHasGroqKey] = useState(false);
  const [hasDeepgramKey, setHasDeepgramKey] = useState(false);
  const [groqKeyInput, setGroqKeyInput] = useState("");
  const [deepgramKeyInput, setDeepgramKeyInput] = useState("");
  const [isEditingApiKey, setIsEditingApiKey] = useState(false);
  const [isEditingDeepgramKey, setIsEditingDeepgramKey] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  // Calendar state
  const [isCalendarConnected, setIsCalendarConnected] = useState(false);
  const [upcomingEvents, setUpcomingEvents] = useState<CalendarEvent[]>([]);
  const [googleClientId, setGoogleClientId] = useState("");
  const [googleClientSecret, setGoogleClientSecret] = useState("");
  const [isConnectingCalendar, setIsConnectingCalendar] = useState(false);

  // Meeting monitor state
  const [meetingMonitorSettings, setMeetingMonitorSettings] = useState<MeetingMonitorSettings>({
    enabled: true,
    start_buffer_minutes: 2,
    detect_meeting_apps: true,
    auto_start_on_time: true,
  });
  const [meetingStatus, setMeetingStatus] = useState<MeetingStatus | null>(null);

  // Meetings state
  const [pastMeetings, setPastMeetings] = useState<StoredMeeting[]>([]);
  const [selectedMeeting, setSelectedMeeting] = useState<StoredMeeting | null>(null);

  // UI state
  const [viewMode, setViewMode] = useState<ViewMode>('home');
  const [showSettings, setShowSettings] = useState(false);
  const [showSaveMeetingModal, setShowSaveMeetingModal] = useState(false);
  const [saveMeetingTitle, setSaveMeetingTitle] = useState("");
  const [, setMeetingContext] = useState("");
  const [contextInput, setContextInput] = useState("");
  const [meetingType, setMeetingType] = useState<string>("custom");
  const [autoGenerateReplies, setAutoGenerateReplies] = useState(true);
  const [hideFromScreenShare, setHideFromScreenShare] = useState(false);
  const [screenShareSupported, setScreenShareSupported] = useState(false);
  const [suggestionsMinimized, setSuggestionsMinimized] = useState(false);

  // Recording state
  const [suggestedReplies, setSuggestedReplies] = useState<string[]>([]);
  const [isGeneratingReplies, setIsGeneratingReplies] = useState(false);
  const [isGeneratingSummary, setIsGeneratingSummary] = useState(false);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const [replyError, setReplyError] = useState<string | null>(null);
  const [recordingTime, setRecordingTime] = useState(0);

  const transcriptionEndRef = useRef<HTMLDivElement>(null);
  const lastTranscriptCount = useRef(0);
  const lastReplyGenerationTime = useRef(0);

  // Computed app state
  type AppState = 'ready' | 'recording' | 'done';
  const appState: AppState = (isLiveTranscribing || isRecordingOnly || isMockTranscribing)
    ? 'recording'
    : (transcription.length > 0 ? 'done' : 'ready');

  // Initialize
  useEffect(() => {
    checkApiKeys();
    checkScreenShareSupport();
    checkCalendarConnection();
    loadPastMeetings();
  }, []);

  // Load calendar events when connected
  useEffect(() => {
    if (isCalendarConnected) {
      loadUpcomingEvents();
    }
  }, [isCalendarConnected]);

  // Recording timer
  useEffect(() => {
    let interval: ReturnType<typeof setInterval>;
    if (isLiveTranscribing || isRecordingOnly) {
      interval = setInterval(() => {
        setRecordingTime(t => t + 1);
      }, 1000);
    } else {
      setRecordingTime(0);
    }
    return () => clearInterval(interval);
  }, [isLiveTranscribing, isRecordingOnly]);

  // Transcript updates listener
  useEffect(() => {
    const unlisten = listen<{ text: string; timestamp: string; speaker: string; is_final: boolean }>(
      "transcript-update",
      (event) => {
        if (event.payload.text && event.payload.text.trim()) {
          const newSegment: TranscriptSegment = {
            timestamp: event.payload.timestamp,
            speaker: event.payload.speaker,
            text: event.payload.text,
            is_final: event.payload.is_final,
          };

          setTranscription((prev) => {
            if (event.payload.is_final) {
              const lastIndex = prev.length - 1;
              if (lastIndex >= 0 && prev[lastIndex].is_final === false) {
                return [...prev.slice(0, lastIndex), newSegment];
              }
              return [...prev, newSegment];
            } else {
              const lastIndex = prev.length - 1;
              if (lastIndex >= 0 && prev[lastIndex].is_final === false) {
                return [...prev.slice(0, lastIndex), newSegment];
              }
              return [...prev, newSegment];
            }
          });
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Auto-scroll transcript
  useEffect(() => {
    transcriptionEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [transcription]);

  // Auto-generate replies when enabled
  useEffect(() => {
    if ((isLiveTranscribing || isMockTranscribing) && autoGenerateReplies && transcription.length > 0 && transcription.length !== lastTranscriptCount.current) {
      lastTranscriptCount.current = transcription.length;
      const lastSpeaker = transcription[transcription.length - 1]?.speaker;
      if (lastSpeaker === "You") return;
      const now = Date.now();
      const timeSinceLastGeneration = now - lastReplyGenerationTime.current;
      if (timeSinceLastGeneration >= 5000 || lastReplyGenerationTime.current === 0) {
        lastReplyGenerationTime.current = now;
        generateRepliesQuietly();
      }
    }
  }, [transcription, isLiveTranscribing, isMockTranscribing, autoGenerateReplies]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

      const keyNum = parseInt(e.key);
      if (keyNum >= 1 && keyNum <= 4 && suggestedReplies.length >= keyNum) {
        handleCopyReply(suggestedReplies[keyNum - 1], keyNum - 1);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [suggestedReplies]);

  // Load meeting monitor settings on mount
  useEffect(() => {
    const loadMeetingMonitorSettings = async () => {
      try {
        const settings = await invoke<MeetingMonitorSettings>("get_meeting_monitor_settings");
        setMeetingMonitorSettings(settings);
      } catch (error) {
        console.error("Failed to load meeting monitor settings:", error);
      }
    };
    loadMeetingMonitorSettings();
  }, []);

  // Listen for meeting auto-start event
  useEffect(() => {
    const unlisten = listen("meeting-auto-start", () => {
      console.log("Meeting auto-start triggered!");
      // Auto-start live transcription
      if (!isLiveTranscribing && hasDeepgramKey) {
        handleStartLiveTranscription();
      }
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, [isLiveTranscribing, hasDeepgramKey]);

  // Poll meeting status periodically when calendar is connected
  useEffect(() => {
    if (!isCalendarConnected || !meetingMonitorSettings.enabled) {
      return;
    }

    const pollMeetingStatus = async () => {
      try {
        const status = await invoke<MeetingStatus>("get_meeting_status");
        setMeetingStatus(status);
      } catch (error) {
        console.error("Failed to get meeting status:", error);
      }
    };

    // Poll immediately and then every 10 seconds
    pollMeetingStatus();
    const interval = setInterval(pollMeetingStatus, 10000);

    return () => clearInterval(interval);
  }, [isCalendarConnected, meetingMonitorSettings.enabled]);

  // API Functions
  const checkApiKeys = async () => {
    try {
      const state = await invoke<{
        has_groq_key: boolean;
        has_deepgram_key: boolean;
        meeting_context: string;
      }>("get_meeting_state");
      setHasGroqKey(state.has_groq_key);
      setHasDeepgramKey(state.has_deepgram_key);
      if (state.meeting_context) {
        setMeetingContext(state.meeting_context);
        setContextInput(state.meeting_context);
      }
    } catch (error) {
      setHasGroqKey(false);
      setHasDeepgramKey(false);
    }
  };

  const checkScreenShareSupport = async () => {
    try {
      const supported = await invoke<boolean>("is_screen_share_exclusion_supported");
      setScreenShareSupported(supported);
    } catch (error) {
      console.error("Failed to check screen share support:", error);
    }
  };

  const checkCalendarConnection = async () => {
    try {
      const connected = await invoke<boolean>("is_calendar_connected");
      setIsCalendarConnected(connected);
    } catch (error) {
      console.error("Failed to check calendar connection:", error);
    }
  };

  const loadUpcomingEvents = async () => {
    try {
      const events = await invoke<CalendarEvent[]>("get_upcoming_events", { limit: 10 });
      setUpcomingEvents(events);
    } catch (error) {
      console.error("Failed to load upcoming events:", error);
    }
  };

  const loadPastMeetings = async () => {
    try {
      const meetings = await invoke<StoredMeeting[]>("get_saved_meetings", { limit: 20 });
      setPastMeetings(meetings);
    } catch (error) {
      console.error("Failed to load past meetings:", error);
    }
  };

  const handleSaveGroqKey = async () => {
    if (!groqKeyInput.trim()) return;
    setIsLoading(true);
    try {
      const isValid = await invoke<boolean>("set_groq_api_key", { key: groqKeyInput.trim() });
      if (isValid) {
        setHasGroqKey(true);
        setGroqKeyInput("");
        setIsEditingApiKey(false);
      }
    } catch (error) {
      console.error("Failed to save Groq API key:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSaveDeepgramKey = async () => {
    if (!deepgramKeyInput.trim()) return;
    setIsLoading(true);
    try {
      const isValid = await invoke<boolean>("set_deepgram_api_key", { key: deepgramKeyInput.trim() });
      if (isValid) {
        setHasDeepgramKey(true);
        setDeepgramKeyInput("");
        setIsEditingDeepgramKey(false);
      }
    } catch (error) {
      console.error("Failed to save Deepgram API key:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleConnectCalendar = async () => {
    if (!googleClientId.trim() || !googleClientSecret.trim()) {
      alert("Please enter Google Client ID and Secret");
      return;
    }

    setIsConnectingCalendar(true);
    try {
      await invoke("set_google_credentials", {
        clientId: googleClientId.trim(),
        clientSecret: googleClientSecret.trim(),
      });

      const authUrl = await invoke<string>("get_google_auth_url");
      await openUrl(authUrl);

      // Show instructions
      alert("A browser window will open. After authorizing, copy the code from the URL and paste it here.");
      const code = prompt("Paste the authorization code:");

      if (code) {
        await invoke("exchange_google_code", { code: code.trim() });
        setIsCalendarConnected(true);
        loadUpcomingEvents();
      }
    } catch (error) {
      console.error("Failed to connect calendar:", error);
      alert("Failed to connect calendar: " + error);
    } finally {
      setIsConnectingCalendar(false);
    }
  };

  const handleDisconnectCalendar = async () => {
    try {
      await invoke("disconnect_calendar");
      setIsCalendarConnected(false);
      setUpcomingEvents([]);
    } catch (error) {
      console.error("Failed to disconnect calendar:", error);
    }
  };

  const handleUpdateMeetingMonitorSettings = async (settings: MeetingMonitorSettings) => {
    try {
      await invoke("update_meeting_monitor_settings", { settings });
      setMeetingMonitorSettings(settings);
    } catch (error) {
      console.error("Failed to update meeting monitor settings:", error);
    }
  };

  const handleSaveContext = async () => {
    try {
      await invoke("set_meeting_context", { context: contextInput.trim() });
      setMeetingContext(contextInput.trim());
    } catch (error) {
      console.error("Failed to save meeting context:", error);
    }
  };

  const handleToggleScreenShare = async (enabled: boolean) => {
    try {
      await invoke("set_screen_share_exclusion", { exclude: enabled });
      setHideFromScreenShare(enabled);
    } catch (error) {
      console.error("Failed to toggle screen share exclusion:", error);
    }
  };

  // Recording functions
  const handleStartLiveTranscription = async () => {
    if (!hasGroqKey) return;
    try {
      await invoke("start_live_transcription");
      setIsLiveTranscribing(true);
      setSuggestedReplies([]);
      lastTranscriptCount.current = 0;
      lastReplyGenerationTime.current = 0;
    } catch (error) {
      console.error("Failed to start live transcription:", error);
      alert("Failed to start: " + error);
    }
  };

  const handleStopLiveTranscription = async () => {
    try {
      const audioPath = await invoke<string>("stop_live_transcription");
      setIsLiveTranscribing(false);
      if (audioPath) setSavedRecordingPath(audioPath);

      if (hasGroqKey && transcription.length > 0) {
        setIsGeneratingSummary(true);
        try {
          const summaryResult = await invoke<MeetingSummary>("generate_structured_summary");
          setStructuredSummary(summaryResult);
          if (summaryResult.raw_summary) setSummary(summaryResult.raw_summary);
        } catch (summaryError) {
          console.error("Failed to generate summary:", summaryError);
        } finally {
          setIsGeneratingSummary(false);
        }
      }
    } catch (error) {
      console.error("Failed to stop live transcription:", error);
    }
  };

  const handleMockTranscription = async () => {
    if (isMockTranscribing) {
      try {
        await invoke('stop_mock_transcription');
        setIsMockTranscribing(false);
      } catch (err) {
        console.error('Failed to stop mock:', err);
      }
    } else {
      try {
        await invoke('start_mock_transcription', {
          testAudioDir: '/Users/vigneshsubbiah/Documents/meetBetter/src-tauri/test_audio'
        });
        setIsMockTranscribing(true);
      } catch (err) {
        console.error('Failed to start mock:', err);
        alert('Mock transcription failed: ' + err);
      }
    }
  };

  const generateRepliesQuietly = async () => {
    if (isGeneratingReplies || !hasGroqKey) return;
    setIsGeneratingReplies(true);
    setReplyError(null);
    try {
      const replies = await invoke<string[]>("generate_auto_replies");
      setSuggestedReplies(replies);
    } catch (error) {
      console.error("Failed to generate replies:", error);
      const errorMsg = String(error);
      if (errorMsg.includes("rate") || errorMsg.includes("429")) {
        setReplyError("Rate limited - waiting before retrying");
      } else {
        setReplyError(errorMsg.substring(0, 100));
      }
    } finally {
      setIsGeneratingReplies(false);
    }
  };

  const handleGenerateSummary = async () => {
    if (transcription.length === 0 || !hasGroqKey) return;
    setIsLoading(true);
    setIsGeneratingSummary(true);
    try {
      const summaryResult = await invoke<MeetingSummary>("generate_structured_summary");
      setStructuredSummary(summaryResult);
      if (summaryResult.raw_summary) setSummary(summaryResult.raw_summary);
    } catch (error) {
      console.error("Failed to generate summary:", error);
    } finally {
      setIsLoading(false);
      setIsGeneratingSummary(false);
    }
  };

  const handleCopyReply = async (reply: string, index: number) => {
    try {
      await navigator.clipboard.writeText(reply);
      setCopiedIndex(index);
      setTimeout(() => setCopiedIndex(null), 2000);
    } catch (error) {
      console.error("Failed to copy:", error);
    }
  };

  const handleClearAll = async () => {
    try {
      await invoke("clear_transcription");
      setTranscription([]);
      setSummary("");
      setStructuredSummary(null);
      setSuggestedReplies([]);
      setSavedRecordingPath(null);
      setViewMode('home');
    } catch (error) {
      console.error("Failed to clear:", error);
    }
  };

  const handleSaveMeeting = async (title: string) => {
    if (transcription.length === 0) {
      alert("No transcript to save. Please record a meeting first.");
      return;
    }

    try {
      console.log("Saving meeting with title:", title, "transcript segments:", transcription.length);
      console.log("Transcript data:", JSON.stringify(transcription.slice(0, 2)));

      // Strip is_final field from transcript before sending to backend
      const cleanTranscript = transcription.map(seg => ({
        timestamp: seg.timestamp,
        speaker: seg.speaker,
        text: seg.text,
      }));

      // Pass transcript and summary from frontend state
      // Note: Tauri expects snake_case parameter names
      const meetingId = await invoke<string>("save_meeting", {
        title,
        attendees: [],
        calendar_event_id: null,
        duration_seconds: recordingTime > 0 ? recordingTime : null,
        transcript: cleanTranscript,
        summary: structuredSummary,
      });
      console.log("Meeting saved with ID:", meetingId);

      // Clear current state and go back to home
      await invoke("clear_transcription");
      setTranscription([]);
      setSummary("");
      setStructuredSummary(null);
      setSuggestedReplies([]);
      setSavedRecordingPath(null);

      // Reload meetings list
      await loadPastMeetings();

      // Go to home view to see saved meetings
      // Clear the current meeting
      await invoke("clear_transcription");
      setTranscription([]);
      setSummary("");
      setStructuredSummary(null);
      setSuggestedReplies([]);
      setSavedRecordingPath(null);

      // Reload past meetings to show the newly saved one
      await loadPastMeetings();

      // Go back to home view
      setViewMode('home');

      alert("Meeting saved successfully! You can find it in Past Meetings.");
    } catch (error) {
      console.error("Failed to save meeting:", error);
      alert("Failed to save meeting: " + error);
    }
  };

  const handleViewMeeting = (meeting: StoredMeeting) => {
    setSelectedMeeting(meeting);
    setViewMode('meeting-detail');
  };

  const handleDeleteMeeting = async (id: string) => {
    if (!confirm("Are you sure you want to delete this meeting?")) return;
    try {
      await invoke("delete_meeting", { id });
      loadPastMeetings();
      if (selectedMeeting?.id === id) {
        setSelectedMeeting(null);
        setViewMode('home');
      }
    } catch (error) {
      console.error("Failed to delete meeting:", error);
    }
  };

  // Meeting type presets
  const meetingPresets: Record<string, { label: string; context: string }> = {
    interview: { label: "Interview", context: "Job interview. I'm the candidate." },
    sales: { label: "Sales Call", context: "Sales call with a potential client." },
    team: { label: "Team Meeting", context: "Internal team meeting." },
    "1on1": { label: "1:1", context: "One-on-one meeting." },
    custom: { label: "Custom", context: "" }
  };

  const handleMeetingTypeChange = (type: string) => {
    setMeetingType(type);
    const preset = meetingPresets[type];
    if (preset && preset.context) {
      setContextInput(preset.context);
      invoke("set_meeting_context", { context: preset.context }).then(() => {
        setMeetingContext(preset.context);
      });
    }
  };

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  };

  return (
    <div className={`app-minimal ${appState}`}>
      {/* Save Meeting Modal */}
      {showSaveMeetingModal && (
        <div className="modal-overlay" onClick={() => setShowSaveMeetingModal(false)}>
          <div className="settings-modal save-meeting-modal" onClick={e => e.stopPropagation()}>
            <div className="modal-header">
              <h2>Save Meeting</h2>
              <button className="close-btn" onClick={() => setShowSaveMeetingModal(false)}>×</button>
            </div>
            <div className="modal-content">
              <div className="setting-item">
                <label>Meeting Title</label>
                <input
                  type="text"
                  value={saveMeetingTitle}
                  onChange={(e) => setSaveMeetingTitle(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && saveMeetingTitle.trim()) {
                      handleSaveMeeting(saveMeetingTitle.trim());
                      setShowSaveMeetingModal(false);
                    }
                  }}
                  placeholder="Enter meeting title"
                  autoFocus
                />
              </div>
              <div className="modal-actions">
                <button className="text-btn" onClick={() => setShowSaveMeetingModal(false)}>Cancel</button>
                <button
                  className="primary-btn"
                  onClick={() => {
                    if (saveMeetingTitle.trim()) {
                      handleSaveMeeting(saveMeetingTitle.trim());
                      setShowSaveMeetingModal(false);
                    }
                  }}
                  disabled={!saveMeetingTitle.trim()}
                >
                  Save
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Settings Modal */}
      {showSettings && (
        <div className="modal-overlay" onClick={() => setShowSettings(false)}>
          <div className="settings-modal" onClick={e => e.stopPropagation()}>
            <div className="modal-header">
              <h2>Settings</h2>
              <button className="close-btn" onClick={() => setShowSettings(false)}>×</button>
            </div>

            <div className="modal-content">
              {/* Groq API Key */}
              <div className="setting-item">
                <label>Groq API Key</label>
                <p className="setting-hint">Powers AI suggestions and batch transcription</p>
                {hasGroqKey && !isEditingApiKey ? (
                  <div className="api-status">
                    <span className="status-dot connected"></span>
                    <span>Connected</span>
                    <button className="text-btn" onClick={() => setIsEditingApiKey(true)}>Change</button>
                  </div>
                ) : (
                  <div className="api-input-row">
                    <input
                      type="password"
                      placeholder="Enter Groq API key"
                      value={groqKeyInput}
                      onChange={(e) => setGroqKeyInput(e.target.value)}
                      onKeyDown={(e) => e.key === 'Enter' && handleSaveGroqKey()}
                    />
                    <button onClick={handleSaveGroqKey} disabled={isLoading || !groqKeyInput.trim()}>Save</button>
                  </div>
                )}
                <span className="help-link" onClick={() => openUrl("https://console.groq.com/keys")}>
                  Get free Groq key →
                </span>
              </div>

              {/* Deepgram API Key */}
              <div className="setting-item">
                <label>Deepgram API Key</label>
                <p className="setting-hint">Enables real-time streaming transcription</p>
                {hasDeepgramKey && !isEditingDeepgramKey ? (
                  <div className="api-status">
                    <span className="status-dot connected"></span>
                    <span>Connected</span>
                    <button className="text-btn" onClick={() => setIsEditingDeepgramKey(true)}>Change</button>
                  </div>
                ) : (
                  <div className="api-input-row">
                    <input
                      type="password"
                      placeholder="Enter Deepgram API key"
                      value={deepgramKeyInput}
                      onChange={(e) => setDeepgramKeyInput(e.target.value)}
                      onKeyDown={(e) => e.key === 'Enter' && handleSaveDeepgramKey()}
                    />
                    <button onClick={handleSaveDeepgramKey} disabled={isLoading || !deepgramKeyInput.trim()}>Save</button>
                  </div>
                )}
                <span className="help-link" onClick={() => openUrl("https://console.deepgram.com/")}>
                  Get Deepgram key →
                </span>
              </div>

              {/* Google Calendar */}
              <div className="setting-item">
                <label>Google Calendar</label>
                <p className="setting-hint">Connect to see your upcoming meetings</p>
                {isCalendarConnected ? (
                  <div className="api-status">
                    <span className="status-dot connected"></span>
                    <span>Connected</span>
                    <button className="text-btn danger" onClick={handleDisconnectCalendar}>Disconnect</button>
                  </div>
                ) : (
                  <div className="calendar-setup">
                    <input
                      type="text"
                      placeholder="Google Client ID"
                      value={googleClientId}
                      onChange={(e) => setGoogleClientId(e.target.value)}
                    />
                    <input
                      type="password"
                      placeholder="Google Client Secret"
                      value={googleClientSecret}
                      onChange={(e) => setGoogleClientSecret(e.target.value)}
                    />
                    <button
                      onClick={handleConnectCalendar}
                      disabled={isConnectingCalendar || !googleClientId.trim() || !googleClientSecret.trim()}
                    >
                      {isConnectingCalendar ? "Connecting..." : "Connect Calendar"}
                    </button>
                    <span className="help-link" onClick={() => openUrl("https://console.cloud.google.com/apis/credentials")}>
                      Get Google credentials (free) →
                    </span>
                  </div>
                )}
              </div>

              {/* Auto-start Meetings */}
              {isCalendarConnected && (
                <div className="setting-item">
                  <label>Auto-start transcription</label>
                  <p className="setting-hint">Automatically start when meetings begin (like Granola)</p>
                  <div className="toggle">
                    <input
                      type="checkbox"
                      checked={meetingMonitorSettings.enabled}
                      onChange={(e) => handleUpdateMeetingMonitorSettings({
                        ...meetingMonitorSettings,
                        enabled: e.target.checked
                      })}
                    />
                    <span className="toggle-track"></span>
                  </div>

                  {meetingMonitorSettings.enabled && (
                    <div className="auto-start-options">
                      <div className="option-row">
                        <label>
                          <input
                            type="checkbox"
                            checked={meetingMonitorSettings.auto_start_on_time}
                            onChange={(e) => handleUpdateMeetingMonitorSettings({
                              ...meetingMonitorSettings,
                              auto_start_on_time: e.target.checked
                            })}
                          />
                          <span>Start at meeting time</span>
                        </label>
                      </div>
                      <div className="option-row">
                        <label>
                          <input
                            type="checkbox"
                            checked={meetingMonitorSettings.detect_meeting_apps}
                            onChange={(e) => handleUpdateMeetingMonitorSettings({
                              ...meetingMonitorSettings,
                              detect_meeting_apps: e.target.checked
                            })}
                          />
                          <span>Detect Zoom/Teams/Meet</span>
                        </label>
                      </div>
                      <div className="option-row">
                        <label>Buffer time:</label>
                        <input
                          type="number"
                          min="0"
                          max="15"
                          value={meetingMonitorSettings.start_buffer_minutes}
                          onChange={(e) => handleUpdateMeetingMonitorSettings({
                            ...meetingMonitorSettings,
                            start_buffer_minutes: parseInt(e.target.value) || 2
                          })}
                          style={{ width: '60px', marginLeft: '8px' }}
                        />
                        <span style={{ marginLeft: '8px' }}>minutes</span>
                      </div>
                    </div>
                  )}
                </div>
              )}

              {/* Auto-suggest */}
              <div className="setting-item">
                <label>Auto-suggest replies</label>
                <div className="toggle">
                  <input
                    type="checkbox"
                    checked={autoGenerateReplies}
                    onChange={(e) => setAutoGenerateReplies(e.target.checked)}
                  />
                  <span className="toggle-track"></span>
                </div>
              </div>

              {/* Privacy */}
              {screenShareSupported && (
                <div className="setting-item">
                  <label>Hide from screen share</label>
                  <div className="toggle">
                    <input
                      type="checkbox"
                      checked={hideFromScreenShare}
                      onChange={(e) => handleToggleScreenShare(e.target.checked)}
                    />
                    <span className="toggle-track"></span>
                  </div>
                </div>
              )}

              {/* Dev Mode */}
              {import.meta.env.DEV && (
                <div className="setting-item">
                  <label>Mock Test</label>
                  <button
                    className={`text-btn dev-link ${isMockTranscribing ? 'active' : ''}`}
                    onClick={() => { handleMockTranscription(); setShowSettings(false); }}
                  >
                    {isMockTranscribing ? "Stop" : "Run"}
                  </button>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* READY STATE - Home with sidebar */}
      {appState === 'ready' && viewMode === 'home' && (
        <div className="home-layout">
          {/* Sidebar */}
          <aside className="sidebar">
            <div className="sidebar-header">
              <span className="logo">Vantage</span>
              <button className="icon-btn" onClick={() => setShowSettings(true)} title="Settings">⚙️</button>
            </div>

            {/* Coming Up Section */}
            <div className="sidebar-section">
              <h3>Coming up</h3>

              {/* Meeting Status Banner */}
              {meetingMonitorSettings.enabled && meetingStatus?.is_meeting_detected && (
                <div className="meeting-status-banner">
                  {meetingStatus.meeting_app_running && (
                    <div className="status-row">
                      <span className="status-dot active"></span>
                      <span className="status-text">{meetingStatus.meeting_app_running} detected</span>
                    </div>
                  )}
                  {meetingStatus.upcoming_meeting && meetingStatus.minutes_until_meeting !== null && (
                    <div className="status-row">
                      <span>Meeting {meetingStatus.minutes_until_meeting <= 0 ? 'started' : `in ${meetingStatus.minutes_until_meeting} min`}</span>
                    </div>
                  )}
                  {meetingStatus.auto_start_triggered && !isLiveTranscribing && (
                    <div className="status-row warning">
                      <span>Ready to auto-start!</span>
                    </div>
                  )}
                </div>
              )}

              {isCalendarConnected ? (
                upcomingEvents.length > 0 ? (
                  <div className="event-list">
                    {upcomingEvents.slice(0, 5).map((event) => (
                      <div key={event.id} className={`event-item ${event.is_today ? 'today' : ''}`}>
                        <div className="event-date">
                          <span className="month">{new Date(event.start_time).toLocaleDateString('en-US', { month: 'short' }).toUpperCase()}</span>
                          <span className="day">{new Date(event.start_time).getDate()}</span>
                        </div>
                        <div className="event-info">
                          <span className="event-title">{event.title}</span>
                          <span className="event-time">{formatDate(event.start_time)}</span>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="empty-state">No upcoming events</p>
                )
              ) : (
                <p className="empty-state">
                  <button className="text-btn" onClick={() => setShowSettings(true)}>Connect Google Calendar</button>
                </p>
              )}
            </div>

            {/* Past Meetings Section */}
            <div className="sidebar-section">
              <h3>Past Meetings</h3>
              {pastMeetings.length > 0 ? (
                <div className="meeting-list">
                  {pastMeetings.slice(0, 10).map((meeting) => (
                    <div
                      key={meeting.id}
                      className="meeting-item"
                      onClick={() => handleViewMeeting(meeting)}
                    >
                      <div className="meeting-icon">📄</div>
                      <div className="meeting-info">
                        <span className="meeting-title">{meeting.title}</span>
                        <span className="meeting-date">{formatDate(meeting.date)}</span>
                      </div>
                    </div>
                  ))}
                  {pastMeetings.length > 10 && (
                    <p className="meeting-list-more">+{pastMeetings.length - 10} more meetings</p>
                  )}
                </div>
              ) : (
                <div className="meeting-list-empty">
                  <p>No saved meetings yet</p>
                  <p style={{marginTop: '8px', fontSize: '12px'}}>
                    Start a meeting and click "Save Meeting" to see it here
                  </p>
                </div>
              )}
            </div>
          </aside>

          {/* Main Content */}
          <main className="main-content">
            {!(hasGroqKey || hasDeepgramKey) ? (
              <div className="setup-prompt">
                <h1>Welcome to Vantage</h1>
                <p>Add your API keys to get started</p>
                <button className="primary-btn large" onClick={() => setShowSettings(true)}>
                  Open Settings
                </button>
              </div>
            ) : (
              <div className="start-section">
                <h1>What kind of meeting?</h1>
                <div className="meeting-types">
                  {Object.entries(meetingPresets).map(([key, preset]) => (
                    <button
                      key={key}
                      className={`type-btn ${meetingType === key ? 'active' : ''}`}
                      onClick={() => handleMeetingTypeChange(key)}
                    >
                      {preset.label}
                    </button>
                  ))}
                </div>

                {meetingType === 'custom' && (
                  <textarea
                    className="context-textarea"
                    placeholder="Describe the meeting context..."
                    value={contextInput}
                    onChange={(e) => setContextInput(e.target.value)}
                    onBlur={handleSaveContext}
                    rows={2}
                  />
                )}

                <button
                  className="primary-btn large start-btn"
                  onClick={handleStartLiveTranscription}
                >
                  Start Meeting
                </button>
              </div>
            )}
          </main>
        </div>
      )}

      {/* Meeting Detail View */}
      {appState === 'ready' && viewMode === 'meeting-detail' && selectedMeeting && (
        <div className="meeting-detail-view">
          <header className="detail-header">
            <button className="back-btn" onClick={() => { setViewMode('home'); setSelectedMeeting(null); }}>
              ← Back
            </button>
            <h2>{selectedMeeting.title}</h2>
            <button className="icon-btn danger" onClick={() => handleDeleteMeeting(selectedMeeting.id)} title="Delete">
              🗑️
            </button>
          </header>

          <div className="detail-content">
            <div className="detail-meta">
              <span>{formatDate(selectedMeeting.date)}</span>
              {selectedMeeting.duration_seconds && (
                <span>Duration: {formatTime(selectedMeeting.duration_seconds)}</span>
              )}
              {selectedMeeting.attendees.length > 0 && (
                <span>Attendees: {selectedMeeting.attendees.join(', ')}</span>
              )}
            </div>

            {/* Summary */}
            {selectedMeeting.summary && (
              <section className="detail-section">
                <h3>Summary</h3>
                {selectedMeeting.summary.key_points?.length > 0 && (
                  <div className="summary-group">
                    <h4>Key Points</h4>
                    <ul>{selectedMeeting.summary.key_points.map((p, i) => <li key={i}>{p}</li>)}</ul>
                  </div>
                )}
                {selectedMeeting.summary.action_items?.length > 0 && (
                  <div className="summary-group">
                    <h4>Action Items</h4>
                    <ul>{selectedMeeting.summary.action_items.map((p, i) => <li key={i}>{p}</li>)}</ul>
                  </div>
                )}
                {selectedMeeting.summary.decisions?.length > 0 && (
                  <div className="summary-group">
                    <h4>Decisions</h4>
                    <ul>{selectedMeeting.summary.decisions.map((p, i) => <li key={i}>{p}</li>)}</ul>
                  </div>
                )}
              </section>
            )}

            {/* Transcript */}
            <section className="detail-section">
              <h3>Transcript ({selectedMeeting.transcript.length} segments)</h3>
              <div className="transcript-list compact">
                {selectedMeeting.transcript.map((seg, i) => (
                  <div key={i} className={`transcript-item ${seg.speaker === 'You' ? 'you' : 'participant'}`}>
                    <span className="speaker">{seg.speaker}:</span>
                    <span className="text">{seg.text}</span>
                  </div>
                ))}
              </div>
            </section>
          </div>
        </div>
      )}

      {/* RECORDING STATE */}
      {appState === 'recording' && (
        <div className="recording-screen">
          <header className="recording-header">
            <div className="recording-status">
              <span className="rec-dot"></span>
              <span className="rec-time">{formatTime(recordingTime)}</span>
              <div className="audio-waveform">
                <span className="bar"></span>
                <span className="bar"></span>
                <span className="bar"></span>
                <span className="bar"></span>
              </div>
            </div>
            <button className="stop-btn" onClick={() => {
              if (isLiveTranscribing) handleStopLiveTranscription();
              else if (isMockTranscribing) handleMockTranscription();
            }}>
              Stop
            </button>
          </header>

          <main className="transcript-main">
            {transcription.length === 0 ? (
              <div className="empty-transcript"><p>Listening...</p></div>
            ) : (
              <div className="transcript-list chat-style">
                {transcription.map((seg, i) => (
                  <div key={i} className={`transcript-item ${seg.speaker === 'You' ? 'you' : 'participant'} ${seg.is_final === false ? 'interim' : ''}`}>
                    <div className="message-bubble">
                      <span className="speaker-label">{seg.speaker}</span>
                      <p>{seg.text}</p>
                      {seg.is_final === false && <span className="interim-badge">...</span>}
                    </div>
                  </div>
                ))}
                <div ref={transcriptionEndRef} />
              </div>
            )}
          </main>

          {/* Collapsible Suggestions Panel */}
          {autoGenerateReplies && (
            <div className={`suggestions-panel ${suggestionsMinimized ? 'minimized' : ''}`}>
              <div className="suggestions-header">
                <span className="suggestions-title">
                  {isGeneratingReplies ? 'Generating...' : 'Suggested Replies'}
                </span>
                <button
                  className="minimize-btn"
                  onClick={() => setSuggestionsMinimized(!suggestionsMinimized)}
                  title={suggestionsMinimized ? 'Expand' : 'Minimize'}
                >
                  {suggestionsMinimized ? '↑' : '−'}
                </button>
              </div>
              {!suggestionsMinimized && (
                <div className="suggestions-content">
                  {suggestedReplies.length > 0 ? (
                    <div className="reply-list">
                      {suggestedReplies.map((reply, i) => (
                        <div
                          key={i}
                          className={`reply-item ${copiedIndex === i ? 'copied' : ''}`}
                          onClick={() => handleCopyReply(reply, i)}
                        >
                          <span className="reply-key">{i + 1}</span>
                          <span className="reply-text">{reply}</span>
                          <span className="copy-hint">{copiedIndex === i ? 'Copied!' : 'Click to copy'}</span>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="empty-suggestions">
                      {isGeneratingReplies ? (
                        <p>Analyzing conversation...</p>
                      ) : replyError ? (
                        <p className="error">{replyError}</p>
                      ) : (
                        <p>Suggestions will appear as the conversation progresses</p>
                      )}
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* DONE STATE */}
      {appState === 'done' && (
        <div className="done-screen">
          <header className="minimal-header">
            <span className="logo">Vantage</span>
            <div className="header-actions">
              <button className="text-btn" onClick={() => {
                setViewMode('home');
                loadPastMeetings();
              }}>← Back to Home</button>
              <button className="text-btn" onClick={() => {
                setSaveMeetingTitle("Meeting " + new Date().toLocaleDateString());
                setShowSaveMeetingModal(true);
              }}>Save Meeting</button>
              <button className="text-btn" onClick={handleClearAll}>New Meeting</button>
              <button className="icon-btn" onClick={() => setShowSettings(true)} title="Settings">⚙️</button>
            </div>
          </header>

          <main className="done-content">
            {/* Summary Section */}
            <section className="summary-section-main">
              <div className="section-title">
                <h2>Meeting Summary</h2>
                <button className="generate-btn" onClick={handleGenerateSummary} disabled={isGeneratingSummary}>
                  {isGeneratingSummary ? "Generating..." : structuredSummary ? "Refresh" : "Generate"}
                </button>
              </div>

              {isGeneratingSummary ? (
                <div className="generating-state">Analyzing your meeting...</div>
              ) : structuredSummary ? (
                <div className="summary-content">
                  {structuredSummary.key_points?.length > 0 && (
                    <div className="summary-group">
                      <h3>Key Points</h3>
                      <ul>{structuredSummary.key_points.map((p, i) => <li key={i}>{p}</li>)}</ul>
                    </div>
                  )}
                  {structuredSummary.action_items?.length > 0 && (
                    <div className="summary-group actions">
                      <h3>Action Items</h3>
                      <ul>{structuredSummary.action_items.map((p, i) => <li key={i}>{p}</li>)}</ul>
                    </div>
                  )}
                  {structuredSummary.decisions?.length > 0 && (
                    <div className="summary-group">
                      <h3>Decisions</h3>
                      <ul>{structuredSummary.decisions.map((p, i) => <li key={i}>{p}</li>)}</ul>
                    </div>
                  )}
                </div>
              ) : (
                <div className="empty-summary"><p>Click "Generate" to create a meeting summary</p></div>
              )}
            </section>

            {/* Transcript Section */}
            <section className="transcript-section">
              <details>
                <summary><h3>Full Transcript ({transcription.length} segments)</h3></summary>
                <div className="transcript-list compact">
                  {transcription.map((seg, i) => (
                    <div key={i} className={`transcript-item ${seg.speaker === 'You' ? 'you' : 'participant'}`}>
                      <span className="speaker">{seg.speaker}:</span>
                      <span className="text">{seg.text}</span>
                    </div>
                  ))}
                </div>
              </details>
            </section>

            {savedRecordingPath && (
              <section className="recording-section">
                <p className="recording-path">Recording saved: {savedRecordingPath.split('/').pop()}</p>
              </section>
            )}
          </main>
        </div>
      )}
    </div>
  );
}

export default App;
