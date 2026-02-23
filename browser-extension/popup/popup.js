// MeetBetter Browser Extension - Popup Script

let currentMeeting = null;
let meetingStartTime = null;
let durationInterval = null;

// Initialize popup
document.addEventListener('DOMContentLoaded', async () => {
  await loadCurrentMeeting();
  await loadRecentMeetings();
  await checkDesktopAppStatus();
  setupEventListeners();
});

// Load current meeting info
async function loadCurrentMeeting() {
  const { currentMeeting: meeting } = await chrome.storage.local.get('currentMeeting');

  if (meeting) {
    currentMeeting = meeting;
    meetingStartTime = new Date(meeting.startTime);
    showCurrentMeeting(meeting);
    startDurationTimer();
  } else {
    showNoMeeting();
  }
}

// Show current meeting UI
function showCurrentMeeting(meeting) {
  document.getElementById('currentMeetingSection').style.display = 'block';
  document.getElementById('noMeetingSection').style.display = 'none';

  // Set platform
  const platformNames = {
    'google-meet': 'Google Meet',
    'zoom': 'Zoom',
    'microsoft-teams': 'Microsoft Teams'
  };
  document.getElementById('meetingPlatform').textContent =
    platformNames[meeting.platform] || meeting.platform;

  // Set title
  document.getElementById('meetingTitle').textContent =
    meeting.title || 'Untitled Meeting';

  // Set participant count
  const count = meeting.participants ? meeting.participants.length : 0;
  document.getElementById('participantCount').textContent =
    `${count} participant${count !== 1 ? 's' : ''}`;
}

// Show no meeting UI
function showNoMeeting() {
  document.getElementById('currentMeetingSection').style.display = 'none';
  document.getElementById('noMeetingSection').style.display = 'block';
}

// Start duration timer
function startDurationTimer() {
  if (durationInterval) {
    clearInterval(durationInterval);
  }

  const updateDuration = () => {
    if (!meetingStartTime) return;

    const now = new Date();
    const diff = now - meetingStartTime;
    const minutes = Math.floor(diff / 60000);
    const seconds = Math.floor((diff % 60000) / 1000);

    document.getElementById('meetingDuration').textContent =
      `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
  };

  updateDuration();
  durationInterval = setInterval(updateDuration, 1000);
}

// Load recent meetings
async function loadRecentMeetings() {
  const { meetingHistory = [] } = await chrome.storage.local.get('meetingHistory');

  const container = document.getElementById('recentMeetings');

  if (meetingHistory.length === 0) {
    container.innerHTML = '<div class="empty-state"><p style="font-size: 13px; color: #999;">No recent meetings</p></div>';
    return;
  }

  // Show last 3 meetings
  const recentMeetings = meetingHistory.slice(0, 3);

  container.innerHTML = recentMeetings.map(meeting => `
    <div class="meeting-item" data-meeting-id="${meeting.id || ''}">
      <div class="meeting-item-platform">${getPlatformName(meeting.platform)}</div>
      <div class="meeting-item-title">${meeting.title || 'Untitled Meeting'}</div>
      <div class="meeting-item-time">${formatMeetingTime(meeting.startTime)}</div>
    </div>
  `).join('');

  // Add click handlers
  container.querySelectorAll('.meeting-item').forEach(item => {
    item.addEventListener('click', () => {
      const meetingId = item.dataset.meetingId;
      openMeetingInDesktopApp(meetingId);
    });
  });
}

// Check desktop app connection status
async function checkDesktopAppStatus() {
  const statusEl = document.getElementById('desktopStatus');

  // Try to read from storage to see if desktop app is active
  const { desktopMessage } = await chrome.storage.local.get('desktopMessage');

  const isConnected = desktopMessage && (Date.now() - desktopMessage.timestamp < 60000);

  if (isConnected) {
    statusEl.classList.remove('disconnected');
    statusEl.querySelector('.text').textContent = 'Connected';
  } else {
    statusEl.classList.add('disconnected');
    statusEl.querySelector('.text').textContent = 'Not Connected';
  }
}

// Setup event listeners
function setupEventListeners() {
  // Start transcription button
  document.getElementById('startTranscriptionBtn').addEventListener('click', async () => {
    const btn = document.getElementById('startTranscriptionBtn');
    btn.textContent = 'Starting...';
    btn.disabled = true;

    // Send message to content script
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (tab) {
      chrome.tabs.sendMessage(tab.id, { type: 'start-transcription' });
    }

    setTimeout(() => {
      btn.innerHTML = '<span class="btn-icon">✓</span> Transcribing';
      btn.style.background = '#10b981';
    }, 1000);
  });

  // Open desktop app button
  document.getElementById('openDesktopBtn').addEventListener('click', () => {
    // Try custom protocol
    window.location.href = 'meetbetter://open';

    // Fallback to download page after delay
    setTimeout(() => {
      window.open('https://github.com/venkateswarisudalai/Vantage/releases', '_blank');
    }, 1000);
  });

  // Settings button
  document.getElementById('settingsBtn').addEventListener('click', () => {
    chrome.runtime.openOptionsPage();
  });

  // View all meetings
  document.getElementById('viewAllBtn').addEventListener('click', () => {
    openMeetingInDesktopApp('all');
  });

  // Help link
  document.getElementById('helpLink').addEventListener('click', (e) => {
    e.preventDefault();
    window.open('https://github.com/venkateswarisudalai/Vantage#readme', '_blank');
  });

  // Feedback link
  document.getElementById('feedbackLink').addEventListener('click', (e) => {
    e.preventDefault();
    window.open('https://github.com/venkateswarisudalai/Vantage/issues', '_blank');
  });
}

// Helper functions
function getPlatformName(platform) {
  const names = {
    'google-meet': 'Google Meet',
    'zoom': 'Zoom',
    'microsoft-teams': 'Microsoft Teams'
  };
  return names[platform] || platform;
}

function formatMeetingTime(timestamp) {
  const date = new Date(timestamp);
  const now = new Date();
  const diff = now - date;
  const days = Math.floor(diff / 86400000);

  if (days === 0) {
    return 'Today at ' + date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
  } else if (days === 1) {
    return 'Yesterday at ' + date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
  } else if (days < 7) {
    return days + ' days ago';
  } else {
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }
}

function openMeetingInDesktopApp(meetingId) {
  // This would use a custom protocol or native messaging
  window.location.href = `meetbetter://meeting/${meetingId}`;
}

// Listen for storage changes
chrome.storage.onChanged.addListener((changes, namespace) => {
  if (namespace === 'local') {
    if (changes.currentMeeting) {
      loadCurrentMeeting();
    }
    if (changes.meetingHistory) {
      loadRecentMeetings();
    }
  }
});

// Cleanup on unload
window.addEventListener('unload', () => {
  if (durationInterval) {
    clearInterval(durationInterval);
  }
});
