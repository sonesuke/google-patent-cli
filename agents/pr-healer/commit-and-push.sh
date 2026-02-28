#!/usr/bin/env bash
set -euo pipefail

# Commit and push changes to a PR
# Usage: commit-and-push.sh <PR_NUMBER> <BRANCH_NAME>
# Note: This script assumes it's run from within the worktree

PR_NUMBER="${1:-}"
BRANCH_NAME="${2:-}"

if [[ -z "$PR_NUMBER" ]] || [[ -z "$BRANCH_NAME" ]]; then
    echo "Usage: $0 <PR_NUMBER> <BRANCH_NAME>" >&2
    exit 1
fi

# Check if changes were made
if git diff --quiet; then
    echo "No changes to commit"
    exit 0
fi

echo "Changes detected - committing and pushing..."

# Commit changes
git commit -m "ci: fix failing checks

- Automatically fix failing CI checks
- Run cargo test to verify changes
- Address formatting and linting issues"

# Push changes
git push origin "$BRANCH_NAME"

# Comment on PR
gh pr comment "$PR_NUMBER" --body "🤖 pr-healer: Automatically fixed failing CI checks. Please review the changes."

echo "Changes committed and pushed successfully"
