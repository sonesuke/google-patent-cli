#!/bin/bash
# Check if output_file was created in log
# Usage: check-output-file.sh <log_file>

LOG_FILE="$1"

if [ -z "$LOG_FILE" ]; then
    echo "[Error] Usage: $0 <log_file>" >&2
    exit 1
fi

# Check if output_file exists in tool_result content
jq -s '[.[] | select(.type == "user") | .message.content[]? | select(type == "object" and .type == "tool_result" and .tool_use_id? and .content? != null) | .content | fromjson | .output_file] | length > 0' "$LOG_FILE"
