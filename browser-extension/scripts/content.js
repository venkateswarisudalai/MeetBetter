// MeetBetter Browser Extension - Content Script
// Detects meetings and communicates with the desktop app

console.log('MeetBetter extension loaded');

let meetingDetected = false;
let meetingInfo = {
  platform: null,
  title: null,
  participants: [],
  meetingId: null,
  url: window.location.href
};

// Detect which meeting platform we're on
function detectPlatform() {
  const url = window.location.href;

  if (url.includes('meet.google.com')) {
    return 'google-meet';
  } else if (url.includes('zoom.us')) {
    return 'zoom';
  } else if (url.includes('teams.microsoft.com')) {
    return 'microsoft-teams';
  }

  return null;
}

// Extract meeting information based on platform
function extractMeetingInfo(platform) {
  let info = {
    platform: platform,
    title: null,
    participants: [],
    meetingId: null,
    url: window.location.href
  };

  try {
    if (platform === 'google-meet') {
      // Extract Google Meet info
      const meetingCode = window.location.pathname.split('/').pop();
      info.meetingId = meetingCode;

      // Try to get meeting title from page
      const titleElement = document.querySelector('[data-meeting-title]') ||
                          document.querySelector('h1');
      if (titleElement) {
        info.title = titleElement.textContent.trim();
      }

      // Count participants
      const participantsList = document.querySelectorAll('[data-participant-id]');
      info.participants = Array.from(participantsList).map(p => p.getAttribute('data-participant-id'));

    } else if (platform === 'zoom') {
      // Extract Zoom info
      const urlParams = new URLSearchParams(window.location.search);
      info.meetingId = urlParams.get('confno') || 'unknown';

      const titleElement = document.querySelector('.meeting-topic') ||
                          document.querySelector('h3');
      if (titleElement) {
        info.title = titleElement.textContent.trim();
      }

    } else if (platform === 'microsoft-teams') {
      // Extract Teams info
      const titleElement = document.querySelector('[data-tid="meeting-title"]') ||
                          document.querySelector('h1');
      if (titleElement) {
        info.title = titleElement.textContent.trim();
      }
    }
  } catch (error) {
    console.error('Error extracting meeting info:', error);
  }

  return info;
}

// Check if we're in an active meeting
function checkMeetingStatus() {
  const platform = detectPlatform();

  if (!platform) {
    return false;
  }

  let inMeeting = false;

  // Platform-specific detection
  if (platform === 'google-meet') {
    // Check if video/audio controls are present
    inMeeting = document.querySelector('[data-is-muted]') !== null ||
                document.querySelector('[aria-label*="microphone"]') !== null ||
                document.querySelector('[data-call-started]') !== null;

  } else if (platform === 'zoom') {
    // Check for Zoom meeting controls
    inMeeting = document.querySelector('.meeting-controls') !== null ||
                document.querySelector('[aria-label*="Mute"]') !== null;

  } else if (platform === 'microsoft-teams') {
    // Check for Teams meeting controls
    inMeeting = document.querySelector('[data-tid="calling-buttons"]') !== null ||
                document.querySelector('[aria-label*="Mute"]') !== null;
  }

  return inMeeting;
}

// Notify extension about meeting status
function notifyMeetingStatus() {
  const platform = detectPlatform();
  const inMeeting = checkMeetingStatus();

  if (inMeeting && !meetingDetected) {
    // Meeting just started
    meetingDetected = true;
    meetingInfo = extractMeetingInfo(platform);

    console.log('Meeting detected:', meetingInfo);

    // Send message to background script
    chrome.runtime.sendMessage({
      type: 'meeting-started',
      data: meetingInfo
    });

    // Visual notification
    showNotification('Meeting detected! MeetBetter is ready to transcribe.');

  } else if (!inMeeting && meetingDetected) {
    // Meeting ended
    meetingDetected = false;

    console.log('Meeting ended');

    chrome.runtime.sendMessage({
      type: 'meeting-ended',
      data: meetingInfo
    });

    showNotification('Meeting ended. Transcript saved to MeetBetter.');
  }
}

// Show in-page notification
function showNotification(message) {
  const notification = document.createElement('div');
  notification.id = 'meetbetter-notification';
  notification.style.cssText = `
    position: fixed;
    top: 20px;
    right: 20px;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
    padding: 16px 24px;
    border-radius: 12px;
    box-shadow: 0 8px 16px rgba(0,0,0,0.2);
    z-index: 10000;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 14px;
    font-weight: 500;
    max-width: 300px;
    animation: slideIn 0.3s ease-out;
  `;

  notification.textContent = '🎯 ' + message;

  // Add animation
  const style = document.createElement('style');
  style.textContent = `
    @keyframes slideIn {
      from {
        transform: translateX(400px);
        opacity: 0;
      }
      to {
        transform: translateX(0);
        opacity: 1;
      }
    }
  `;
  document.head.appendChild(style);

  document.body.appendChild(notification);

  // Remove after 5 seconds
  setTimeout(() => {
    notification.style.animation = 'slideIn 0.3s ease-out reverse';
    setTimeout(() => notification.remove(), 300);
  }, 5000);
}

// Listen for messages from popup or background
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'get-meeting-info') {
    sendResponse({
      detected: meetingDetected,
      info: meetingInfo
    });
  } else if (message.type === 'start-transcription') {
    showNotification('Starting transcription...');
    sendResponse({ success: true });
  }
});

// Monitor for meeting status changes
setInterval(notifyMeetingStatus, 3000); // Check every 3 seconds

// Initial check
setTimeout(notifyMeetingStatus, 2000);
