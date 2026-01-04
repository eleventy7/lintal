#!/bin/bash
# Start Claude Code container with an ISOLATED worktree
# This protects your main checkout from any damage
#
# Usage: ./docker/start-isolated.sh [branch-name]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BRANCH_NAME="${1:-claude-work}"
WORKTREE_DIR="$(dirname "$PROJECT_DIR")/lintal-claude-worktree"

cd "$PROJECT_DIR"

# Create worktree if it doesn't exist
if [ ! -d "$WORKTREE_DIR" ]; then
    echo "Creating isolated worktree..."
    "$SCRIPT_DIR/scripts/create-worktree.sh" "$BRANCH_NAME"
    echo ""
fi

# Verify worktree exists
if [ ! -d "$WORKTREE_DIR/.git" ] && [ ! -f "$WORKTREE_DIR/.git" ]; then
    echo "Error: Worktree not found at $WORKTREE_DIR"
    echo "Run: ./docker/scripts/create-worktree.sh"
    exit 1
fi

# Export for docker-compose
export WORKTREE_PATH="$WORKTREE_DIR"

echo "Building and starting container..."
docker compose -f docker/docker-compose.worktree.yml up -d --build

echo ""
echo "Container started with ISOLATED worktree!"
echo ""
echo "Worktree: $WORKTREE_DIR"
echo "Branch:   $BRANCH_NAME"
echo ""
echo "Connect with:"
echo "  ssh -p 2222 developer@localhost"
echo "  Password: developer"
echo ""
echo "Once connected:"
echo "  cd workspace/lintal"
echo "  claude --dangerously-skip-permissions"
echo ""
echo "Your main checkout at $PROJECT_DIR is SAFE."
echo ""
echo "To review changes:"
echo "  cd $WORKTREE_DIR && git diff"
echo ""
echo "To merge good changes:"
echo "  cd $PROJECT_DIR && git merge $BRANCH_NAME"
echo ""
echo "To discard everything:"
echo "  docker compose -f docker/docker-compose.worktree.yml down"
echo "  git worktree remove $WORKTREE_DIR"
echo "  git branch -D $BRANCH_NAME"
echo ""
echo "To stop container:"
echo "  docker compose -f docker/docker-compose.worktree.yml down"
