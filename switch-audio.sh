#!/bin/bash

# Audio routing helper for MeetBetter dual-channel capture
# This script helps route audio through BlackHole for testing

echo "=== MeetBetter Audio Routing Helper ==="
echo ""
echo "Current audio setup:"
system_profiler SPAudioDataType | grep -A 2 "Default Output Device: Yes" | head -4

echo ""
echo "===================================="
echo "IMPORTANT: To test dual audio (You vs Participant):"
echo ""
echo "1. Open System Settings → Sound → Output"
echo "2. Select 'BlackHole 2ch' as output"
echo "   ⚠️  You won't hear audio (it goes to BlackHole)"
echo ""
echo "3. Play a video/audio"
echo "   - Video audio → Channel 1 (Participant)"
echo "   - Your voice → Channel 0 (You)"
echo ""
echo "4. To HEAR audio while testing:"
echo "   - Open: /Applications/Utilities/Audio MIDI Setup.app"
echo "   - Click '+' → Create Multi-Output Device"
echo "   - Check both: MacBook Pro Speakers + BlackHole 2ch"
echo "   - Use Multi-Output as your output device"
echo ""
echo "===================================="
echo ""
echo "Quick test:"
echo "1. Switch output to BlackHole 2ch"
echo "2. Run: say 'This is a participant speaking' &"
echo "3. Then say something yourself"
echo "4. Check MeetBetter - you should see:"
echo "   - You: [your words]"
echo "   - Participant: [synthesized speech]"
echo ""
