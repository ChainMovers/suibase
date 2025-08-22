#!/bin/bash
# Notification sound hook - plays when Claude is prompting/waiting
ffplay /home/olet/suibase/.claude/sounds/notification.mp3 -nodisp -autoexit -volume 50 >/dev/null 2>&1 &