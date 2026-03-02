#!/bin/bash
# Check if MCP server loaded successfully in a log file
# Usage: check-mcp-loaded.sh <log_file> <mcp_server_name>
# Returns: 0 if MCP server loaded successfully, 1 if failed or not found

LOG_FILE="$1"
MCP_SERVER_NAME="$2"

if [[ -z "$LOG_FILE" ]] || [[ -z "$MCP_SERVER_NAME" ]]; then
  echo "Usage: $0 <log_file> <mcp_server_name>" >&2
  exit 2
fi

if [[ ! -f "$LOG_FILE" ]]; then
  echo "Log file not found: $LOG_FILE" >&2
  exit 2
fi

# Check MCP server status in init message (first line is init message)
# The mcp_servers array contains objects with name and status fields
STATUS=$(head -1 "$LOG_FILE" | jq -r '
  .mcp_servers? // []
  | .[] | select(.name? | test("'"$MCP_SERVER_NAME"'"))
  | .status // "not_found"
')

if [[ "$STATUS" == "not_found" ]]; then
  echo "MCP server $MCP_SERVER_NAME not found in log" >&2
  exit 1
fi

if [[ "$STATUS" == "failed" ]]; then
  echo "MCP server $MCP_SERVER_NAME failed to load (status: failed)" >&2
  exit 1
fi

exit 0
