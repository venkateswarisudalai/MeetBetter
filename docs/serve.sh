#!/bin/bash

# Quick local server for MeetBetter website
echo "🎯 Starting MeetBetter website local server..."
echo "📂 Serving from: $(pwd)"
echo ""

# Try different server options
if command -v python3 &> /dev/null; then
    echo "✅ Using Python 3 HTTP server"
    echo "🌐 Open: http://localhost:8000"
    echo ""
    python3 -m http.server 8000
elif command -v python &> /dev/null; then
    echo "✅ Using Python 2 HTTP server"
    echo "🌐 Open: http://localhost:8000"
    echo ""
    python -m SimpleHTTPServer 8000
elif command -v npx &> /dev/null; then
    echo "✅ Using npx serve"
    echo "🌐 Server will show URL..."
    echo ""
    npx serve
else
    echo "❌ No server found. Install one of:"
    echo "  - Python: brew install python3"
    echo "  - Node.js: brew install node"
    exit 1
fi
