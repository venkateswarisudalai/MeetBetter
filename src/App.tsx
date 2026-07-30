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
  user_notes: string | null;
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
  const [hasProxy, setHasProxy] = useState(false);
  const [groqKeyInput, setGroqKeyInput] = useState("");
  const [deepgramKeyInput, setDeepgramKeyInput] = useState("");
  const [isEditingApiKey, setIsEditingApiKey] = useState(false);
  const [isEditingDeepgramKey, setIsEditingDeepgramKey] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  // Calendar state
  const [isCalendarConnected, setIsCalendarConnected] = useState(false);
  const [upcomingEvents, setUpcomingEvents] = useState<CalendarEvent[]>([]);
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

  // Calendar event tracking
  const [currentCalendarEventId, setCurrentCalendarEventId] = useState<string | null>(null);
  const [isShareToCalendarLoading, setIsShareToCalendarLoading] = useState(false);

  // Audio diagnostics state
  const [audioMode, setAudioMode] = useState<"multichannel" | "diarize" | null>(null);
  const [captureMethod, setCaptureMethod] = useState<string | null>(null);
  const [systemAudioSilent, setSystemAudioSilent] = useState(false);
  const [silenceMessage, setSilenceMessage] = useState<string | null>(null);
  const [micPermission, setMicPermission] = useState<"granted" | "denied" | "checking">("checking");

  // Recording state
  const [suggestedReplies, setSuggestedReplies] = useState<string[]>([]);
  const [isGeneratingReplies, setIsGeneratingReplies] = useState(false);
  const [isGeneratingSummary, setIsGeneratingSummary] = useState(false);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const [replyError, setReplyError] = useState<string | null>(null);
  const [recordingTime, setRecordingTime] = useState(0);

  // Cloud sync state
  const [cloudSyncEnabled, setCloudSyncEnabled] = useState(false);
  const [isSyncing, setIsSyncing] = useState(false);

  // Notepad state (Granola-style)
  const [userNotes, setUserNotes] = useState("");
  const [showTranscriptPanel, setShowTranscriptPanel] = useState(true);
  const [enhancedNotes, setEnhancedNotes] = useState<string | null>(null);
  const [isEnhancing, setIsEnhancing] = useState(false);
  const [askAiQuestion, setAskAiQuestion] = useState("");
  const [askAiAnswer, setAskAiAnswer] = useState("");
  const [isAskingAi, setIsAskingAi] = useState(false);
  const notepadRef = useRef<HTMLDivElement>(null);

  const transcriptionEndRef = useRef<HTMLDivElement>(null);
  const lastTranscriptCount = useRef(0);
  const lastReplyGenerationTime = useRef(0);
  const userScrolledUp = useRef(false);
  const transcriptContainerRef = useRef<HTMLDivElement>(null);

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
    checkCloudSyncStatus();
    // Check microphone permission early
    invoke<string>("check_microphone_permission").then((status) => {
      setMicPermission(status === "granted" ? "granted" : "denied");
    }).catch(() => setMicPermission("granted")); // assume granted if check fails
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
            // Find the last interim for THIS speaker (not just the last item)
            const speaker = event.payload.speaker;
            let lastInterimIndex = -1;
            for (let i = prev.length - 1; i >= 0; i--) {
              if (prev[i].speaker === speaker && prev[i].is_final === false) {
                lastInterimIndex = i;
                break;
              }
              // Stop searching if we hit a final from the same speaker
              if (prev[i].speaker === speaker && prev[i].is_final !== false) {
                break;
              }
            }

            if (event.payload.is_final) {
              // Replace the interim for this speaker with the final
              if (lastInterimIndex >= 0) {
                const updated = [...prev];
                updated[lastInterimIndex] = newSegment;
                return updated;
              }
              return [...prev, newSegment];
            } else {
              // Replace existing interim for this speaker, or append new one
              if (lastInterimIndex >= 0) {
                const updated = [...prev];
                updated[lastInterimIndex] = newSegment;
                return updated;
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

  // Listen for transcript-remove events (retroactive speaker correction)
  useEffect(() => {
    const unlisten = listen<{ speaker: string; text: string }>(
      "transcript-remove",
      (event) => {
        const { speaker, text } = event.payload;
        setTranscription((prev) =>
          prev.filter((seg) => !(seg.speaker === speaker && seg.text === text))
        );
      }
    );
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // Listen for audio-mode event
  useEffect(() => {
    const unlisten = listen<{ mode: string; system_audio_device: string; channels: number; capture_method?: string }>(
      "audio-mode",
      (event) => {
        const mode = event.payload.mode as "multichannel" | "diarize";
        setAudioMode(mode);
        if (event.payload.capture_method) {
          setCaptureMethod(event.payload.capture_method);
        }
        console.log(`Audio mode: ${mode}, capture: ${event.payload.capture_method || 'unknown'}, device: ${event.payload.system_audio_device}, channels: ${event.payload.channels}`);
      }
    );
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // Listen for system-audio-silent warning
  useEffect(() => {
    const unlisten = listen<{ message: string }>(
      "system-audio-silent",
      (event) => {
        setSystemAudioSilent(true);
        setSilenceMessage(event.payload.message);
      }
    );
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // Detect if user scrolled up in transcript panel
  useEffect(() => {
    const container = transcriptContainerRef.current;
    if (!container) return;
    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = container;
      // If user is within 100px of bottom, consider them "following"
      userScrolledUp.current = scrollHeight - scrollTop - clientHeight > 100;
    };
    container.addEventListener('scroll', handleScroll);
    return () => container.removeEventListener('scroll', handleScroll);
  }, [showTranscriptPanel, appState]);

  // Auto-scroll transcript only when user hasn't scrolled up
  useEffect(() => {
    if (!userScrolledUp.current) {
      transcriptionEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [transcription]);

  // Auto-generate replies when enabled
  useEffect(() => {
    if ((isLiveTranscribing || isMockTranscribing) && autoGenerateReplies && transcription.length > 0 && transcription.length !== lastTranscriptCount.current) {
      lastTranscriptCount.current = transcription.length;
      const now = Date.now();
      const timeSinceLastGeneration = now - lastReplyGenerationTime.current;
      // Generate replies every 8 seconds regardless of speaker,
      // since speaker detection may not always differentiate correctly
      if (timeSinceLastGeneration >= 8000 || lastReplyGenerationTime.current === 0) {
        lastReplyGenerationTime.current = now;
        generateRepliesQuietly();
      }
    }
  }, [transcription, isLiveTranscribing, isMockTranscribing, autoGenerateReplies]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || (e.target as HTMLElement).isContentEditable) return;

      const keyNum = parseInt(e.key);
      if (keyNum >= 1 && keyNum <= 6 && suggestedReplies.length >= keyNum) {
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
      // Capture calendar event ID from meeting status
      if (meetingStatus?.upcoming_meeting?.id) {
        setCurrentCalendarEventId(meetingStatus.upcoming_meeting.id);
      }
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
        has_proxy: boolean;
        meeting_context: string;
      }>("get_meeting_state");
      setHasGroqKey(state.has_groq_key);
      setHasDeepgramKey(state.has_deepgram_key);
      setHasProxy(state.has_proxy);
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

  const checkCloudSyncStatus = async () => {
    try {
      const status = await invoke<{ is_configured: boolean; is_enabled: boolean }>("get_cloud_sync_status");
      setCloudSyncEnabled(status.is_enabled);
    } catch (error) {
      console.error("Failed to check cloud sync status:", error);
    }
  };

  const handleToggleCloudSync = async (enabled: boolean) => {
    try {
      await invoke("toggle_cloud_sync", { enabled });
      setCloudSyncEnabled(enabled);
    } catch (error) {
      console.error("Failed to toggle cloud sync:", error);
    }
  };

  const handleSyncNow = async () => {
    setIsSyncing(true);
    try {
      const synced = await invoke<number>("sync_meetings_to_cloud");
      alert(`Synced ${synced} meetings to cloud.`);
    } catch (error) {
      console.error("Failed to sync meetings:", error);
      alert("Failed to sync: " + error);
    } finally {
      setIsSyncing(false);
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
    setIsConnectingCalendar(true);
    try {
      const authUrl = await invoke<string>("get_google_auth_url");

      // Start the local callback server FIRST, then open the browser
      const callbackPromise = invoke<string>("wait_for_oauth_callback");
      await openUrl(authUrl);

      // Wait for the OAuth callback (server captures the code automatically)
      const code = await callbackPromise;

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
    if (!hasGroqKey && !hasDeepgramKey && !hasProxy) return;
    try {
      await invoke("start_live_transcription");
      setIsLiveTranscribing(true);
      setSuggestedReplies([]);
      setAudioMode(null);
      setCaptureMethod(null);
      setSystemAudioSilent(false);
      setSilenceMessage(null);
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

      if ((hasGroqKey || hasProxy) && transcription.length > 0) {
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
    if (isGeneratingReplies || (!hasGroqKey && !hasProxy)) return;
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
    if (transcription.length === 0 || (!hasGroqKey && !hasProxy)) return;
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

  const handleEnhanceNotes = async () => {
    if (!userNotes.trim() || (!hasGroqKey && !hasProxy)) return;
    setIsEnhancing(true);
    try {
      const result = await invoke<string>("enhance_notes", {
        userNotes: userNotes.trim(),
      });
      setEnhancedNotes(result);
    } catch (error) {
      console.error("Failed to enhance notes:", error);
    } finally {
      setIsEnhancing(false);
    }
  };

  const handleAskAboutMeeting = async () => {
    if (!askAiQuestion.trim() || (!hasGroqKey && !hasProxy)) return;
    setIsAskingAi(true);
    setAskAiAnswer("");
    try {
      const result = await invoke<string>("ask_about_meeting", {
        question: askAiQuestion.trim(),
        userNotes: userNotes.trim(),
      });
      setAskAiAnswer(result);
    } catch (error) {
      console.error("Failed to ask AI:", error);
      setAskAiAnswer("Sorry, I couldn't process your question. Please try again.");
    } finally {
      setIsAskingAi(false);
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
      setUserNotes("");
      setEnhancedNotes(null);
      setAskAiQuestion("");
      setAskAiAnswer("");
      setCurrentCalendarEventId(null);
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
        calendar_event_id: currentCalendarEventId,
        duration_seconds: recordingTime > 0 ? recordingTime : null,
        transcript: cleanTranscript,
        summary: structuredSummary,
        user_notes: userNotes.trim() || null,
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
      await loadPastMeetings();
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

  const formatSummaryForCalendar = (summary: MeetingSummary | null, notes: string): string => {
    const parts: string[] = [];
    parts.push("=== Meeting Summary (Vantage) ===\n");

    if (notes.trim()) {
      parts.push("NOTES:\n" + notes.trim() + "\n");
    }

    if (summary) {
      if (summary.key_points?.length > 0) {
        parts.push("KEY POINTS:");
        summary.key_points.forEach(p => parts.push("- " + p));
        parts.push("");
      }
      if (summary.action_items?.length > 0) {
        parts.push("ACTION ITEMS:");
        summary.action_items.forEach(p => parts.push("- " + p));
        parts.push("");
      }
      if (summary.decisions?.length > 0) {
        parts.push("DECISIONS:");
        summary.decisions.forEach(p => parts.push("- " + p));
        parts.push("");
      }
    }

    return parts.join("\n");
  };

  const handleShareToCalendar = async (eventId: string, summary: MeetingSummary | null, notes: string) => {
    setIsShareToCalendarLoading(true);
    try {
      const description = formatSummaryForCalendar(summary, notes);
      await invoke("update_calendar_event_description", {
        eventId,
        description,
      });
      alert("Meeting summary shared to Google Calendar event!");
    } catch (error) {
      console.error("Failed to share to calendar:", error);
      alert("Failed to share to calendar: " + error);
    } finally {
      setIsShareToCalendarLoading(false);
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

  const formatEventTime = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit', hour12: true });
  };

  const formatDuration = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    if (mins < 60) return `${mins}m`;
    const hrs = Math.floor(mins / 60);
    const remainMins = mins % 60;
    return remainMins > 0 ? `${hrs}h ${remainMins}m` : `${hrs}h`;
  };

  const getInitials = (email: string) => {
    const name = email.split('@')[0];
    const parts = name.split(/[._-]/);
    if (parts.length >= 2) return (parts[0][0] + parts[1][0]).toUpperCase();
    return name.substring(0, 2).toUpperCase();
  };

  const isEventHappeningNow = (event: CalendarEvent) => {
    const now = new Date();
    const start = new Date(event.start_time);
    const end = new Date(event.end_time);
    return now >= start && now <= end;
  };

  const renderEnhancedLine = (line: string, i: number) => {
    const trimmed = line.trim();
    if (!trimmed) return <br key={i} />;

    // Parse timestamps like [12:34] or [1:23:45]
    const parseTimestamps = (text: string) => {
      const parts = text.split(/(\[\d{1,2}:\d{2}(?::\d{2})?\])/g);
      return parts.map((part, j) => {
        if (/^\[\d{1,2}:\d{2}(?::\d{2})?\]$/.test(part)) {
          return <span key={j} className="timestamp-link">{part}</span>;
        }
        return part;
      });
    };

    if (trimmed.startsWith('[USER]')) {
      return <p key={i} className="user-note-line">{parseTimestamps(trimmed.replace('[USER]', '').trim())}</p>;
    }
    if (trimmed.startsWith('[AI]')) {
      return <p key={i} className="ai-note-line">{parseTimestamps(trimmed.replace('[AI]', '').trim())}</p>;
    }
    if (trimmed.startsWith('#')) {
      return <h3 key={i} className="enhanced-section-heading">{trimmed.replace(/^#+\s*/, '')}</h3>;
    }
    return <p key={i} className="ai-note-line">{parseTimestamps(trimmed)}</p>;
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
                  <button
                    onClick={handleConnectCalendar}
                    disabled={isConnectingCalendar}
                  >
                    {isConnectingCalendar ? "Connecting..." : "Connect Calendar"}
                  </button>
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

              {/* Cloud Sync */}
              <div className="setting-item">
                <label>Cloud Sync</label>
                <p className="setting-hint">Backup meetings to the cloud automatically</p>
                <div>
                  <div className="api-status">
                    <span className={`status-dot ${cloudSyncEnabled ? 'connected' : ''}`}></span>
                    <span>{cloudSyncEnabled ? 'Enabled' : 'Disabled'}</span>
                    <div className="toggle" style={{ marginLeft: 'auto' }}>
                      <input
                        type="checkbox"
                        checked={cloudSyncEnabled}
                        onChange={(e) => handleToggleCloudSync(e.target.checked)}
                      />
                      <span className="toggle-track"></span>
                    </div>
                  </div>
                  {cloudSyncEnabled && (
                    <div style={{ marginTop: '10px' }}>
                      <button
                        className="text-btn"
                        onClick={handleSyncNow}
                        disabled={isSyncing}
                      >
                        {isSyncing ? "Syncing..." : "Sync Now"}
                      </button>
                    </div>
                  )}
                </div>
              </div>

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
                    {upcomingEvents.slice(0, 5).map((event) => {
                      const isNow = isEventHappeningNow(event);
                      return (
                        <div key={event.id} className={`event-item ${event.is_today ? 'today' : ''} ${isNow ? 'now' : ''}`}>
                          <div className="event-time-col">
                            <span className="event-time-start">{formatEventTime(event.start_time)}</span>
                            <span className="event-time-end">{formatEventTime(event.end_time)}</span>
                          </div>
                          <div className="event-info">
                            <span className="event-title">
                              {event.title}
                              {isNow && <span className="live-badge">LIVE</span>}
                            </span>
                            <div className="event-meta">
                              {event.attendees.length > 0 && (
                                <div className="event-attendees">
                                  {event.attendees.slice(0, 3).map((a, i) => (
                                    <span key={i} className="attendee-avatar" title={a}>{getInitials(a)}</span>
                                  ))}
                                  {event.attendees.length > 3 && (
                                    <span className="attendee-avatar">+{event.attendees.length - 3}</span>
                                  )}
                                </div>
                              )}
                              {event.meeting_link && (
                                <span className="meeting-link-indicator" title="Has meeting link">🔗</span>
                              )}
                            </div>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <p className="empty-state">No upcoming events</p>
                )
              ) : (
                <div className="connect-calendar-prompt">
                  <p>See your schedule here</p>
                  <button className="text-btn" onClick={() => setShowSettings(true)}>Connect Google Calendar →</button>
                </div>
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
                      <div className="meeting-icon">{meeting.summary ? '📋' : '📄'}</div>
                      <div className="meeting-info">
                        <span className="meeting-title">{meeting.title}</span>
                        <span className="meeting-date">
                          {formatDate(meeting.date)}
                          {meeting.duration_seconds && (
                            <> · <span className="meeting-duration-badge">{formatDuration(meeting.duration_seconds)}</span></>
                          )}
                        </span>
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
            {!(hasGroqKey || hasDeepgramKey || hasProxy) ? (
              <div className="setup-prompt">
                <h1>Welcome to Vantage</h1>
                <p>Real-time meeting transcription with AI summaries</p>
                <div className="setup-steps">
                  <div className={`setup-step ${hasDeepgramKey ? 'done' : ''}`}>
                    <span className="step-number">1</span>
                    <div className="step-content">
                      <strong>Deepgram</strong> — Real-time transcription
                      <span className="help-link" onClick={() => openUrl("https://console.deepgram.com/")}>
                        Get free key (includes $200 credit) →
                      </span>
                    </div>
                  </div>
                  <div className={`setup-step ${hasGroqKey ? 'done' : ''}`}>
                    <span className="step-number">2</span>
                    <div className="step-content">
                      <strong>Groq</strong> — AI summaries &amp; suggestions
                      <span className="help-link" onClick={() => openUrl("https://console.groq.com/keys")}>
                        Get free key →
                      </span>
                    </div>
                  </div>
                </div>
                <button className="primary-btn large" onClick={() => setShowSettings(true)}>
                  Enter API Keys
                </button>
              </div>
            ) : (
              <div className="start-section">
                {micPermission === "denied" && (
                  <div className="permission-banner">
                    <strong>Microphone access required</strong>
                    <p>Go to <strong>System Settings → Privacy & Security → Microphone</strong> and enable Vantage. Then restart the app.</p>
                    <p className="permission-tip">If permission keeps being asked: Right-click the app → Open, or run <code>xattr -cr /Applications/Vantage.app</code> in Terminal to remove quarantine.</p>
                  </div>
                )}
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

                <textarea
                  className="context-textarea"
                  placeholder="Describe the meeting context — e.g. 'Weekly standup with engineering team' or 'Interview for senior frontend role'. This helps AI generate better suggestions and summaries."
                  value={contextInput}
                  onChange={(e) => setContextInput(e.target.value)}
                  onBlur={handleSaveContext}
                  rows={2}
                />

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
            {isCalendarConnected && selectedMeeting.calendar_event_id && (
              <button
                className="text-btn"
                onClick={() => handleShareToCalendar(
                  selectedMeeting.calendar_event_id!,
                  selectedMeeting.summary ? {
                    key_points: selectedMeeting.summary.key_points,
                    action_items: selectedMeeting.summary.action_items,
                    decisions: selectedMeeting.summary.decisions,
                    notes: selectedMeeting.summary.notes,
                    raw_summary: selectedMeeting.summary.raw_summary || "",
                  } : null,
                  selectedMeeting.user_notes || ""
                )}
                disabled={isShareToCalendarLoading}
              >
                {isShareToCalendarLoading ? "Sharing..." : "Share to Calendar"}
              </button>
            )}
            <button className="icon-btn danger" onClick={() => handleDeleteMeeting(selectedMeeting.id)} title="Delete">
              🗑️
            </button>
          </header>

          <div className="detail-content">
            <div className="detail-meta">
              <span>{formatDate(selectedMeeting.date)}</span>
              {selectedMeeting.duration_seconds && (
                <span>{formatDuration(selectedMeeting.duration_seconds)}</span>
              )}
              {selectedMeeting.attendees.length > 0 && (
                <span>{selectedMeeting.attendees.length} attendee{selectedMeeting.attendees.length !== 1 ? 's' : ''}</span>
              )}
              {selectedMeeting.summary && <span>Has summary</span>}
            </div>

            {/* User Notes */}
            {selectedMeeting.user_notes && (
              <section className="detail-section">
                <h3>Notes</h3>
                <div className="user-note-text">{selectedMeeting.user_notes}</div>
              </section>
            )}

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

      {/* RECORDING STATE - Granola-style notepad + transcript */}
      {appState === 'recording' && (
        <div className="recording-screen">
          <header className="recording-header">
            <div className="recording-status">
              <div className="green-bars">
                <span className="gbar"></span>
                <span className="gbar"></span>
                <span className="gbar"></span>
                <span className="gbar"></span>
              </div>
              <span className="rec-time">{formatTime(recordingTime)}</span>
              {audioMode && (
                <span className={`audio-mode-badge ${audioMode} ${systemAudioSilent ? 'warning' : ''}`}
                  title={audioMode === "multichannel"
                    ? `Stereo mode (${captureMethod || 'unknown'}): Channel 0 = You, Channel 1 = Participant`
                    : "Mono mode with AI diarization — speaker separation may be less accurate"
                  }
                >
                  {audioMode === "multichannel"
                    ? (systemAudioSilent ? "Stereo (no system audio)" : captureMethod === "ScreenCaptureKit" ? "SCK Stereo" : "Stereo")
                    : "Mono (diarize)"}
                </span>
              )}
            </div>
            <div className="recording-header-actions">
              <button
                className="transcript-toggle"
                onClick={() => setShowTranscriptPanel(!showTranscriptPanel)}
                title={showTranscriptPanel ? 'Hide transcript' : 'Show transcript'}
              >
                {showTranscriptPanel ? 'Hide Transcript' : 'Show Transcript'}
              </button>
              <button className="stop-btn" onClick={() => {
                if (isLiveTranscribing) handleStopLiveTranscription();
                else if (isMockTranscribing) handleMockTranscription();
              }}>
                Stop
              </button>
            </div>
          </header>

          {/* Warning: no system audio / diarize-only mode */}
          {audioMode === 'diarize' && (
            <div className="silence-warning-banner">
              <span className="silence-warning-text">
                <strong>Mono mode</strong> — Using AI to guess who's speaking (less accurate).
                For reliable "You" vs "Participant" labels, grant <strong>Screen Recording</strong> permission in System Settings → Privacy & Security → Screen Recording, then restart.
              </span>
              <button className="silence-warning-dismiss" onClick={() => setAudioMode(null)}>Dismiss</button>
            </div>
          )}

          {/* Warning: system audio channel is silent despite stereo mode */}
          {systemAudioSilent && (
            <div className="silence-warning-banner">
              <span className="silence-warning-text">
                {silenceMessage || "System audio channel is silent. Go to System Settings → Privacy & Security → Screen Recording and enable Vantage."}
              </span>
              <button className="silence-warning-dismiss" onClick={() => setSystemAudioSilent(false)}>Dismiss</button>
            </div>
          )}

          <div className="recording-body">
            {/* Notepad Panel (left) */}
            <div className={`notepad-panel ${!showTranscriptPanel ? 'full-width' : ''}`}>
              <div
                ref={notepadRef}
                className="notepad-editor"
                contentEditable
                suppressContentEditableWarning
                data-placeholder="Take notes during the meeting..."
                onInput={(e) => setUserNotes((e.target as HTMLDivElement).innerText)}
              />
              {/* Inline Suggestions Panel */}
              {autoGenerateReplies && (
                <div className={`suggestions-inline ${suggestionsMinimized ? 'minimized' : ''}`}>
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

            {/* Live Transcript Panel (right) */}
            {showTranscriptPanel && (
              <div className="transcript-panel-right">
                <div className="transcript-panel-header">
                  <span>Live Transcript</span>
                </div>
                <div className="transcript-panel-body" ref={transcriptContainerRef}>
                  {transcription.length === 0 ? (
                    <div className="empty-transcript"><p>Listening...</p></div>
                  ) : (
                    <div className="transcript-list chat-style">
                      {transcription.map((seg, i) => (
                        <div key={i} className={`transcript-item ${seg.speaker === 'You' ? 'you' : 'participant'} ${seg.is_final === false ? 'interim' : ''}`}>
                          <div className="message-bubble">
                            <span className={`speaker-label ${seg.speaker.startsWith('Speaker') ? 'diarized' : ''}`}>{seg.speaker}</span>
                            <p>{seg.text}</p>
                            {seg.is_final === false && <span className="interim-badge">...</span>}
                          </div>
                        </div>
                      ))}
                      <div ref={transcriptionEndRef} />
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>

        </div>
      )}

      {/* DONE STATE */}
      {appState === 'done' && (
        <div className="done-screen">
          <header className="minimal-header">
            <span className="logo">Vantage</span>
            <div className="header-actions">
              <button className="text-btn" onClick={handleClearAll}>← Back to Home</button>
              <button className="text-btn" onClick={() => {
                setSaveMeetingTitle("Meeting " + new Date().toLocaleDateString());
                setShowSaveMeetingModal(true);
              }}>Save Meeting</button>
              {isCalendarConnected && currentCalendarEventId && (
                <button
                  className="text-btn"
                  onClick={() => handleShareToCalendar(currentCalendarEventId, structuredSummary, userNotes)}
                  disabled={isShareToCalendarLoading}
                >
                  {isShareToCalendarLoading ? "Sharing..." : "Share to Calendar"}
                </button>
              )}
              <button className="text-btn" onClick={handleClearAll}>New Meeting</button>
              <button className="icon-btn" onClick={() => setShowSettings(true)} title="Settings">⚙️</button>
            </div>
          </header>

          <main className="done-content">
            {/* User Notes Section */}
            {userNotes.trim() && (
              <section className="user-notes-section">
                <div className="section-title">
                  <h2>Your Notes</h2>
                  <button
                    className="enhance-btn"
                    onClick={handleEnhanceNotes}
                    disabled={isEnhancing}
                  >
                    {isEnhancing ? "Enhancing..." : "Enhance with AI"}
                  </button>
                </div>
                <div className="user-note-text">{userNotes}</div>
              </section>
            )}

            {/* Enhanced Notes Section — Two-Tone Display */}
            {enhancedNotes && (
              <section className="enhanced-notes">
                <h2>Enhanced Notes</h2>
                <div className="enhanced-notes-content">
                  {enhancedNotes.split('\n').map((line, i) => renderEnhancedLine(line, i))}
                </div>
              </section>
            )}

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

            {/* Ask AI Section */}
            {(hasGroqKey || hasProxy) && (
              <section className="ask-ai-section">
                <h2>Ask AI</h2>
                <p className="ask-ai-hint">Ask a question about this meeting — e.g. "What were the main action items?" or "Summarize what John said"</p>
                <div className="ask-ai-input-row">
                  <input
                    type="text"
                    className="ask-ai-input"
                    placeholder="Ask a question about your meeting..."
                    value={askAiQuestion}
                    onChange={(e) => setAskAiQuestion(e.target.value)}
                    onKeyDown={(e) => { if (e.key === 'Enter') handleAskAboutMeeting(); }}
                    disabled={isAskingAi}
                  />
                  <button
                    className="ask-ai-btn"
                    onClick={handleAskAboutMeeting}
                    disabled={isAskingAi || !askAiQuestion.trim()}
                  >
                    {isAskingAi ? "Thinking..." : "Ask"}
                  </button>
                </div>
                {askAiAnswer && (
                  <div className="ask-ai-answer">
                    {askAiAnswer.split('\n').map((line, i) => (
                      <p key={i}>{line}</p>
                    ))}
                  </div>
                )}
              </section>
            )}

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
