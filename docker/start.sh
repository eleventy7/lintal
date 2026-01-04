#!/bin/bash
# Quick start script for the Claude Code development container
# Usage: ./docker/start.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "Building and starting container..."
docker compose -f docker/docker-compose.yml up -d --build

echo ""
echo "Container started!"
echo ""
echo "Connect with:"
echo "  ssh -p 2222 developer@localhost"
echo "  Password: developer"
echo ""
echo "Once connected:"
echo "  cd workspace/lintal"
echo ""

if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo "Authentication:"
    echo "  Run: claude --dangerously-skip-permissions"
    echo "  Copy the OAuth URL to your browser to authenticate"
    echo ""
    echo "Or set ANTHROPIC_API_KEY before starting:"
    echo "  export ANTHROPIC_API_KEY=sk-ant-..."
    echo "  ./docker/start.sh"
else
    echo "API key detected. Run:"
    echo "  claude-yolo"
fi
echo ""
echo "To stop:"
echo "  docker compose -f docker/docker-compose.yml down"
