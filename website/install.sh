#!/bin/bash
# Vantage Installer — One-command install for macOS
# Usage: curl -fsSL https://vantage-meeting-app.netlify.app/install.sh | bash

set -e

VERSION="0.4.0"
APP_NAME="Vantage"
DMG_NAME="Vantage-${VERSION}-universal.dmg"
DOWNLOAD_URL="https://github.com/venkateswarisudalai/MeetBetter/releases/download/v${VERSION}/${DMG_NAME}"
INSTALL_DIR="/Applications"

echo ""
echo "  ╔══════════════════════════════════════╗"
echo "  ║   Vantage — AI Meeting Assistant     ║"
echo "  ║   v${VERSION}  •  macOS Universal         ║"
echo "  ╚══════════════════════════════════════╝"
echo ""

# Check macOS
if [[ "$(uname)" != "Darwin" ]]; then
  echo "❌ Vantage is only available for macOS."
  exit 1
fi

# Download
echo "⬇️  Downloading Vantage v${VERSION}..."
TMPDIR=$(mktemp -d)
curl -fSL --progress-bar "$DOWNLOAD_URL" -o "$TMPDIR/$DMG_NAME"

# Mount DMG
echo "📦 Installing..."
MOUNT_POINT=$(hdiutil attach "$TMPDIR/$DMG_NAME" -nobrowse -quiet | grep "Volumes" | awk '{print $3}')

# Copy to Applications
if [ -d "$INSTALL_DIR/$APP_NAME.app" ]; then
  echo "🔄 Updating existing installation..."
  rm -rf "$INSTALL_DIR/$APP_NAME.app"
fi
cp -R "$MOUNT_POINT/$APP_NAME.app" "$INSTALL_DIR/"

# Unmount
hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null || true

# Remove quarantine (needed for ad-hoc signed apps)
echo "🔓 Removing macOS quarantine flag..."
xattr -cr "$INSTALL_DIR/$APP_NAME.app" 2>/dev/null || true

# Clean up
rm -rf "$TMPDIR"

echo ""
echo "✅ Vantage installed to /Applications/Vantage.app"
echo ""
echo "📋 Next steps:"
echo "   1. Open Vantage from Applications or Spotlight"
echo "   2. Grant Microphone permission when asked"
echo "   3. Grant Screen Recording permission when asked"
echo "   4. Add your free API keys in Settings:"
echo "      • Deepgram: https://deepgram.com (free tier)"
echo "      • Groq: https://console.groq.com (free tier)"
echo ""
echo "🎉 You're all set! Open Vantage and start your first meeting."
echo ""

# Offer to open
read -p "Open Vantage now? [Y/n] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]] || [[ -z $REPLY ]]; then
  open "$INSTALL_DIR/$APP_NAME.app"
fi
