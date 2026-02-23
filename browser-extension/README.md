# MeetBetter Browser Extension

Auto-detect online meetings and seamlessly integrate with the MeetBetter desktop app for real-time transcription.

## Features

- **Automatic Meeting Detection** - Detects when you join Google Meet, Zoom, or Microsoft Teams
- **One-Click Transcription** - Start transcribing with a single click
- **Desktop App Integration** - Seamlessly communicates with MeetBetter desktop app
- **Meeting History** - View recent meetings and transcripts
- **Real-time Notifications** - Get notified when meetings start/end
- **Multi-Platform Support** - Works with Google Meet, Zoom, and Microsoft Teams

## Installation

### From Source (Development)

1. **Clone the repository**
   ```bash
   git clone https://github.com/venkateswarisudalai/Vantage.git
   cd Vantage/browser-extension
   ```

2. **Load in Chrome**
   - Open Chrome and go to `chrome://extensions/`
   - Enable "Developer mode" (toggle in top right)
   - Click "Load unpacked"
   - Select the `browser-extension` folder

3. **Load in Firefox**
   - Open Firefox and go to `about:debugging#/runtime/this-firefox`
   - Click "Load Temporary Add-on"
   - Select any file in the `browser-extension` folder

### Icons (Required Before Loading)

You need to add icon files before loading the extension:

```
browser-extension/icons/
├── icon16.png   (16x16px)
├── icon48.png   (48x48px)
└── icon128.png  (128x128px)
```

You can use any PNG images for now, or create proper icons later.

**Quick way to create placeholder icons:**

```bash
cd browser-extension/icons

# On macOS with ImageMagick installed:
convert -size 16x16 xc:purple -fill white -gravity center -pointsize 12 -annotate +0+0 "MB" icon16.png
convert -size 48x48 xc:purple -fill white -gravity center -pointsize 36 -annotate +0+0 "MB" icon48.png
convert -size 128x128 xc:purple -fill white -gravity center -pointsize 96 -annotate +0+0 "MB" icon128.png

# Or create simple colored squares online at https://placeholder.com/
# Download 16x16, 48x48, and 128x128 images
```

## How It Works

### 1. Meeting Detection

The extension monitors URLs and page content to detect active meetings:

- **Google Meet**: Detects `meet.google.com` URLs and checks for meeting controls
- **Zoom**: Detects `zoom.us` URLs with active meeting sessions
- **Microsoft Teams**: Detects `teams.microsoft.com` meeting URLs

### 2. Information Extraction

When a meeting is detected, the extension extracts:
- Meeting platform
- Meeting title (if available)
- Participant count
- Meeting ID/URL
- Start time

### 3. Desktop App Communication

The extension communicates with the MeetBetter desktop app through:
- **Chrome Storage API** - For basic messaging
- **Native Messaging** (future) - For advanced integration

### 4. User Interface

- **Popup**: Click the extension icon to see current meeting status
- **Notifications**: In-page notifications when meetings start/end
- **Badge**: Shows active meeting indicator

## Usage

### Basic Usage

1. **Install the Extension** - Follow installation steps above
2. **Install Desktop App** - Download from [GitHub Releases](https://github.com/venkateswarisudalai/Vantage/releases)
3. **Join a Meeting** - Open Google Meet, Zoom, or Teams
4. **Auto Detection** - Extension automatically detects the meeting
5. **Start Transcription** - Click "Start Transcription" in the popup

### Manual Start

1. Click the MeetBetter extension icon
2. Check if meeting is detected (green badge)
3. Click "Start Transcription" button
4. Open the desktop app to view the transcript

## Supported Platforms

| Platform | Detection | Auto-start | Metadata |
|----------|-----------|------------|----------|
| Google Meet | ✅ | ✅ | ✅ Title, Participants |
| Zoom (Web) | ✅ | ✅ | ✅ Meeting ID |
| Microsoft Teams | ✅ | ✅ | ✅ Title |

## Development

### File Structure

```
browser-extension/
├── manifest.json           # Extension configuration
├── scripts/
│   ├── background.js      # Background service worker
│   └── content.js         # Content script (meeting detection)
├── popup/
│   ├── popup.html         # Popup UI
│   ├── popup.css          # Popup styles
│   └── popup.js           # Popup logic
├── icons/
│   ├── icon16.png
│   ├── icon48.png
│   └── icon128.png
└── README.md
```

### Testing

1. **Load Extension** - Follow installation steps
2. **Open DevTools** - Right-click extension icon → "Inspect popup"
3. **Join Test Meeting** - Go to Google Meet test meeting
4. **Check Console** - View logs for meeting detection
5. **Test Notifications** - Verify in-page notifications appear

### Debugging

**View Logs:**
- Background script: `chrome://extensions` → Extension details → "Inspect views: service worker"
- Content script: Open DevTools on meeting page (F12) → Console
- Popup: Right-click extension icon → "Inspect popup"

**Common Issues:**

1. **Meeting not detected**
   - Check if URL matches supported platforms
   - Verify content script is loaded (check DevTools console)
   - Try refreshing the meeting page

2. **Desktop app not connected**
   - Ensure desktop app is running
   - Check native messaging configuration (future feature)

3. **Extension not loading**
   - Verify all icon files exist
   - Check manifest.json syntax
   - Review errors in `chrome://extensions`

## Permissions Explained

- **tabs**: To detect meeting URLs
- **activeTab**: To inject content scripts
- **storage**: To save meeting history and settings
- **nativeMessaging**: To communicate with desktop app (future)
- **host_permissions**: To access meeting platform pages

## Roadmap

- [ ] Native messaging for direct desktop app communication
- [ ] Firefox and Edge support
- [ ] Custom meeting detection rules
- [ ] Meeting calendar integration
- [ ] Keyboard shortcuts
- [ ] Settings page
- [ ] Export transcripts directly from extension
- [ ] Speaker identification
- [ ] Real-time summary view

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## Privacy

The extension:
- ✅ Only runs on meeting platform pages
- ✅ Does not collect or transmit personal data
- ✅ Stores meeting metadata locally
- ✅ All transcription happens in the desktop app
- ❌ No tracking or analytics

## License

MIT License - See LICENSE file for details

## Support

- **Issues**: [GitHub Issues](https://github.com/venkateswarisudalai/Vantage/issues)
- **Discussions**: [GitHub Discussions](https://github.com/venkateswarisudalai/Vantage/discussions)
- **Email**: support@meetbetter.app

## Links

- **Desktop App**: [MeetBetter on GitHub](https://github.com/venkateswarisudalai/Vantage)
- **Releases**: [Download Desktop App](https://github.com/venkateswarisudalai/Vantage/releases)
- **Documentation**: [User Guide](https://github.com/venkateswarisudalai/Vantage#readme)

---

**Made with ❤️ for better meetings**
