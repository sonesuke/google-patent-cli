#!/bin/bash
# Check if the final output does NOT contain a specific pattern
# Usage: check-output-not-contains.sh <log_file> <pattern>
#
# This checks the final result text, not intermediate tool calls

LOG_FILE="${1:-}"
PATTERN="${2:-}"

if [ -z "$LOG_FILE" ] || [ -z "$PATTERN" ]; then
    echo "[Error] Usage: $0 <log_file> <pattern>" >&2
    exit 1
fi

# Extract the final result text and check if it does NOT contain the pattern
# The result is in a "result" type message with a "result" field
! jq -r 'select(.type == "result") | .result' "$LOG_FILE" 2>/dev/null | grep -q "$PATTERN"
