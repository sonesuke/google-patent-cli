#!/bin/bash
# Check if MCP tool calls succeeded in a log file
# Usage: check-mcp-success.sh <log_file> <mcp_tool_name> [--optional]
# Returns: 0 if all MCP calls succeeded (or none made with --optional), 1 if any failed

LOG_FILE="$1"
MCP_TOOL_NAME="$2"
OPTIONAL_FLAG="${3:-}"

if [[ -z "$LOG_FILE" ]] || [[ -z "$MCP_TOOL_NAME" ]]; then
  echo "Usage: $0 <log_file> <mcp_tool_name> [--optional]" >&2
  exit 2
fi

if [[ ! -f "$LOG_FILE" ]]; then
  echo "Log file not found: $LOG_FILE" >&2
  exit 2
fi

# Extract tool_use IDs for the specified MCP tool
TOOL_USE_IDS=$(jq -r '
  .[]
  | select(.type? == "assistant")
  | (.message.content? // [])
  | select(type == "array")
  | .[]
  | select(type == "object" and .type? == "tool_use" and (.name? // "") | test("'"$MCP_TOOL_NAME"'"))
  | .id
' "$LOG_FILE")

ID_COUNT=$(echo "$TOOL_USE_IDS" | grep -c '^\w*$' || true)

if [[ $ID_COUNT -eq 0 ]]; then
  if [[ "$OPTIONAL_FLAG" == "--optional" ]]; then
    exit 0
  else
    echo "No $MCP_TOOL_NAME tool calls found in log" >&2
    exit 1
  fi
fi

# Check if any tool_results have is_error: true
while IFS= read -r tool_id; do
  if [[ -n "$tool_id" ]]; then
    ERROR_CHECK=$(jq -r "
      .[]
      | select(.type? == \"user\")
      | (.message.content? // [])
      | select(type == \"array\")
      | .[]
      | select(type == \"object\" and .type? == \"tool_result\" and .tool_use_id? == \"$tool_id\")
      | .is_error // false
    " "$LOG_FILE")

    if [[ "$ERROR_CHECK" == "true" ]]; then
      echo "MCP tool $MCP_TOOL_NAME (tool_use_id: $tool_id) returned an error" >&2
      exit 1
    fi
  fi
done <<< "$TOOL_USE_IDS"

exit 0
