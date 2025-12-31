import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import "./App.css";

interface TranscriptSegment {
  timestamp: string;
  speaker: string;
  text: string;
  is_final?: boolean;  // true = finalized, false = interim (still being transcribed)
}

interface MeetingSummary {
  key_points: string[];
  action_items: string[];
  decisions: string[];
  notes: string[];
  raw_summary: string;
}

function App() {
  const [isLiveTranscribing, setIsLiveTranscribing] = useState(false);
  const [isRecordingOnly, setIsRecordingOnly] = useState(false);
  const [transcription, setTranscription] = useState<TranscriptSegment[]>([]);
  const [summary, setSummary] = useState("");
  const [hasGroqKey, setHasGroqKey] = useState(false);
  const [hasDeepgramKey, setHasDeepgramKey] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [groqKeyInput, setGroqKeyInput] = useState("");
  const [deepgramKeyInput, setDeepgramKeyInput] = useState("");
  const [isEditingDeepgramKey, setIsEditingDeepgramKey] = useState(false);
  const [savedRecordingPath, setSavedRecordingPath] = useState<string | null>(null);
  const [hideFromScreenShare, setHideFromScreenShare] = useState(false);
  const [screenShareSupported, setScreenShareSupported] = useState(false);
  const [structuredSummary, setStructuredSummary] = useState<MeetingSummary | null>(null);
  const [isGeneratingSummary, setIsGeneratingSummary] = useState(false);
  const [suggestedReplies, setSuggestedReplies] = useState<string[]>([]);
  const [isGeneratingReplies, setIsGeneratingReplies] = useState(false);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const [autoGenerateReplies, setAutoGenerateReplies] = useState(true);
  const [isTranscribingRecording, setIsTranscribingRecording] = useState(false);
  const [isEditingApiKey, setIsEditingApiKey] = useState(false);
  const [meetingContext, setMeetingContext] = useState("");
  const [contextInput, setContextInput] = useState("");
  const [meetingType, setMeetingType] = useState<string>("custom");
  const [replyError, setReplyError] = useState<string | null>(null);
  const [isMockTranscribing, setIsMockTranscribing] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  const transcriptionEndRef = useRef<HTMLDivElement>(null);
  const lastTranscriptCount = useRef(0);
  const lastReplyGenerationTime = useRef(0);

  // Computed app state for UI layout
  type AppState = 'ready' | 'recording' | 'done';
  const appState: AppState = (isLiveTranscribing || isRecordingOnly || isMockTranscribing)
    ? 'recording'
    : (transcription.length > 0 ? 'done' : 'ready');

  useEffect(() => {
    checkApiKeys();
    checkScreenShareSupport();
  }, []);

  // Keyboard shortcuts: Press 1-4 to copy suggestions
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Only handle 1-4 keys when not in an input field
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }

      const keyNum = parseInt(e.key);
      if (keyNum >= 1 && keyNum <= 4 && suggestedReplies.length >= keyNum) {
        const index = keyNum - 1;
        handleCopyReply(suggestedReplies[index], index);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [suggestedReplies]);

  // Dev mode: Mock transcription handler
  const handleMockTranscription = async () => {
    if (isMockTranscribing) {
      console.log('Stopping mock transcription...');
      try {
        await invoke('stop_mock_transcription');
        setIsMockTranscribing(false);
        console.log('Mock transcription stopped');
      } catch (err) {
        console.error('Failed to stop mock:', err);
      }
    } else {
      console.log('Starting mock transcription...');
      try {
        // Uses files: you_1.wav, participant_1.wav, you_2.wav, participant_2.wav, etc.
        const result = await invoke('start_mock_transcription', {
          testAudioDir: '/Users/vigneshsubbiah/Documents/MeetBetter/src-tauri/test_audio'
        });
        setIsMockTranscribing(true);
        console.log('Mock transcription started:', result);
      } catch (err) {
        console.error('Failed to start mock:', err);
        alert('Mock transcription failed: ' + err);
      }
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

  const handleToggleScreenShare = async (enabled: boolean) => {
    try {
      await invoke("set_screen_share_exclusion", { exclude: enabled });
      setHideFromScreenShare(enabled);
    } catch (error) {
      console.error("Failed to toggle screen share exclusion:", error);
    }
  };

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
              // Final result - add to transcript (remove any trailing interim first)
              const lastIndex = prev.length - 1;
              if (lastIndex >= 0 && prev[lastIndex].is_final === false) {
                // Replace interim with final
                return [...prev.slice(0, lastIndex), newSegment];
              }
              // Just add the final result
              return [...prev, newSegment];
            } else {
              // Interim result - replace previous interim or add new one
              const lastIndex = prev.length - 1;
              if (lastIndex >= 0 && prev[lastIndex].is_final === false) {
                // Replace existing interim
                return [...prev.slice(0, lastIndex), newSegment];
              }
              // Add new interim
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

  // Auto-scroll to bottom when new transcription arrives
  useEffect(() => {
    transcriptionEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [transcription]);

  // Auto-generate replies when new transcription arrives (during recording or mock mode)
  // Only generate when the last speaker is NOT "You" (i.e., when participant speaks)
  useEffect(() => {
    if ((isLiveTranscribing || isMockTranscribing) && autoGenerateReplies && transcription.length > 0 && transcription.length !== lastTranscriptCount.current) {
      lastTranscriptCount.current = transcription.length;

      // Only generate replies when the other person speaks, not when "You" speak
      const lastSpeaker = transcription[transcription.length - 1]?.speaker;
      if (lastSpeaker === "You") {
        return; // Don't generate suggestions for your own speech
      }

      // Time-based debounce - generate replies at most every 5 seconds
      const now = Date.now();
      const timeSinceLastGeneration = now - lastReplyGenerationTime.current;

      if (timeSinceLastGeneration >= 5000 || lastReplyGenerationTime.current === 0) {
        lastReplyGenerationTime.current = now;
        generateRepliesQuietly();
      }
    }
  }, [transcription, isLiveTranscribing, isMockTranscribing, autoGenerateReplies]);

  const generateRepliesQuietly = async () => {
    if (isGeneratingReplies || !hasGroqKey) return;

    setIsGeneratingReplies(true);
    setReplyError(null);
    try {
      const replies = await invoke<string[]>("generate_auto_replies");
      setSuggestedReplies(replies);
      setReplyError(null);
    } catch (error) {
      console.error("Failed to generate replies:", error);
      const errorMsg = String(error);
      if (errorMsg.includes("rate") || errorMsg.includes("429") || errorMsg.includes("limit")) {
        setReplyError("Rate limited - waiting before retrying");
      } else if (errorMsg.includes("timeout") || errorMsg.includes("Timeout")) {
        setReplyError("Request timed out - will retry");
      } else {
        setReplyError(errorMsg.length > 100 ? errorMsg.substring(0, 100) + "..." : errorMsg);
      }
    } finally {
      setIsGeneratingReplies(false);
    }
  };

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

  const handleSaveContext = async () => {
    try {
      await invoke("set_meeting_context", { context: contextInput.trim() });
      setMeetingContext(contextInput.trim());
    } catch (error) {
      console.error("Failed to save meeting context:", error);
    }
  };

  const handleStartLiveTranscription = async () => {
    console.log("handleStartLiveTranscription called, hasGroqKey:", hasGroqKey);
    if (!hasGroqKey) {
      console.log("No Groq key, returning early");
      return;
    }

    try {
      console.log("Calling start_live_transcription...");
      await invoke("start_live_transcription");
      console.log("start_live_transcription succeeded");
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
      if (audioPath) {
        setSavedRecordingPath(audioPath);
      }

      // Auto-generate summary after stopping
      if (hasGroqKey && transcription.length > 0) {
        setIsGeneratingSummary(true);
        try {
          const summaryResult = await invoke<MeetingSummary>("generate_structured_summary");
          setStructuredSummary(summaryResult);
          if (summaryResult.raw_summary) {
            setSummary(summaryResult.raw_summary);
          }
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

  const handleStartRecordingOnly = async () => {
    try {
      const audioPath = await invoke<string>("start_recording");
      setIsRecordingOnly(true);
      setSavedRecordingPath(audioPath);
    } catch (error) {
      console.error("Failed to start recording:", error);
      alert("Failed to start recording: " + error);
    }
  };

  const handleStopRecordingOnly = async () => {
    try {
      const audioPath = await invoke<string>("stop_recording");
      setIsRecordingOnly(false);
      if (audioPath) {
        setSavedRecordingPath(audioPath);
      }
    } catch (error) {
      console.error("Failed to stop recording:", error);
    }
  };

  const handleGenerateSummary = async () => {
    if (transcription.length === 0 || !hasGroqKey) return;

    setIsLoading(true);
    setIsGeneratingSummary(true);
    try {
      const summaryResult = await invoke<MeetingSummary>("generate_structured_summary");
      setStructuredSummary(summaryResult);
      if (summaryResult.raw_summary) {
        setSummary(summaryResult.raw_summary);
      }
    } catch (error) {
      console.error("Failed to generate summary:", error);
      try {
        const result = await invoke<string>("generate_summary");
        setSummary(result);
        setStructuredSummary(null);
      } catch (fallbackError) {
        console.error("Fallback summary failed:", fallbackError);
      }
    } finally {
      setIsLoading(false);
      setIsGeneratingSummary(false);
    }
  };

  const handleGenerateReplies = async () => {
    if (transcription.length === 0 || !hasGroqKey) return;

    setIsGeneratingReplies(true);
    setReplyError(null);
    try {
      const replies = await invoke<string[]>("generate_auto_replies");
      setSuggestedReplies(replies);
      setReplyError(null);
    } catch (error) {
      console.error("Failed to generate replies:", error);
      const errorMsg = String(error);
      if (errorMsg.includes("rate") || errorMsg.includes("429") || errorMsg.includes("limit")) {
        setReplyError("Rate limited - please wait before retrying");
      } else if (errorMsg.includes("timeout") || errorMsg.includes("Timeout")) {
        setReplyError("Request timed out");
      } else {
        setReplyError(errorMsg.length > 100 ? errorMsg.substring(0, 100) + "..." : errorMsg);
      }
    } finally {
      setIsGeneratingReplies(false);
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
    } catch (error) {
      console.error("Failed to clear:", error);
    }
  };

  const handleTranscribeFromRecording = async () => {
    if (!savedRecordingPath || !hasGroqKey) return;

    setIsTranscribingRecording(true);
    try {
      const segments = await invoke<TranscriptSegment[]>("transcribe_recording", {
        filePath: savedRecordingPath
      });
      if (segments.length > 0) {
        setTranscription(prev => [...prev, ...segments]);
      }
    } catch (error) {
      console.error("Failed to transcribe recording:", error);
      alert("Failed to transcribe recording: " + error);
    } finally {
      setIsTranscribingRecording(false);
    }
  };

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  // Meeting type presets with context templates
  const meetingPresets: Record<string, { label: string; context: string }> = {
    interview: {
      label: "Interview",
      context: "Job interview. I'm the candidate. Help me highlight my experience, ask insightful questions about the role, and demonstrate genuine interest. Be confident but not arrogant."
    },
    sales: {
      label: "Sales Call",
      context: "Sales call with a potential client. I'm the seller. Help me understand their needs, address objections gracefully, build rapport, and guide toward next steps. Focus on value, not features."
    },
    team: {
      label: "Team Meeting",
      context: "Internal team meeting. Help me contribute constructively, suggest solutions, take ownership of action items, and keep discussions focused and productive."
    },
    "1on1": {
      label: "1:1",
      context: "One-on-one meeting. Help me listen actively, ask thoughtful follow-up questions, provide supportive feedback, and ensure both parties feel heard."
    },
    custom: {
      label: "Custom",
      context: ""
    }
  };

  const handleMeetingTypeChange = (type: string) => {
    setMeetingType(type);
    const preset = meetingPresets[type];
    if (preset && preset.context) {
      setContextInput(preset.context);
      // Auto-save the preset context
      invoke("set_meeting_context", { context: preset.context }).then(() => {
        setMeetingContext(preset.context);
      });
    }
  };

  const [recordingTime, setRecordingTime] = useState(0);

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

  return (
    <div className={`app-minimal ${appState}`}>
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
                    <button onClick={handleSaveGroqKey} disabled={isLoading || !groqKeyInput.trim()}>
                      Save
                    </button>
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
                    <button onClick={handleSaveDeepgramKey} disabled={isLoading || !deepgramKeyInput.trim()}>
                      Save
                    </button>
                  </div>
                )}
                <span className="help-link" onClick={() => openUrl("https://console.deepgram.com/")}>
                  Get Deepgram key →
                </span>
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

              {/* Dev Mode in Settings */}
              {import.meta.env.DEV && (
                <div className="setting-item">
                  <label>Mock Test</label>
                  <button
                    className={`text-btn dev-link ${isMockTranscribing ? 'active' : ''}`}
                    onClick={() => {
                      handleMockTranscription();
                      setShowSettings(false);
                    }}
                  >
                    {isMockTranscribing ? "Stop" : "Run"}
                  </button>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* READY STATE - Clean centered start screen */}
      {appState === 'ready' && (
        <div className="ready-screen">
          <header className="minimal-header">
            <span className="logo">MeetBetter</span>
            <button className="icon-btn" onClick={() => setShowSettings(true)} title="Settings">
              ⚙️
            </button>
          </header>

          <main className="ready-content">
            {!(hasGroqKey || hasDeepgramKey) ? (
              <div className="setup-prompt">
                <h1>Welcome to MeetBetter</h1>
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

      {/* RECORDING STATE - Full-screen transcript with floating suggestions */}
      {appState === 'recording' && (
        <div className="recording-screen">
          {/* Minimal header during recording */}
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
              if (isLiveTranscribing) {
                handleStopLiveTranscription();
              } else if (isMockTranscribing) {
                handleMockTranscription();
              } else if (isRecordingOnly) {
                handleStopRecordingOnly();
              }
            }}>
              Stop
            </button>
          </header>

          {/* Transcript Area - Takes most of the screen */}
          <main className="transcript-main">
            {transcription.length === 0 ? (
              <div className="empty-transcript">
                <p>Listening...</p>
              </div>
            ) : (
              <div className="transcript-list chat-style">
                {transcription.map((seg, i) => (
                  <div
                    key={i}
                    className={`transcript-item ${seg.speaker === 'You' ? 'you' : 'participant'} ${seg.is_final === false ? 'interim' : ''}`}
                  >
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

          {/* Floating Suggestions at Bottom */}
          {suggestedReplies.length > 0 && (() => {
            const parseSuggestion = (s: string) => {
              const isRecommended = s.startsWith('★');
              const cleaned = s.replace(/^★\s*/, '');
              const match = cleaned.match(/^(PROBE|INSIGHT|MIRROR|REFRAME|CLARIFY|LABEL):\s*(.+)$/i);
              if (match) {
                return { type: match[1].toUpperCase(), text: match[2], isRecommended, raw: s };
              }
              return { type: 'OTHER', text: cleaned, isRecommended, raw: s };
            };

            const parsed = suggestedReplies.map(parseSuggestion);
            const recommended = parsed.find(p => p.isRecommended);
            const others = parsed.filter(p => !p.isRecommended);

            return (
              <div className="floating-suggestions">
                {recommended && (
                  <button
                    className={`suggestion-chip recommended ${copiedIndex === suggestedReplies.indexOf(recommended.raw) ? 'copied' : ''}`}
                    onClick={() => handleCopyReply(recommended.raw, suggestedReplies.indexOf(recommended.raw))}
                  >
                    <span className="chip-label">★</span>
                    <span className="chip-text">{recommended.text}</span>
                  </button>
                )}
                {others.slice(0, 3).map((item, i) => (
                  <button
                    key={i}
                    className={`suggestion-chip ${copiedIndex === suggestedReplies.indexOf(item.raw) ? 'copied' : ''}`}
                    onClick={() => handleCopyReply(item.raw, suggestedReplies.indexOf(item.raw))}
                  >
                    <span className="chip-label">{item.type}</span>
                    <span className="chip-text">{item.text}</span>
                  </button>
                ))}
                {isGeneratingReplies && <span className="generating-indicator">...</span>}
              </div>
            );
          })()}

          {replyError && (
            <div className="error-toast">
              {replyError}
            </div>
          )}
        </div>
      )}

      {/* DONE STATE - Summary view */}
      {appState === 'done' && (
        <div className="done-screen">
          <header className="minimal-header">
            <span className="logo">MeetBetter</span>
            <div className="header-actions">
              <button className="text-btn" onClick={handleClearAll}>New Meeting</button>
              <button className="icon-btn" onClick={() => setShowSettings(true)} title="Settings">⚙️</button>
            </div>
          </header>

          <main className="done-content">
            {/* Summary Section */}
            <section className="summary-section-main">
              <div className="section-title">
                <h2>Meeting Summary</h2>
                <button
                  className="generate-btn"
                  onClick={handleGenerateSummary}
                  disabled={isGeneratingSummary}
                >
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
                      <ul>
                        {structuredSummary.key_points.map((p, i) => <li key={i}>{p}</li>)}
                      </ul>
                    </div>
                  )}
                  {structuredSummary.action_items?.length > 0 && (
                    <div className="summary-group actions">
                      <h3>Action Items</h3>
                      <ul>
                        {structuredSummary.action_items.map((p, i) => <li key={i}>{p}</li>)}
                      </ul>
                    </div>
                  )}
                  {structuredSummary.decisions?.length > 0 && (
                    <div className="summary-group">
                      <h3>Decisions</h3>
                      <ul>
                        {structuredSummary.decisions.map((p, i) => <li key={i}>{p}</li>)}
                      </ul>
                    </div>
                  )}
                </div>
              ) : (
                <div className="empty-summary">
                  <p>Click "Generate" to create a meeting summary</p>
                </div>
              )}
            </section>

            {/* Collapsible Transcript */}
            <section className="transcript-section">
              <details>
                <summary>
                  <h3>Full Transcript ({transcription.length} segments)</h3>
                </summary>
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

            {/* Saved Recording */}
            {savedRecordingPath && (
              <section className="recording-section">
                <p className="recording-path">
                  Recording saved: {savedRecordingPath.split('/').pop()}
                </p>
              </section>
            )}
          </main>
        </div>
      )}
    </div>
  );
}

export default App;
