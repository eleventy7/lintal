#!/bin/bash
# Start Claude Code in YOLO mode (auto-approve all tool calls)
# Usage: claude-yolo [prompt]

set -e

# Ensure mise environment is loaded
eval "$(/home/developer/.local/bin/mise activate bash)"

# Change to workspace if it exists
if [ -d "/home/developer/workspace/lintal" ]; then
    cd /home/developer/workspace/lintal
fi

# Run Claude Code with dangerously skip permissions (YOLO mode)
# The --dangerously-skip-permissions flag enables auto-approval of all tool calls
if [ -n "$1" ]; then
    claude --dangerously-skip-permissions "$@"
else
    claude --dangerously-skip-permissions
fi
