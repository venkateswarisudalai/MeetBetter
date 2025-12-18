import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

interface TranscriptSegment {
  timestamp: string;
  speaker: string;
  text: string;
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
  const [isLoading, setIsLoading] = useState(false);
  const [groqKeyInput, setGroqKeyInput] = useState("");
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

  const transcriptionEndRef = useRef<HTMLDivElement>(null);
  const lastTranscriptCount = useRef(0);
  const lastReplyGenerationTime = useRef(0);

  useEffect(() => {
    checkApiKeys();
    checkScreenShareSupport();
  }, []);

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
    const unlisten = listen<{ text: string; timestamp: string; speaker: string }>(
      "transcript-update",
      (event) => {
        if (event.payload.text && event.payload.text.trim()) {
          setTranscription((prev) => [
            ...prev,
            {
              timestamp: event.payload.timestamp,
              speaker: event.payload.speaker,
              text: event.payload.text,
            },
          ]);
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

  // Auto-generate replies when new transcription arrives (during recording)
  useEffect(() => {
    if (isLiveTranscribing && autoGenerateReplies && transcription.length > 0 && transcription.length !== lastTranscriptCount.current) {
      lastTranscriptCount.current = transcription.length;

      // Time-based debounce - generate replies at most every 5 seconds
      const now = Date.now();
      const timeSinceLastGeneration = now - lastReplyGenerationTime.current;

      if (timeSinceLastGeneration >= 5000 || lastReplyGenerationTime.current === 0) {
        lastReplyGenerationTime.current = now;
        generateRepliesQuietly();
      }
    }
  }, [transcription, isLiveTranscribing, autoGenerateReplies]);

  const generateRepliesQuietly = async () => {
    if (isGeneratingReplies || !hasGroqKey) return;

    setIsGeneratingReplies(true);
    try {
      const replies = await invoke<string[]>("generate_auto_replies");
      setSuggestedReplies(replies);
    } catch (error) {
      console.error("Failed to generate replies:", error);
    } finally {
      setIsGeneratingReplies(false);
    }
  };

  const checkApiKeys = async () => {
    try {
      const state = await invoke<{
        has_groq_key: boolean;
      }>("get_meeting_state");
      setHasGroqKey(state.has_groq_key);
    } catch (error) {
      setHasGroqKey(false);
    }
  };

  const handleSaveApiKey = async () => {
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
      console.error("Failed to save API key:", error);
    } finally {
      setIsLoading(false);
    }
  };

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
    try {
      const replies = await invoke<string[]>("generate_auto_replies");
      setSuggestedReplies(replies);
    } catch (error) {
      console.error("Failed to generate replies:", error);
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
    <div className="app">
      {/* Left Sidebar */}
      <aside className="sidebar">
        <div className="sidebar-header">
          <span className="logo-icon">🎙️</span>
          <span className="logo-text">WiselyMeet</span>
        </div>

        <div className="sidebar-content">
          {/* API Key */}
          <div className="setting-group">
            <label className="setting-label">API Key</label>
            {hasGroqKey && !isEditingApiKey ? (
              <div className="api-connected">
                <span className="dot"></span>
                <span>Connected</span>
                <button
                  className="edit-key-btn"
                  onClick={() => setIsEditingApiKey(true)}
                  title="Change API key"
                >
                  Edit
                </button>
              </div>
            ) : (
              <>
                <div className="api-input">
                  <input
                    type="password"
                    placeholder="Enter Groq API key"
                    value={groqKeyInput}
                    onChange={(e) => setGroqKeyInput(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleSaveApiKey()}
                  />
                  <button onClick={handleSaveApiKey} disabled={isLoading || !groqKeyInput.trim()}>
                    Save
                  </button>
                </div>
                {isEditingApiKey && (
                  <button
                    className="cancel-edit-btn"
                    onClick={() => {
                      setIsEditingApiKey(false);
                      setGroqKeyInput("");
                    }}
                  >
                    Cancel
                  </button>
                )}
                <a href="https://console.groq.com/keys" target="_blank" rel="noopener noreferrer" className="link">
                  Get free key
                </a>
              </>
            )}
          </div>

          {/* Auto Replies Toggle */}
          <div className="setting-group">
            <label className="setting-label">Live Features</label>
            <label className="toggle-row">
              <span>Auto-suggest replies</span>
              <div className="toggle">
                <input
                  type="checkbox"
                  checked={autoGenerateReplies}
                  onChange={(e) => setAutoGenerateReplies(e.target.checked)}
                />
                <span className="toggle-track"></span>
              </div>
            </label>
          </div>

          {/* Privacy */}
          {screenShareSupported && (
            <div className="setting-group">
              <label className="setting-label">Privacy</label>
              <label className="toggle-row">
                <span>Hide from screen share</span>
                <div className="toggle">
                  <input
                    type="checkbox"
                    checked={hideFromScreenShare}
                    onChange={(e) => handleToggleScreenShare(e.target.checked)}
                  />
                  <span className="toggle-track"></span>
                </div>
              </label>
            </div>
          )}

          {/* Quick Actions - show when has transcript */}
          {transcription.length > 0 && (
            <div className="setting-group">
              <label className="setting-label">Actions</label>
              <div className="quick-actions">
                <button
                  className="action-btn primary"
                  onClick={handleGenerateSummary}
                  disabled={isGeneratingSummary}
                >
                  {isGeneratingSummary ? "Generating..." : "Generate Summary"}
                </button>
                <button
                  className="action-btn"
                  onClick={handleGenerateReplies}
                  disabled={isGeneratingReplies}
                >
                  {isGeneratingReplies ? "Generating..." : "Suggest Replies"}
                </button>
                <button
                  className="action-btn danger"
                  onClick={handleClearAll}
                  disabled={isLiveTranscribing}
                >
                  Clear All
                </button>
              </div>
            </div>
          )}

          {/* Saved Recording - show after meeting ends */}
          {savedRecordingPath && !isLiveTranscribing && (
            <div className="setting-group">
              <label className="setting-label">Saved Recording</label>
              <div className="recording-info">
                <span className="recording-path" title={savedRecordingPath}>
                  {savedRecordingPath.split('/').pop()}
                </span>
                <button
                  className="action-btn small"
                  onClick={handleTranscribeFromRecording}
                  disabled={isTranscribingRecording}
                >
                  {isTranscribingRecording ? "Transcribing..." : "Re-transcribe"}
                </button>
              </div>
            </div>
          )}
        </div>

        <div className="sidebar-footer">
          Powered by Groq
        </div>
      </aside>

      {/* Main Content */}
      <main className="main">
        {/* Status Bar */}
        <div className="status-bar">
          {isLiveTranscribing ? (
            <>
              <span className="rec-dot"></span>
              <span>Live Transcribing {formatTime(recordingTime)}</span>
              {isGeneratingReplies && <span className="generating-badge">Generating replies...</span>}
            </>
          ) : isRecordingOnly ? (
            <>
              <span className="rec-dot recording-only"></span>
              <span>Recording Only {formatTime(recordingTime)}</span>
            </>
          ) : hasGroqKey ? (
            transcription.length > 0 ? `${transcription.length} segments recorded` : "Ready"
          ) : (
            "Add API key for live transcription"
          )}
        </div>

        {/* Content Area */}
        <div className="content-wrapper">
          {/* Transcript Panel */}
          <div className="transcript-panel">
            <div className="panel-header">
              <h2>{isRecordingOnly ? "Recording" : "Live Transcription"}</h2>
              {isLiveTranscribing && (
                <span className="live-indicator">LIVE</span>
              )}
              {isRecordingOnly && (
                <span className="recording-indicator">REC</span>
              )}
            </div>
            <div className="transcript-area">
              {transcription.length === 0 ? (
                <div className="empty">
                  <div className="empty-icon">🎤</div>
                  {isRecordingOnly ? (
                    <>
                      <p>Recording in progress...</p>
                      <p className="empty-sub">Audio is being saved. Transcribe after recording stops.</p>
                    </>
                  ) : (
                    <>
                      <p>Choose how to start</p>
                      <p className="empty-sub">
                        <strong>Live Transcription:</strong> Real-time text as you speak<br />
                        <strong>Record Only:</strong> Save audio and transcribe later
                      </p>
                    </>
                  )}
                </div>
              ) : (
                <div className="transcript-list">
                  {transcription.map((seg, i) => (
                    <div key={i} className="transcript-item">
                      <span className="time">{seg.timestamp}</span>
                      <p>{seg.text}</p>
                    </div>
                  ))}
                  <div ref={transcriptionEndRef} />
                </div>
              )}
            </div>
          </div>

          {/* Right Panel - Always visible when there's content */}
          <div className="right-panel">
            {/* Suggested Replies - Show during and after recording */}
            <div className="panel-section">
              <div className="section-header">
                <h3>Suggested Replies</h3>
                {isGeneratingReplies && <span className="loading-dot"></span>}
              </div>
              {suggestedReplies.length > 0 ? (
                <div className="replies-list">
                  {suggestedReplies.map((reply, i) => (
                    <div key={i} className="reply-item" onClick={() => handleCopyReply(reply, i)}>
                      <p>{reply}</p>
                      <span className="copy-hint">
                        {copiedIndex === i ? "Copied!" : "Click to copy"}
                      </span>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="empty-replies">
                  <p>{isLiveTranscribing ? "Replies will appear as you talk..." : "Start recording to get suggestions"}</p>
                </div>
              )}
            </div>

            {/* Summary Section - Always show when not recording and has transcript */}
            {!isLiveTranscribing && transcription.length > 0 && (
              <div className="panel-section">
                <div className="section-header">
                  <h3>Meeting Summary</h3>
                  <button
                    className="generate-btn"
                    onClick={handleGenerateSummary}
                    disabled={isGeneratingSummary}
                  >
                    {isGeneratingSummary ? "..." : structuredSummary || summary ? "Refresh" : "Generate"}
                  </button>
                </div>
                {isGeneratingSummary ? (
                  <div className="generating">Generating summary...</div>
                ) : structuredSummary || summary ? (
                  <>
                    {structuredSummary?.key_points?.length ? (
                      <div className="summary-section">
                        <h4>Key Points</h4>
                        <ul>
                          {structuredSummary.key_points.map((p, i) => <li key={i}>{p}</li>)}
                        </ul>
                      </div>
                    ) : null}
                    {structuredSummary?.action_items?.length ? (
                      <div className="summary-section actions">
                        <h4>Action Items</h4>
                        <ul>
                          {structuredSummary.action_items.map((p, i) => <li key={i}>{p}</li>)}
                        </ul>
                      </div>
                    ) : null}
                    {structuredSummary?.decisions?.length ? (
                      <div className="summary-section">
                        <h4>Decisions</h4>
                        <ul>
                          {structuredSummary.decisions.map((p, i) => <li key={i}>{p}</li>)}
                        </ul>
                      </div>
                    ) : null}
                    {!structuredSummary?.key_points?.length && !structuredSummary?.action_items?.length && summary && (
                      <p className="summary-text">{summary}</p>
                    )}
                  </>
                ) : (
                  <div className="no-summary">Click "Generate" to create a summary</div>
                )}
              </div>
            )}
          </div>
        </div>

        {/* Controls */}
        <div className="controls">
          {!isLiveTranscribing && !isRecordingOnly ? (
            <>
              <button
                className="btn-record"
                onClick={handleStartLiveTranscription}
                disabled={!hasGroqKey}
                title={!hasGroqKey ? "Add API key first" : "Start with live transcription"}
              >
                Live Transcription
              </button>
              <button
                className="btn-record-only"
                onClick={handleStartRecordingOnly}
                title="Record audio only, transcribe later"
              >
                Record Only
              </button>
            </>
          ) : isLiveTranscribing ? (
            <button className="btn-stop" onClick={handleStopLiveTranscription}>
              Stop Transcription
            </button>
          ) : (
            <button className="btn-stop" onClick={handleStopRecordingOnly}>
              Stop Recording
            </button>
          )}
        </div>
      </main>
    </div>
  );
}

export default App;
