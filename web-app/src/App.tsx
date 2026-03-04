import { useState, useEffect, useRef, useCallback } from 'react';
import { DeepgramClient, TranscriptResult } from './lib/deepgram';
import { generateSummary, askAboutMeeting } from './lib/groq';
import { getApiKeys, saveApiKeys, getMeetings, saveMeeting, deleteMeeting, Meeting } from './lib/storage';
import './App.css';

type Page = 'home' | 'meeting' | 'done' | 'settings' | 'history' | 'view-meeting';
type AudioMode = 'mic' | 'tab' | 'both';

interface TranscriptLine {
  text: string;
  speaker?: number;
  timestamp: number;
  isFinal: boolean;
}

function CopyButton({ text, label }: { text: string; label: string }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback for older browsers
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.position = 'fixed';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.select();
      document.execCommand('copy');
      document.body.removeChild(ta);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };
  return (
    <button className="btn btn-ghost btn-sm" onClick={handleCopy}>
      {copied ? 'Copied!' : label}
    </button>
  );
}

function App() {
  const [page, setPage] = useState<Page>('home');
  const [keys, setKeys] = useState(getApiKeys());
  const [transcript, setTranscript] = useState<TranscriptLine[]>([]);
  const [isRecording, setIsRecording] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [meetingContext, setMeetingContext] = useState('');
  const [summary, setSummary] = useState('');
  const [loadingSummary, setLoadingSummary] = useState(false);
  const [meetings, setMeetings] = useState<Meeting[]>(getMeetings());
  const [viewMeeting, setViewMeeting] = useState<Meeting | null>(null);
  const [askQuestion, setAskQuestion] = useState('');
  const [askAnswer, setAskAnswer] = useState('');
  const [asking, setAsking] = useState(false);
  const [error, setError] = useState('');
  const [connectionStatus, setConnectionStatus] = useState('');
  const [audioMode, setAudioMode] = useState<AudioMode>('both');

  const clientRef = useRef<DeepgramClient | null>(null);
  const timerRef = useRef<number | null>(null);
  const transcriptEndRef = useRef<HTMLDivElement>(null);
  const startTimeRef = useRef<number>(0);

  // Auto-scroll transcript
  useEffect(() => {
    transcriptEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [transcript]);

  // Check if keys are configured
  const hasKeys = keys.deepgram.length > 5 && keys.groq.length > 5;

  const handleSaveKeys = (dg: string, gr: string) => {
    const newKeys = { deepgram: dg, groq: gr };
    setKeys(newKeys);
    saveApiKeys(newKeys);
    setPage('home');
  };

  const getFullTranscript = useCallback(() => {
    return transcript
      .filter(l => l.isFinal)
      .map(l => {
        const speaker = l.speaker !== undefined ? `Speaker ${l.speaker + 1}` : 'You';
        return `[${speaker}]: ${l.text}`;
      })
      .join('\n');
  }, [transcript]);

  const startMeeting = async () => {
    if (!hasKeys) {
      setPage('settings');
      return;
    }

    setError('');
    setTranscript([]);
    setSummary('');
    setElapsed(0);
    setConnectionStatus('');

    try {
      const client = new DeepgramClient(
        keys.deepgram,
        (result: TranscriptResult) => {
          setTranscript(prev => {
            if (result.isFinal) {
              const filtered = prev.filter(l => l.isFinal);
              return [...filtered, {
                text: result.text,
                speaker: result.speaker,
                timestamp: Date.now(),
                isFinal: true,
              }];
            } else {
              const finals = prev.filter(l => l.isFinal);
              return [...finals, {
                text: result.text,
                speaker: result.speaker,
                timestamp: Date.now(),
                isFinal: false,
              }];
            }
          });
        },
        (errorMsg) => setError(errorMsg),
        (status) => setConnectionStatus(status),
        {
          captureMic: audioMode === 'mic' || audioMode === 'both',
          captureTab: audioMode === 'tab' || audioMode === 'both',
        },
      );

      await client.start();
      clientRef.current = client;
      setIsRecording(true);
      startTimeRef.current = Date.now();
      setPage('meeting');

      // Start timer
      timerRef.current = window.setInterval(() => {
        setElapsed(Math.floor((Date.now() - startTimeRef.current) / 1000));
      }, 1000);
    } catch (err: any) {
      setError(err.message || 'Failed to start. Please allow microphone access.');
    }
  };

  const stopMeeting = async () => {
    clientRef.current?.stop();
    clientRef.current = null;
    setIsRecording(false);

    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }

    setPage('done');

    // Generate summary
    const fullTranscript = transcript
      .filter(l => l.isFinal)
      .map(l => {
        const speaker = l.speaker !== undefined ? `Speaker ${l.speaker + 1}` : 'You';
        return `[${speaker}]: ${l.text}`;
      })
      .join('\n');

    if (fullTranscript.trim().length > 20) {
      setLoadingSummary(true);
      try {
        const s = await generateSummary(keys.groq, fullTranscript, meetingContext);
        setSummary(s);
      } catch (err: any) {
        setSummary('Failed to generate summary: ' + err.message);
      } finally {
        setLoadingSummary(false);
      }
    }
  };

  const saveMeetingToHistory = () => {
    const fullTranscript = getFullTranscript();
    const meeting: Meeting = {
      id: Date.now().toString(),
      title: meetingContext || `Meeting on ${new Date().toLocaleDateString()}`,
      date: new Date().toISOString(),
      duration: elapsed,
      transcript: fullTranscript,
      summary: summary || undefined,
      context: meetingContext || undefined,
    };
    saveMeeting(meeting);
    setMeetings(getMeetings());
  };

  const handleAsk = async () => {
    if (!askQuestion.trim()) return;
    setAsking(true);
    try {
      const fullTranscript = viewMeeting ? viewMeeting.transcript : getFullTranscript();
      const answer = await askAboutMeeting(keys.groq, fullTranscript, askQuestion);
      setAskAnswer(answer);
    } catch (err: any) {
      setAskAnswer('Error: ' + err.message);
    } finally {
      setAsking(false);
    }
  };

  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60).toString().padStart(2, '0');
    const s = (secs % 60).toString().padStart(2, '0');
    return `${m}:${s}`;
  };

  const formatDuration = (secs: number) => {
    if (secs < 60) return `${secs}s`;
    return `${Math.floor(secs / 60)}m ${secs % 60}s`;
  };

  // =================== RENDER ===================

  // Settings Page
  if (page === 'settings') {
    return (
      <div className="app">
        <Header onBack={() => setPage('home')} title="Settings" />
        <SettingsPage
          deepgram={keys.deepgram}
          groq={keys.groq}
          onSave={handleSaveKeys}
        />
      </div>
    );
  }

  // History Page
  if (page === 'history') {
    return (
      <div className="app">
        <Header onBack={() => setPage('home')} title="Past Meetings" />
        <div className="content">
          {meetings.length === 0 ? (
            <div className="empty-state">
              <div className="empty-icon">📋</div>
              <h3>No meetings yet</h3>
              <p>Your meeting recordings will appear here</p>
            </div>
          ) : (
            <div className="meetings-list">
              {meetings.map(m => (
                <div key={m.id} className="meeting-card" onClick={() => { setViewMeeting(m); setPage('view-meeting'); }}>
                  <div className="meeting-card-info">
                    <h4>{m.title}</h4>
                    <p>{new Date(m.date).toLocaleDateString()} &middot; {formatDuration(m.duration)}</p>
                  </div>
                  <button className="btn-icon" onClick={(e) => { e.stopPropagation(); deleteMeeting(m.id); setMeetings(getMeetings()); }}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    );
  }

  // View Meeting
  if (page === 'view-meeting' && viewMeeting) {
    return (
      <div className="app">
        <Header onBack={() => { setViewMeeting(null); setPage('history'); setAskAnswer(''); setAskQuestion(''); }} title={viewMeeting.title} />
        <div className="content">
          <div className="done-actions" style={{ marginTop: 0, marginBottom: 16 }}>
            <CopyButton text={viewMeeting.transcript} label="Copy Transcript" />
            {viewMeeting.summary && (
              <CopyButton text={viewMeeting.summary} label="Copy Summary" />
            )}
            <button className="btn btn-sm" style={{ background: 'var(--red)', color: 'white', marginLeft: 'auto' }} onClick={() => {
              deleteMeeting(viewMeeting.id);
              setMeetings(getMeetings());
              setViewMeeting(null);
              setPage('history');
            }}>
              Delete
            </button>
          </div>
          {viewMeeting.summary && (
            <div className="summary-card">
              <h3>Summary</h3>
              <div className="summary-text" dangerouslySetInnerHTML={{ __html: renderMarkdown(viewMeeting.summary) }} />
            </div>
          )}
          <div className="transcript-section">
            <h3>Transcript</h3>
            <div className="transcript-box">
              {viewMeeting.transcript.split('\n').filter(l => l.trim()).map((line, i) => {
                const isUser = line.startsWith('[You]') || line.startsWith('[Speaker 1]');
                return (
                  <div key={i} className={`chat-bubble-sm ${isUser ? 'chat-user-sm' : 'chat-participant-sm'}`}>
                    <span className="chat-speaker-sm">{isUser ? 'You' : 'Participant'}</span>
                    <span>{line.replace(/^\[.*?\]:\s*/, '')}</span>
                  </div>
                );
              })}
            </div>
          </div>
          <div className="ask-section">
            <h3>Ask about this meeting</h3>
            <div className="ask-input-row">
              <input
                value={askQuestion}
                onChange={e => setAskQuestion(e.target.value)}
                placeholder="What were the action items?"
                onKeyDown={e => e.key === 'Enter' && handleAsk()}
              />
              <button onClick={handleAsk} disabled={asking} className="btn btn-sm">
                {asking ? '...' : 'Ask'}
              </button>
            </div>
            {askAnswer && <div className="ask-answer">{askAnswer}</div>}
          </div>
        </div>
      </div>
    );
  }

  // Meeting (recording)
  if (page === 'meeting') {
    return (
      <div className="app">
        <div className="meeting-header">
          <div className="recording-indicator">
            <span className="recording-dot"></span>
            <span>Recording</span>
          </div>
          <span className="timer">{formatTime(elapsed)}</span>
          <button className="btn btn-stop" onClick={stopMeeting}>Stop</button>
        </div>
        <div className="transcript-live">
          {transcript.length === 0 ? (
            <div className="transcript-waiting">
              <div className="pulse-ring"></div>
              <p>{connectionStatus || 'Connecting...'}</p>
            </div>
          ) : (
            transcript.map((line, i) => {
              const isUser = line.speaker === undefined || line.speaker === 0;
              return (
                <div key={i} className={`chat-bubble ${isUser ? 'chat-user' : 'chat-participant'} ${line.isFinal ? '' : 'chat-interim'}`}>
                  <span className="chat-speaker">{isUser ? 'You' : `Participant ${line.speaker}`}</span>
                  <span className="chat-text">{line.text}</span>
                </div>
              );
            })
          )}
          <div ref={transcriptEndRef} />
        </div>
      </div>
    );
  }

  // Done (summary)
  if (page === 'done') {
    const fullTranscript = getFullTranscript();
    return (
      <div className="app">
        <Header onBack={() => setPage('home')} title="Meeting Complete" />
        <div className="content">
          <div className="done-stats">
            <div className="stat">
              <span className="stat-value">{formatTime(elapsed)}</span>
              <span className="stat-label">Duration</span>
            </div>
            <div className="stat">
              <span className="stat-value">{transcript.filter(l => l.isFinal).length}</span>
              <span className="stat-label">Utterances</span>
            </div>
          </div>

          {loadingSummary ? (
            <div className="summary-loading">
              <div className="spinner"></div>
              <p>Generating AI summary...</p>
            </div>
          ) : summary ? (
            <div className="summary-card">
              <h3>AI Summary</h3>
              <div className="summary-text" dangerouslySetInnerHTML={{ __html: renderMarkdown(summary) }} />
            </div>
          ) : (
            <div className="summary-card">
              <p style={{ color: '#999' }}>Not enough transcript to generate a summary.</p>
            </div>
          )}

          <div className="transcript-section">
            <h3>Full Transcript</h3>
            <div className="transcript-box">
              {transcript.filter(l => l.isFinal).map((line, i) => {
                const isUser = line.speaker === undefined || line.speaker === 0;
                return (
                  <div key={i} className={`chat-bubble-sm ${isUser ? 'chat-user-sm' : 'chat-participant-sm'}`}>
                    <span className="chat-speaker-sm">{isUser ? 'You' : `Participant ${line.speaker}`}</span>
                    <span>{line.text}</span>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="ask-section">
            <h3>Ask about this meeting</h3>
            <div className="ask-input-row">
              <input
                value={askQuestion}
                onChange={e => setAskQuestion(e.target.value)}
                placeholder="What did we decide about...?"
                onKeyDown={e => e.key === 'Enter' && handleAsk()}
              />
              <button onClick={handleAsk} disabled={asking} className="btn btn-sm">
                {asking ? '...' : 'Ask'}
              </button>
            </div>
            {askAnswer && <div className="ask-answer">{askAnswer}</div>}
          </div>

          <div className="done-actions">
            <button className="btn btn-primary" onClick={() => { saveMeetingToHistory(); setPage('home'); }}>
              Save &amp; Close
            </button>
            <CopyButton text={fullTranscript} label="Copy Transcript" />
            {summary && <CopyButton text={summary} label="Copy Summary" />}
          </div>
        </div>
      </div>
    );
  }

  // Home
  return (
    <div className="app">
      <nav className="home-nav">
        <div className="logo">
          <span className="logo-icon">V</span>
          <span>Vantage</span>
        </div>
        <div className="nav-actions">
          <button className="btn-icon" onClick={() => setPage('history')} title="History">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
          </button>
          <button className="btn-icon" onClick={() => setPage('settings')} title="Settings">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="3"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/></svg>
          </button>
        </div>
      </nav>

      <div className="home-content">
        <div className="home-hero">
          <h1>Your meetings,<br /><em>captured</em></h1>
          <p>Real-time transcription and AI summaries. Everything stays in your browser.</p>
        </div>

        {!hasKeys && (
          <div className="setup-banner" onClick={() => setPage('settings')}>
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>
            <span>Set up your free API keys to get started</span>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M9 18l6-6-6-6"/></svg>
          </div>
        )}

        <div className="meeting-setup">
          <textarea
            className="context-input"
            placeholder="Describe the meeting context (optional)&#10;e.g., Weekly standup with engineering team"
            value={meetingContext}
            onChange={e => setMeetingContext(e.target.value)}
            rows={3}
          />

          {/* Audio source selector */}
          <div className="audio-mode-selector">
            <label className="audio-mode-label">Audio source:</label>
            <div className="audio-mode-options">
              <button
                className={`audio-mode-btn ${audioMode === 'mic' ? 'active' : ''}`}
                onClick={() => setAudioMode('mic')}
              >
                Mic only
              </button>
              <button
                className={`audio-mode-btn ${audioMode === 'tab' ? 'active' : ''}`}
                onClick={() => setAudioMode('tab')}
              >
                Tab audio
              </button>
              <button
                className={`audio-mode-btn ${audioMode === 'both' ? 'active' : ''}`}
                onClick={() => setAudioMode('both')}
              >
                Mic + Tab
              </button>
            </div>
          </div>
          <div className="audio-mode-info">
            {audioMode === 'mic' && (
              <p>Captures your microphone only. Best for in-person meetings or when you're the only speaker.</p>
            )}
            {audioMode === 'tab' && (
              <div>
                <p>Captures audio from a browser tab (YouTube, webinar, etc). No mic.</p>
                <div className="audio-mode-steps">
                  <p><strong>How it works:</strong></p>
                  <ol>
                    <li>Click "Start Meeting" below</li>
                    <li>In the popup, click <strong>"Chrome Tab"</strong> at the top</li>
                    <li>Select the tab playing audio (e.g. YouTube)</li>
                    <li>Check <strong>"Share audio"</strong> at the bottom-left</li>
                    <li>Click "Share"</li>
                  </ol>
                  <p className="audio-mode-warning">"Entire Screen" and "Window" do not capture audio on macOS — you must select a Chrome Tab.</p>
                </div>
              </div>
            )}
            {audioMode === 'both' && (
              <div>
                <p>Captures both your mic and a browser tab's audio. Best for web meetings (Google Meet, Zoom in browser).</p>
                <div className="audio-mode-steps">
                  <p><strong>How it works:</strong></p>
                  <ol>
                    <li>Click "Start Meeting" below</li>
                    <li>In the popup, click <strong>"Chrome Tab"</strong> at the top</li>
                    <li>Select the tab with your meeting</li>
                    <li>Check <strong>"Share audio"</strong> at the bottom-left</li>
                    <li>Click "Share" — then allow microphone access</li>
                  </ol>
                  <p className="audio-mode-warning">"Entire Screen" and "Window" do not capture audio on macOS. On Windows, "Entire Screen" + "Share system audio" works.</p>
                </div>
              </div>
            )}
          </div>

          <button className="btn btn-primary btn-large start-btn" onClick={startMeeting}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><path d="M12 1a3 3 0 00-3 3v8a3 3 0 006 0V4a3 3 0 00-3-3z"/><path d="M19 10v2a7 7 0 01-14 0v-2" fill="none" stroke="currentColor" strokeWidth="2"/><line x1="12" y1="19" x2="12" y2="23" stroke="currentColor" strokeWidth="2"/></svg>
            Start Meeting
          </button>
          {error && <p className="error-text">{error}</p>}
        </div>

        {meetings.length > 0 && (
          <div className="recent-meetings">
            <h3>Recent Meetings</h3>
            {meetings.slice(0, 3).map(m => (
              <div key={m.id} className="meeting-card" onClick={() => { setViewMeeting(m); setPage('view-meeting'); }}>
                <div className="meeting-card-info">
                  <h4>{m.title}</h4>
                  <p>{new Date(m.date).toLocaleDateString()} &middot; {formatDuration(m.duration)}</p>
                </div>
                <button className="btn-icon" onClick={(e) => { e.stopPropagation(); deleteMeeting(m.id); setMeetings(getMeetings()); }}>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>
                </button>
              </div>
            ))}
            {meetings.length > 3 && (
              <button className="btn btn-ghost btn-sm" onClick={() => setPage('history')}>View all</button>
            )}
          </div>
        )}

        <div className="home-features">
          <div className="feature">
            <div className="feature-icon">🎙️</div>
            <div>
              <h4>Live Transcription</h4>
              <p>Words appear as you speak with Deepgram Nova-3</p>
            </div>
          </div>
          <div className="feature">
            <div className="feature-icon">🤖</div>
            <div>
              <h4>AI Summaries</h4>
              <p>Key points, action items, and decisions via Groq</p>
            </div>
          </div>
          <div className="feature">
            <div className="feature-icon">🔒</div>
            <div>
              <h4>Privacy-First</h4>
              <p>Everything stays in your browser. No server, no tracking</p>
            </div>
          </div>
        </div>

        <footer className="home-footer">
          <p>Vantage &middot; <a href="https://github.com/venkateswarisudalai/MeetBetter" target="_blank" rel="noopener">Open Source</a> &middot; Also available as a <a href="https://vantage-meeting-app.netlify.app" target="_blank" rel="noopener">macOS desktop app</a></p>
        </footer>
      </div>
    </div>
  );
}

// Simple components
function Header({ onBack, title }: { onBack: () => void; title: string }) {
  return (
    <div className="page-header">
      <button className="btn-back" onClick={onBack}>
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M15 18l-6-6 6-6"/></svg>
      </button>
      <h2>{title}</h2>
    </div>
  );
}

function SettingsPage({ deepgram, groq, onSave }: { deepgram: string; groq: string; onSave: (dg: string, gr: string) => void }) {
  const [dg, setDg] = useState(deepgram);
  const [gr, setGr] = useState(groq);

  return (
    <div className="content settings-content">
      <div className="settings-group">
        <label>Deepgram API Key</label>
        <p className="settings-help">Sign up free at <a href="https://deepgram.com" target="_blank" rel="noopener">deepgram.com</a> — $200 free credit</p>
        <input type="password" value={dg} onChange={e => setDg(e.target.value)} placeholder="Enter your Deepgram API key" />
      </div>
      <div className="settings-group">
        <label>Groq API Key</label>
        <p className="settings-help">Sign up free at <a href="https://console.groq.com" target="_blank" rel="noopener">console.groq.com</a> — generous free tier</p>
        <input type="password" value={gr} onChange={e => setGr(e.target.value)} placeholder="Enter your Groq API key" />
      </div>
      <button className="btn btn-primary" onClick={() => onSave(dg, gr)}>Save Keys</button>
      <p className="settings-note">Your keys are stored locally in your browser. They never leave your device.</p>
    </div>
  );
}

function renderMarkdown(text: string): string {
  return text
    .replace(/## (.+)/g, '<h4>$1</h4>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/^- (.+)/gm, '<li>$1</li>')
    .replace(/(<li>.*<\/li>)/s, '<ul>$1</ul>')
    .replace(/\n/g, '<br/>');
}

export default App;
