#!/bin/bash
# Export uncommitted changes from the workspace as patch files to /patches directory
# These can then be applied on the host machine for review

set -e

WORKSPACE="${1:-/home/developer/workspace/lintal}"
PATCHES_DIR="/home/developer/patches"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

if [ ! -d "$WORKSPACE/.git" ]; then
    echo "Error: $WORKSPACE is not a git repository"
    exit 1
fi

cd "$WORKSPACE"

# Check if there are any changes
if git diff --quiet && git diff --cached --quiet; then
    echo "No changes to export"
    exit 0
fi

# Create patch filename
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "detached")
PATCH_FILE="$PATCHES_DIR/changes-${BRANCH}-${TIMESTAMP}.patch"

# Export all changes (staged and unstaged)
echo "Exporting changes to: $PATCH_FILE"
git diff HEAD > "$PATCH_FILE"

# Also create a summary of changed files
SUMMARY_FILE="$PATCHES_DIR/changes-${BRANCH}-${TIMESTAMP}.summary"
echo "Changes Summary - $(date)" > "$SUMMARY_FILE"
echo "========================" >> "$SUMMARY_FILE"
echo "" >> "$SUMMARY_FILE"
echo "Branch: $BRANCH" >> "$SUMMARY_FILE"
echo "Base commit: $(git rev-parse HEAD)" >> "$SUMMARY_FILE"
echo "" >> "$SUMMARY_FILE"
echo "Files changed:" >> "$SUMMARY_FILE"
git diff --stat HEAD >> "$SUMMARY_FILE"

echo ""
echo "Created files:"
echo "  Patch:   $PATCH_FILE"
echo "  Summary: $SUMMARY_FILE"
echo ""
echo "To apply on host:"
echo "  cd /path/to/lintal"
echo "  git apply /path/to/patches/$(basename $PATCH_FILE)"
