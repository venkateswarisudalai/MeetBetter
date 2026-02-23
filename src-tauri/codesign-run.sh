#!/bin/bash
# Codesign the dev binary with a stable identifier so macOS TCC
# remembers microphone/screen-recording permissions across rebuilds.
BINARY="$1"
shift

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ENTITLEMENTS="$SCRIPT_DIR/entitlements.plist"

# Sign with a STABLE identifier (com.vantage.app) so TCC treats all
# dev rebuilds as the same app. Without --identifier, ad-hoc signing
# generates a hash-based ID that changes every rebuild.
if [ -f "$ENTITLEMENTS" ]; then
    codesign -f -s - --identifier "com.vantage.app" --entitlements "$ENTITLEMENTS" "$BINARY" 2>/dev/null
else
    codesign -f -s - --identifier "com.vantage.app" "$BINARY" 2>/dev/null
fi

exec "$BINARY" "$@"
