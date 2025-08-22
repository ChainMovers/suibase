#!/bin/bash

# Simple completion sound for Claude Code
# Plays a "bing" sound when Claude completes processing a user prompt

# Try different sound methods in order of preference
if command -v paplay >/dev/null 2>&1; then
    # PulseAudio (most common on Linux)
    paplay /usr/share/sounds/alsa/Front_Left.wav 2>/dev/null ||
    paplay /usr/share/sounds/ubuntu/stereo/message.ogg 2>/dev/null ||
    paplay /usr/share/sounds/freedesktop/stereo/message-new-instant.oga 2>/dev/null
elif command -v aplay >/dev/null 2>&1; then
    # ALSA fallback
    aplay /usr/share/sounds/alsa/Front_Left.wav 2>/dev/null
elif command -v play >/dev/null 2>&1; then
    # SoX play command
    play -n synth 0.1 sine 800 2>/dev/null
elif command -v speaker-test >/dev/null 2>&1; then
    # Last resort - very brief speaker test
    timeout 0.1 speaker-test -t sine -f 800 -l 1 2>/dev/null
else
    # If no audio commands available, just exit silently
    exit 0
fi