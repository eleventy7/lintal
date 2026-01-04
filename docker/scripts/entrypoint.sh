#!/bin/bash
# Entrypoint script for Claude Code development container
# Handles permissions and auth setup before starting SSH

set -e

# Fix ownership of mounted directories if needed
# This handles macOS uid/gid differences
if [ -d /home/developer/workspace ]; then
    chown -R developer:developer /home/developer/workspace 2>/dev/null || true
fi

if [ -d /home/developer/patches ]; then
    chown -R developer:developer /home/developer/patches 2>/dev/null || true
fi

# Handle ~/.claude mount - don't change ownership (it's from host)
# but ensure the directory exists
if [ ! -d /home/developer/.claude ]; then
    mkdir -p /home/developer/.claude
    chown developer:developer /home/developer/.claude
fi

# If CLAUDE_CODE_OAUTH_TOKEN is set, write it to a credentials file
# This allows Claude Max token to be passed via environment variable
if [ -n "$CLAUDE_CODE_OAUTH_TOKEN" ]; then
    echo "Setting up Claude OAuth credentials from environment..."
    # Claude Code looks for credentials in specific locations
    # We'll create a helper script that returns the token
    cat > /home/developer/.claude-credentials << EOF
$CLAUDE_CODE_OAUTH_TOKEN
EOF
    chown developer:developer /home/developer/.claude-credentials
    chmod 600 /home/developer/.claude-credentials
fi

# Execute the main command (sshd)
exec "$@"
