#!/bin/bash
# Sign and package the Vantage app for distribution
# Usage: ./scripts/sign-and-package.sh
#
# This script tries to use "Vantage Development" certificate first (persists permissions).
# Falls back to ad-hoc signing if no certificate found.

set -e

APP_NAME="Vantage"
IDENTIFIER="com.vantage.app"
ENTITLEMENTS="src-tauri/entitlements.plist"

# Find the built app
APP_PATH=""
for candidate in \
  "src-tauri/target/universal-apple-darwin/release/bundle/macos/${APP_NAME}.app" \
  "src-tauri/target/release/bundle/macos/${APP_NAME}.app" \
  "src-tauri/target/aarch64-apple-darwin/release/bundle/macos/${APP_NAME}.app" \
  "src-tauri/target/x86_64-apple-darwin/release/bundle/macos/${APP_NAME}.app"; do
  if [ -d "$candidate" ]; then
    APP_PATH="$candidate"
    break
  fi
done

if [ -z "$APP_PATH" ]; then
  echo "ERROR: Could not find ${APP_NAME}.app bundle. Build first with: cargo tauri build"
  exit 1
fi

echo "Found app at: $APP_PATH"

# Determine signing identity
SIGN_IDENTITY="-"
if security find-identity -v -p codesigning 2>/dev/null | grep -q "Vantage Development"; then
  SIGN_IDENTITY="Vantage Development"
  echo "Using certificate: $SIGN_IDENTITY"
else
  echo "No 'Vantage Development' certificate found. Using ad-hoc signing."
  echo "To create one: open Keychain Access → Certificate Assistant → Create Certificate"
  echo "  Name: Vantage Development, Type: Code Signing"
fi

# 1. Remove extended attributes (quarantine, etc.)
echo "Removing extended attributes..."
xattr -cr "$APP_PATH"

# 2. Sign all nested binaries and frameworks first
echo "Signing nested binaries..."
find "$APP_PATH/Contents/Frameworks" -type f -perm +111 2>/dev/null | while read -r binary; do
  codesign -f -s "$SIGN_IDENTITY" --identifier "$IDENTIFIER" "$binary" 2>/dev/null || true
done

find "$APP_PATH/Contents/MacOS" -type f ! -name "$APP_NAME" 2>/dev/null | while read -r binary; do
  codesign -f -s "$SIGN_IDENTITY" --identifier "$IDENTIFIER" "$binary" 2>/dev/null || true
done

# 3. Sign the main binary with entitlements
echo "Signing main binary with entitlements..."
codesign -f -s "$SIGN_IDENTITY" \
  --identifier "$IDENTIFIER" \
  --entitlements "$ENTITLEMENTS" \
  "$APP_PATH/Contents/MacOS/$APP_NAME"

# 4. Sign the entire app bundle
echo "Signing app bundle..."
codesign -f -s "$SIGN_IDENTITY" \
  --identifier "$IDENTIFIER" \
  --entitlements "$ENTITLEMENTS" \
  "$APP_PATH"

# 5. Verify the signature
echo "Verifying signature..."
codesign -v --deep "$APP_PATH" && echo "Signature OK" || echo "WARNING: Signature verification failed"
codesign -dvv "$APP_PATH" 2>&1 | grep -E "Authority|Identifier|TeamIdentifier"

# 6. Create DMG
DMG_NAME="${APP_NAME}-$(grep '"version"' package.json | head -1 | sed 's/.*: "\(.*\)".*/\1/').dmg"
echo "Creating DMG: $DMG_NAME"

rm -f "$DMG_NAME"

hdiutil create -volname "$APP_NAME" \
  -srcfolder "$APP_PATH" \
  -ov -format UDZO \
  "$DMG_NAME"

echo ""
echo "Done! $DMG_NAME is ready to share."
echo ""
echo "IMPORTANT for recipients:"
echo "  1. After downloading/AirDrop, open Terminal and run:"
echo "     xattr -cr ~/Downloads/${DMG_NAME}"
echo "  2. Then open the DMG and drag Vantage to Applications"
echo "  3. Right-click Vantage.app → Open (first time only)"
echo "  4. Grant Microphone and Screen Recording permissions when asked"
echo "  5. Permissions will persist across app restarts"
