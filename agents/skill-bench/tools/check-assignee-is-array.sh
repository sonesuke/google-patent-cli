#!/bin/bash
# Check if the assignee parameter was passed as an array
# Usage: check-assignee-is-array.sh <log_file>

LOG_FILE="$1"

if [ -z "$LOG_FILE" ]; then
    echo "[Error] Usage: $0 <log_file>" >&2
    exit 1
fi

# Check if assignee parameter is an array type in search_patents tool call
jq -s "[.[] | select(.type == \"assistant\") | .message.content[]? | select(type == \"object\" and .type == \"tool_use\") | select(.name | test(\"search_patents\"; \"i\")) | .input.assignee | type == \"array\"] | any" "$LOG_FILE"
