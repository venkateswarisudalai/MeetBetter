// MeetBetter Browser Extension - Background Service Worker
// Handles communication between content script and desktop app

console.log('MeetBetter background service worker started');

let currentMeeting = null;
let desktopAppConnected = false;

// Listen for messages from content scripts
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  console.log('Background received message:', message);

  if (message.type === 'meeting-started') {
    handleMeetingStarted(message.data, sender.tab);
    sendResponse({ success: true });

  } else if (message.type === 'meeting-ended') {
    handleMeetingEnded(message.data);
    sendResponse({ success: true });
  }

  return true; // Keep channel open for async response
});

// Handle meeting started event
async function handleMeetingStarted(meetingData, tab) {
  console.log('Meeting started:', meetingData);

  currentMeeting = {
    ...meetingData,
    tabId: tab.id,
    startTime: new Date().toISOString()
  };

  // Store meeting info
  await chrome.storage.local.set({
    currentMeeting: currentMeeting,
    lastMeetingTime: Date.now()
  });

  // Try to communicate with desktop app
  const desktopResponse = await sendToDesktopApp({
    type: 'meeting-detected',
    meeting: currentMeeting
  });

  if (desktopResponse && desktopResponse.success) {
    desktopAppConnected = true;
    console.log('Desktop app notified successfully');

    // Show notification
    chrome.notifications.create({
      type: 'basic',
      iconUrl: '../icons/icon128.png',
      title: 'MeetBetter',
      message: `Meeting detected: ${meetingData.title || 'Untitled Meeting'}\nDesktop app is ready to transcribe.`,
      priority: 2
    });

  } else {
    desktopAppConnected = false;
    console.warn('Desktop app not connected');

    // Show notification to open desktop app
    chrome.notifications.create({
      type: 'basic',
      iconUrl: '../icons/icon128.png',
      title: 'MeetBetter Desktop App Not Running',
      message: 'Please open the MeetBetter desktop app to start transcription.',
      priority: 2,
      buttons: [
        { title: 'Open Desktop App' }
      ]
    });
  }

  // Update extension badge
  chrome.action.setBadgeText({ text: '●' });
  chrome.action.setBadgeBackgroundColor({ color: '#10b981' }); // Green
}

// Handle meeting ended event
async function handleMeetingEnded(meetingData) {
  console.log('Meeting ended:', meetingData);

  if (currentMeeting) {
    currentMeeting.endTime = new Date().toISOString();

    // Notify desktop app
    await sendToDesktopApp({
      type: 'meeting-ended',
      meeting: currentMeeting
    });

    // Store in history
    const { meetingHistory = [] } = await chrome.storage.local.get('meetingHistory');
    meetingHistory.unshift(currentMeeting);

    // Keep only last 50 meetings
    if (meetingHistory.length > 50) {
      meetingHistory.length = 50;
    }

    await chrome.storage.local.set({
      meetingHistory: meetingHistory,
      currentMeeting: null
    });

    currentMeeting = null;
  }

  // Clear badge
  chrome.action.setBadgeText({ text: '' });
}

// Send message to desktop app via native messaging
async function sendToDesktopApp(message) {
  try {
    // Native messaging to desktop app
    // This requires a native messaging host to be configured
    // For now, we'll use storage as a bridge
    await chrome.storage.local.set({
      desktopMessage: {
        ...message,
        timestamp: Date.now()
      }
    });

    // The desktop app can poll this storage or we can use WebSocket
    // For a more robust solution, implement native messaging host

    return { success: true };

  } catch (error) {
    console.error('Error sending to desktop app:', error);
    return { success: false, error: error.message };
  }
}

// Listen for extension icon clicks
chrome.action.onClicked.addListener((tab) => {
  // Open popup automatically (defined in manifest)
  console.log('Extension icon clicked');
});

// Listen for notification button clicks
chrome.notifications.onButtonClicked.addListener((notificationId, buttonIndex) => {
  if (buttonIndex === 0) {
    // Open Desktop App button clicked
    // This would launch the desktop app if installed
    console.log('Open desktop app requested');

    // You can use a custom protocol handler: meetbetter://
    chrome.tabs.create({ url: 'meetbetter://open' }).catch(() => {
      // If protocol handler not registered, show install page
      chrome.tabs.create({ url: 'https://meetbetter.app/download' });
    });
  }
});

// Monitor tabs for meeting URLs
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (changeInfo.status === 'complete' && tab.url) {
    const isMeetingUrl = tab.url.includes('meet.google.com') ||
                        tab.url.includes('zoom.us') ||
                        tab.url.includes('teams.microsoft.com');

    if (isMeetingUrl) {
      console.log('Meeting URL detected:', tab.url);
      // Content script will handle the detailed detection
    }
  }
});

// Initialize on install
chrome.runtime.onInstalled.addListener((details) => {
  if (details.reason === 'install') {
    console.log('MeetBetter extension installed');

    // Set default settings
    chrome.storage.local.set({
      settings: {
        autoStart: true,
        notifications: true,
        platforms: {
          googleMeet: true,
          zoom: true,
          teams: true
        }
      }
    });

    // Open welcome page
    chrome.tabs.create({ url: chrome.runtime.getURL('popup/welcome.html') });
  }
});

// Keep service worker alive
chrome.alarms.create('keepAlive', { periodInMinutes: 1 });

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === 'keepAlive') {
    console.log('Service worker alive');
  }
});
