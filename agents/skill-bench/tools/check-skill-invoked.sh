#!/bin/bash
# Check if a specific skill was invoked
# Usage: check-skill-invoked.sh <log_file> <skill_name>

LOG_FILE="${1:-}"
SKILL_NAME="${2:-}"

if [ -z "$LOG_FILE" ] || [ -z "$SKILL_NAME" ]; then
    echo "[Error] Usage: $0 <log_file> <skill_name>" >&2
    exit 1
fi

# Check if the skill was invoked in the log
grep -q '"Skill"' "$LOG_FILE" && grep -q '"skill":".*'"$SKILL_NAME" "$LOG_FILE"
