#!/usr/bin/env bash
set -euo pipefail

# Get main worktree path (works from any worktree)
GIT_COMMON_DIR=$(git rev-parse --git-common-dir)
MAIN_WORKTREE=$(dirname "$GIT_COMMON_DIR")
PROGRESS_FILE="$MAIN_WORKTREE/agents/pr-healer/progress.jsonl"

# Ensure directory exists
mkdir -p "$(dirname "$PROGRESS_FILE")"

# Parse arguments
PR_NUMBER="${1:-}"
BRANCH_NAME="${2:-}"
TIMESTAMP="${3:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"
STATUS="${4:-unknown}"

if [[ -z "$PR_NUMBER" ]]; then
    echo "Usage: $0 <PR_NUMBER> <BRANCH_NAME> [TIMESTAMP] [STATUS]" >&2
    exit 1
fi

# Create JSON record
RECORD=$(jq -n \
    --arg pr "$PR_NUMBER" \
    --arg branch "$BRANCH_NAME" \
    --arg timestamp "$TIMESTAMP" \
    --arg status "$STATUS" \
    '{pr: $pr, branch: $branch, timestamp: $timestamp, status: $status}')

# Append to progress file
echo "$RECORD" >> "$PROGRESS_FILE"

echo "✅ Progress recorded: PR #$PR_NUMBER ($BRANCH_NAME) - $STATUS"

# Show recent count
RECENT=$(wc -l < "$PROGRESS_FILE" 2>/dev/null || echo "0")
echo "   Total records: $RECENT"
