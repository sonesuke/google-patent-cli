#!/bin/bash
# Check if a specific skill was invoked
# Usage: check-skill-invoked.sh <skill_name> <log_file>

SKILL_NAME="${1:-}"
LOG_FILE="${2:-}"

if [ -z "$LOG_FILE" ] || [ -z "$SKILL_NAME" ]; then
    echo "[Error] Usage: $0 <skill_name> <log_file>" >&2
    exit 1
fi

# Check if the skill was invoked in the log
grep -q '"Skill"' "$LOG_FILE" && grep -q '"skill":".*'"$SKILL_NAME" "$LOG_FILE"
