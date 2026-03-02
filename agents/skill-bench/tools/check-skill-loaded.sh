#!/bin/bash
# Check if a skill was loaded successfully in a log file
# Usage: check-skill-loaded.sh <log_file> <skill_name>
# Returns: 0 if skill found in init skills array, 1 if not found

LOG_FILE="$1"
SKILL_NAME="$2"

if [[ -z "$LOG_FILE" ]] || [[ -z "$SKILL_NAME" ]]; then
  echo "Usage: $0 <log_file> <skill_name>" >&2
  exit 2
fi

if [[ ! -f "$LOG_FILE" ]]; then
  echo "Log file not found: $LOG_FILE" >&2
  exit 2
fi

# Check if skill is in the init skills array (first line is init message)
FOUND=$(head -1 "$LOG_FILE" | jq -c '
  .skills | any(.[]; contains("'$SKILL_NAME'"))
')

if [[ "$FOUND" != "true" ]]; then
  echo "Skill $SKILL_NAME not found in init skills array" >&2
  exit 1
fi

exit 0
