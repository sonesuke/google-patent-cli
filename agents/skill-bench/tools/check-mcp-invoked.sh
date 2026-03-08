#!/bin/bash
# Check if MCP tool was invoked (regardless of success or error)
# Usage: check-mcp-invoked.sh <log_file> <mcp_tool_name>
# Returns: 0 if tool was invoked, 1 if not

LOG_FILE="$1"
MCP_TOOL_NAME="$2"

if [[ -z "$LOG_FILE" ]] || [[ -z "$MCP_TOOL_NAME" ]]; then
  echo "Usage: $0 <log_file> <mcp_tool_name>" >&2
  exit 2
fi

if [[ ! -f "$LOG_FILE" ]]; then
  echo "Log file not found: $LOG_FILE" >&2
  exit 2
fi

# Check if MCP tool was called using grep
TOOL_CALL_COUNT=$(grep -c "\"name.*${MCP_TOOL_NAME}\"" "$LOG_FILE" || echo 0)

if [[ $TOOL_CALL_COUNT -gt 0 ]]; then
  exit 0
else
  echo "No $MCP_TOOL_NAME tool calls found in log" >&2
  exit 1
fi
