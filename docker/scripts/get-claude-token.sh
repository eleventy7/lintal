#!/bin/bash
# Extract Claude OAuth token from macOS Keychain
# Run this on the HOST machine, not in Docker
# Usage: ./get-claude-token.sh > .claude-token

security find-generic-password -s "claude" -w 2>/dev/null
