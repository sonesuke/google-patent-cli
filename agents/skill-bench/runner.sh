#!/bin/bash
# agents/skill-bench/runner.sh
# Skill test runner for google-patent-cli.
# All execution happens inside the container.
#
# Usage: ./runner.sh [pattern]
#   pattern:  Glob pattern to match test files (default: "cases/*/*.toml")

set -o pipefail

# Determine workspace root
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# Determine skill-bench root
SKILL_BENCH_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Resolve pattern relative to skill-bench root
PATTERN="${1:-cases/*/*.toml}"
# Convert to absolute path
if [[ "$PATTERN" != /* ]]; then
    TARGET_PATTERN="$SKILL_BENCH_ROOT/$PATTERN"
else
    TARGET_PATTERN="$PATTERN"
fi

echo "=================================================="
echo "[SkillBench] Starting Skill Test Runner"
echo "[SkillBench] Workspace: $WORKSPACE_ROOT"
echo "[SkillBench] Pattern: $TARGET_PATTERN"
echo "=================================================="

TOTAL_CASES=0
TOTAL_PASS=0
TOTAL_FAIL=0

# --- Collect test files matching pattern ---
TEST_FILES=()
TEST_SKILLS=()
TEST_NAMES=()

for TEST_FILE in $TARGET_PATTERN; do
    [ -f "$TEST_FILE" ] || continue

    TEST_FILE_REL="${TEST_FILE#$WORKSPACE_ROOT/}"
    SKILL_NAME=$(basename "$(dirname "$TEST_FILE_REL")")
    TEST_NAME=$(basename "$TEST_FILE" .toml)

    TEST_FILES+=("$TEST_FILE")
    TEST_SKILLS+=("$SKILL_NAME")
    TEST_NAMES+=("$TEST_NAME")
done

if [ ${#TEST_FILES[@]} -eq 0 ]; then
    echo "[SkillBench] No test files found matching pattern: $TARGET_PATTERN"
    exit 1
fi

# --- Process each test file ---
for IDX in "${!TEST_FILES[@]}"; do
    TEST_FILE="${TEST_FILES[$IDX]}"
    SKILL_NAME="${TEST_SKILLS[$IDX]}"
    TEST_NAME="${TEST_NAMES[$IDX]}"
    TEST_CASE_NAME="${SKILL_NAME}/${TEST_NAME}"
    TOTAL_CASES=$((TOTAL_CASES + 1))

    # Read test configuration
    TEST_PROMPT=$(yq eval '.test_prompt' "$TEST_FILE")
    TEST_TIMEOUT=$(yq eval '.timeout // 300' "$TEST_FILE")

    echo ""
    echo "──────────────────────────────────────────────────"
    echo "[SkillBench] Test Case: $TEST_CASE_NAME"
    echo "──────────────────────────────────────────────────"

    # --- Phase 1: Setup and Execute trial ---
    # Create skill-specific log directory
    LOG_DIR="$SKILL_BENCH_ROOT/logs/${SKILL_NAME}"
    mkdir -p "$LOG_DIR"

    TIMESTAMP=$(date +%Y%m%d_%H%M%S)
    LOG_FILE="$LOG_DIR/${TIMESTAMP}_${TEST_NAME}.log"
    WORK_DIR="/tmp/skill-bench-${TIMESTAMP}_${SKILL_NAME}-${TEST_NAME}"

    # Setup workspace
    echo "[SkillBench]   📦 Setting up workspace: $WORK_DIR"
    rm -rf "${WORK_DIR}"
    mkdir -p "${WORK_DIR}"

    # Copy claude-plugin (required for skill testing)
    cp -r "$WORKSPACE_ROOT/claude-plugin" "$WORK_DIR/" 2>/dev/null || true

    # Read setup files from test.toml [[setup]] array
    NUM_SETUP=$(yq eval '.setup | length // 0' "$TEST_FILE")
    if [ "$NUM_SETUP" -gt 0 ]; then
        for SETUP_IDX in $(seq 0 $((NUM_SETUP - 1))); do
            SETUP_PATH=$(yq eval ".setup[$SETUP_IDX].path" "$TEST_FILE")
            SETUP_DIR=$(dirname "$WORK_DIR/$SETUP_PATH")
            mkdir -p "$SETUP_DIR"
            yq eval ".setup[$SETUP_IDX].content" "$TEST_FILE" > "$WORK_DIR/${SETUP_PATH}"
        done
    fi

    # Execute trial
    echo "[SkillBench]   Running trial → $LOG_FILE"
    START_TIME=$(date +%s)

    (cd "$WORK_DIR" && claude -p \
        --dangerously-skip-permissions \
        --verbose \
        --output-format stream-json \
        --plugin-dir ./claude-plugin \
        -- "$TEST_PROMPT" < /dev/null | jq -c '(. + {timestamp: now})') > "$LOG_FILE" 2>&1

    EXIT_CODE=$?
    END_TIME=$(date +%s)
    DURATION=$(( END_TIME - START_TIME ))

    if [ $EXIT_CODE -eq 0 ]; then
        echo "[SkillBench]   ✅ Trial finished (took ${DURATION}s)"
    else
        echo "[SkillBench]   ⚠️  Trial exited with code $EXIT_CODE (took ${DURATION}s)"
    fi

    # --- Phase 2: Evaluate trial ---
    echo "[SkillBench]   Running evaluation..."

    CASE_PASS=true
    TOOLS_DIR="$(cd "$(dirname "$0")" && pwd)/tools"

    # Run checks from test.toml
    NUM_CHECKS=$(yq eval '.checks | length' "$TEST_FILE")
    for CHECK_IDX in $(seq 0 $((NUM_CHECKS - 1))); do
        CHECK_NAME=$(yq eval ".checks[$CHECK_IDX].name" "$TEST_FILE")
        CHECK_CMD=$(yq eval ".checks[$CHECK_IDX].command" "$TEST_FILE")

        if $TOOLS_DIR/$CHECK_CMD "$LOG_FILE" >/dev/null 2>&1; then
            echo "[SkillBench]     ✅ $CHECK_NAME"
        else
            echo "[SkillBench]     ❌ $CHECK_NAME"
            CASE_PASS=false
        fi
    done

    # Display case result
    if [ "$CASE_PASS" = true ]; then
        echo "[SkillBench]   ✅ $TEST_CASE_NAME: PASS"
        TOTAL_PASS=$((TOTAL_PASS + 1))
    else
        echo "[SkillBench]   ❌ $TEST_CASE_NAME: FAIL"
        TOTAL_FAIL=$((TOTAL_FAIL + 1))
    fi
done

# --- Summary ---
echo ""
echo "=================================================="
echo "[SkillBench] Test Summary"
echo "[SkillBench] Total: $TOTAL_CASES | Pass: $TOTAL_PASS | Fail: $TOTAL_FAIL"
echo "=================================================="

exit "$TOTAL_FAIL"
