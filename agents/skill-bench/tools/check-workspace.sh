#!/bin/bash
# Check if workspace condition is true
# Usage: check-workspace.sh <work_dir> <command>

WORK_DIR="$1"
CHECK_CMD="$2"

if [ -z "$WORK_DIR" ] || [ -z "$CHECK_CMD" ]; then
    echo "[Error] Usage: $0 <work_dir> <command>" >&2
    exit 1
fi

(cd "$WORK_DIR" && eval "$CHECK_CMD")
