#!/bin/bash
# Create an isolated git worktree for Claude Code to work on
# Usage: ./docker/scripts/create-worktree.sh [branch-name]
#
# This creates a worktree at ../lintal-claude-worktree
# which can be safely mounted into Docker without risking your main checkout

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"
BRANCH_NAME="${1:-claude-work}"
WORKTREE_DIR="$(dirname "$PROJECT_DIR")/lintal-claude-worktree"

cd "$PROJECT_DIR"

# Check if worktree already exists
if [ -d "$WORKTREE_DIR" ]; then
    echo "Worktree already exists at: $WORKTREE_DIR"
    echo ""
    echo "To remove and recreate:"
    echo "  git worktree remove $WORKTREE_DIR"
    echo "  $0 $BRANCH_NAME"
    exit 1
fi

# Create branch if it doesn't exist (from current HEAD)
if ! git show-ref --verify --quiet "refs/heads/$BRANCH_NAME"; then
    echo "Creating new branch: $BRANCH_NAME"
    git branch "$BRANCH_NAME"
fi

# Create the worktree
echo "Creating worktree at: $WORKTREE_DIR"
git worktree add "$WORKTREE_DIR" "$BRANCH_NAME"

echo ""
echo "Worktree created successfully!"
echo ""
echo "Location: $WORKTREE_DIR"
echo "Branch:   $BRANCH_NAME"
echo ""
echo "Now start Docker with the worktree mounted:"
echo "  WORKTREE_PATH=$WORKTREE_DIR docker compose -f docker/docker-compose.worktree.yml up -d --build"
echo ""
echo "Or use the convenience script:"
echo "  ./docker/start-isolated.sh"
echo ""
echo "When done, review changes in the worktree:"
echo "  cd $WORKTREE_DIR"
echo "  git status"
echo "  git diff"
echo ""
echo "To merge good changes back to main:"
echo "  cd $PROJECT_DIR"
echo "  git merge $BRANCH_NAME"
echo ""
echo "To discard everything:"
echo "  git worktree remove $WORKTREE_DIR"
echo "  git branch -D $BRANCH_NAME"
