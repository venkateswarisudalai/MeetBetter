# MeetBetter Browser Extension - Installation Guide

## Quick Install (5 minutes)

### Chrome / Edge / Brave

1. **Open Extensions Page**
   - Chrome: Navigate to `chrome://extensions/`
   - Edge: Navigate to `edge://extensions/`
   - Brave: Navigate to `brave://extensions/`

2. **Enable Developer Mode**
   - Toggle "Developer mode" switch in the top-right corner

3. **Load Extension**
   - Click "Load unpacked" button
   - Navigate to: `/Users/vigneshsubbiah/Documents/MeetBetter/browser-extension`
   - Click "Select" (or "Open")

4. **Verify Installation**
   - You should see "MeetBetter - Meeting Assistant" in your extensions list
   - The purple "MB" icon should appear in your toolbar

5. **Pin Extension** (Optional but recommended)
   - Click the puzzle piece icon in Chrome toolbar
   - Find "MeetBetter" and click the pin icon

### Firefox

1. **Open Debugging Page**
   - Navigate to `about:debugging#/runtime/this-firefox`

2. **Load Temporary Add-on**
   - Click "Load Temporary Add-on..."
   - Navigate to: `/Users/vigneshsubbiah/Documents/MeetBetter/browser-extension`
   - Select the `manifest.json` file
   - Click "Open"

3. **Verify Installation**
   - The extension should now appear in the list
   - Note: In Firefox, temporary extensions are removed when the browser closes

### Safari (macOS)

Safari extensions require conversion to Safari Web Extension format. We recommend using Chrome/Firefox for now.

---

## Testing the Extension

### Test Meeting Detection

1. **Open Google Meet**
   ```
   https://meet.google.com/new
   ```

2. **Join Test Meeting**
   - Click "Join now" or create a test meeting

3. **Check Detection**
   - You should see a purple notification in top-right: "Meeting detected! MeetBetter is ready to transcribe."
   - Extension badge should turn green
   - Click extension icon to see meeting details

### Test Popup

1. **Click Extension Icon**
   - Should show current meeting information
   - Platform, title, duration, participant count

2. **Click "Start Transcription"**
   - Button should change to "Transcribing"
   - Notification should appear

3. **Check Recent Meetings**
   - Should show last 3 meetings (after you've attended some)

---

## Troubleshooting

### Extension Not Loading

**Error: "Manifest file is missing or unreadable"**
- Solution: Make sure you selected the `browser-extension` folder, not a file inside it

**Error: "Icons not found"**
- Solution: Icons are already created. If missing, run:
  ```bash
  cd ~/Documents/MeetBetter/browser-extension
  node create-icons.cjs
  ```

### Meeting Not Detected

1. **Check Supported Platforms**
   - Google Meet: `meet.google.com/*`
   - Zoom: `*.zoom.us/*`
   - Microsoft Teams: `teams.microsoft.com/*`

2. **Refresh the Page**
   - After loading extension, refresh the meeting page
   - Try rejoining the meeting

3. **Check Console**
   - Press F12 to open DevTools
   - Go to Console tab
   - Look for "MeetBetter extension loaded" message
   - If not present, content script didn't load

4. **Check Permissions**
   - Go to `chrome://extensions/`
   - Click "Details" on MeetBetter extension
   - Verify "Site access" shows the meeting platforms

### Desktop App Not Connected

The extension shows "Not Connected" status:

1. **This is normal for now** - Native messaging is not yet implemented
2. **Future update** will add direct communication with desktop app
3. **Current workaround**: Extension stores meeting info in browser storage

### Notifications Not Showing

1. **Check Browser Permissions**
   - Chrome: Settings → Privacy and security → Site Settings → Notifications
   - Allow notifications for Chrome extensions

2. **Check Extension Permissions**
   - Go to `chrome://extensions/`
   - MeetBetter should have "notifications" permission

---

## Using the Extension

### Workflow

1. **Install MeetBetter Desktop App**
   - Download from: https://github.com/venkateswarisudalai/Vantage/releases
   - Install and run the app

2. **Join Meeting**
   - Open Google Meet, Zoom, or Teams in browser
   - Extension auto-detects the meeting

3. **Start Transcription**
   - Click extension icon
   - Click "Start Transcription"
   - Open desktop app to view transcript

4. **View History**
   - Click extension icon anytime
   - See recent meetings
   - Click meeting to open in desktop app

### Keyboard Shortcuts (Future Feature)

- `Alt+Shift+T` - Toggle transcription
- `Alt+Shift+M` - Open meeting history
- `Alt+Shift+S` - Open settings

---

## Updating the Extension

When you pull new updates from Git:

1. **Pull Latest Changes**
   ```bash
   cd ~/Documents/MeetBetter
   git pull
   ```

2. **Reload Extension**
   - Go to `chrome://extensions/`
   - Click the refresh icon on MeetBetter extension
   - Or click "Reload" button

---

## Uninstalling

### Chrome / Edge / Brave

1. Go to `chrome://extensions/`
2. Find "MeetBetter - Meeting Assistant"
3. Click "Remove"
4. Confirm removal

### Firefox

1. Go to `about:addons`
2. Find "MeetBetter"
3. Click "..." menu
4. Click "Remove"

---

## Development Mode

### Viewing Logs

**Background Script Logs:**
```
1. Go to chrome://extensions/
2. Find MeetBetter extension
3. Click "service worker" link (under "Inspect views")
4. Console will show background script logs
```

**Content Script Logs:**
```
1. Open meeting page
2. Press F12 (DevTools)
3. Go to Console tab
4. Look for "MeetBetter extension loaded" message
```

**Popup Logs:**
```
1. Click extension icon
2. Right-click anywhere in popup
3. Select "Inspect"
4. Console will show popup script logs
```

### Making Changes

1. **Edit Files**
   - Modify any file in `browser-extension/` folder

2. **Reload Extension**
   - Go to `chrome://extensions/`
   - Click refresh icon on MeetBetter
   - Or use Ctrl+R / Cmd+R in extension's inspect window

3. **Test Changes**
   - Refresh meeting page to reload content script
   - Close and reopen popup to reload popup script

---

## Common Issues & Solutions

| Issue | Solution |
|-------|----------|
| Extension not in toolbar | Click puzzle icon, pin MeetBetter |
| No notification on meeting join | Refresh page after loading extension |
| "Start Transcription" does nothing | Check desktop app is running |
| Recent meetings empty | History is stored locally, will populate after meetings |
| Badge not turning green | Meeting detection might have failed, check console |

---

## Next Steps

1. **Test with Real Meeting**
   - Join a real Google Meet or Zoom call
   - Verify detection works
   - Test transcription with desktop app

2. **Configure Settings** (Future)
   - Right-click extension icon → Options
   - Customize auto-start, notifications, etc.

3. **Provide Feedback**
   - Report issues: https://github.com/venkateswarisudalai/Vantage/issues
   - Suggest features: https://github.com/venkateswarisudalai/Vantage/discussions

---

## Support

- **GitHub**: https://github.com/venkateswarisudalai/Vantage
- **Issues**: https://github.com/venkateswarisudalai/Vantage/issues
- **Email**: support@meetbetter.app

---

**Happy transcribing! 🎯**
