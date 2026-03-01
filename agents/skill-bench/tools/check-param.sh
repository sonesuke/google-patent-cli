#!/bin/bash
# Check if parameter was used in tool call
# Usage: check-param.sh <log_file> <tool_name> <param_name> <expected_value>

LOG_FILE="$1"
TOOL_NAME="$2"
PARAM_NAME="$3"
EXPECTED_VALUE="$4"

if [ -z "$LOG_FILE" ] || [ -z "$TOOL_NAME" ] || [ -z "$PARAM_NAME" ]; then
    echo "[Error] Usage: $0 <log_file> <tool_name> <param_name> [expected_value]" >&2
    exit 1
fi

if [ -n "$EXPECTED_VALUE" ]; then
    # Check if parameter equals expected value
    jq -s "[.[] | select(.type == \"assistant\") | .message.content[]? | select(type == \"object\" and .type == \"tool_use\" and .name == \"$TOOL_NAME\") | .input.$PARAM_NAME == \"$EXPECTED_VALUE\"] | any" "$LOG_FILE"
else
    # Check if parameter exists
    jq -s "[.[] | select(.type == \"assistant\") | .message.content[]? | select(type == \"object\" and .type == \"tool_use\" and .name == \"$TOOL_NAME\") | .input.$PARAM_NAME] | length > 0" "$LOG_FILE"
fi
