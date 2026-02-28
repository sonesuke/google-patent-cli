#!/usr/bin/env bash
set -euo pipefail

# pr-healer: An autonomous agent that monitors and fixes failing PR CI checks
# This script runs entirely inside the devcontainer

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROGRESS_FILE="$REPO_ROOT/agents/pr-healer/progress.jsonl"

# Check gh auth
if ! gh auth status &> /dev/null; then
    echo "Error: GitHub CLI not authenticated. Run 'gh auth login'"
    exit 1
fi

# Calculate sleep duration based on recent activity
calculate_sleep() {
    if [[ ! -f "$PROGRESS_FILE" ]]; then
        echo 300  # 5 minutes if no history
        return
    fi

    # Get the last record's timestamp
    LAST_RECORD=$(tail -1 "$PROGRESS_FILE" 2>/dev/null || echo "")
    if [[ -z "$LAST_RECORD" ]]; then
        echo 300
        return
    fi

    LAST_TIMESTAMP=$(echo "$LAST_RECORD" | jq -r '.timestamp')
    if [[ -z "$LAST_TIMESTAMP" ]] || [[ "$LAST_TIMESTAMP" == "null" ]]; then
        echo 300
        return
    fi

    # Convert to epoch seconds
    LAST_EPOCH=$(date -d "$LAST_TIMESTAMP" +%s 2>/dev/null || echo "0")
    CURRENT_EPOCH=$(date +%s)
    DIFF=$((CURRENT_EPOCH - LAST_EPOCH))

    # If last activity was within 5 minutes, sleep 1 minute
    # Otherwise sleep 5 minutes
    if [[ $DIFF -lt 300 ]]; then
        echo 60
    else
        echo 300
    fi
}

# Main loop
while true; do
    echo "===== pr-healer: Looking for PRs needing attention ====="

    # Run Claude with worktree
    echo "Running Claude..."
    PROMPT_FILE="$REPO_ROOT/agents/pr-healer/prompt.txt"
    TEMP_ERR=$(mktemp)

    if claude -p \
        --worktree \
        --dangerously-skip-permissions \
        --verbose \
        --output-format stream-json \
        "$(cat "$PROMPT_FILE")" < /dev/null 2>"$TEMP_ERR" | jq . ; then
        rm -f "$TEMP_ERR"
    else
        cat "$TEMP_ERR" >&2
        rm -f "$TEMP_ERR"
        echo "Claude execution failed"
    fi

    # Calculate sleep duration based on activity
    SLEEP_DURATION=$(calculate_sleep)
    SLEEP_MINUTES=$((SLEEP_DURATION / 60))

    echo "----- Cycle complete -----"
    echo "Sleeping for ${SLEEP_MINUTES} minute(s)..."
    sleep "$SLEEP_DURATION"
done

echo "pr-healer: Done"
