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

# Check if MCP tool was called using grep (more reliable than jq for mixed content)
# Search for tool name in the log file
TOOL_CALL_COUNT=$(grep -c "\"name.*${MCP_TOOL_NAME}\"" "$LOG_FILE" || echo 0)

if [[ $TOOL_CALL_COUNT -eq 0 ]]; then
  if [[ "$OPTIONAL_FLAG" == "--optional" ]]; then
    exit 0
  else
    echo "No $MCP_TOOL_NAME tool calls found in log" >&2
    exit 1
  fi
fi

# Check if any tool_results for the specified MCP tool have is_error: true
# Extract tool_use_ids from error results and check if they belong to our tool
while IFS= read -r error_line; do
  # Extract tool_use_id from error result
  TOOL_USE_ID=$(echo "$error_line" | grep -o '"tool_use_id":"[^"]*"' | cut -d'"' -f4)
  if [[ -n "$TOOL_USE_ID" ]]; then
    # Find the corresponding tool_use and check if it's our MCP tool
    TOOL_USE=$(grep -o "\"id\":\"$TOOL_USE_ID\"[^}]*\"name\":\"[^\"]*${MCP_TOOL_NAME}[^\"]*\"" "$LOG_FILE")
    if [[ -n "$TOOL_USE" ]]; then
      echo "MCP tool $MCP_TOOL_NAME (tool_use_id: $TOOL_USE_ID) returned an error" >&2
      exit 1
    fi
  fi
done < <(grep '"is_error":true' "$LOG_FILE")

exit 0
