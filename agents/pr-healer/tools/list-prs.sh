#!/usr/bin/env bash
set -euo pipefail

# List PRs that need attention
# Outputs JSON array of PRs with their issues
# Usage: list-prs.sh

# Query for open PRs
gh pr list \
    --state open \
    --json number,title,headRefName,headRepository,author,statusCheckRollup,reviewDecision,isDraft,mergeStateStatus \
    --jq '[.[] | select(.isDraft == false) | {
        number: .number,
        title: .title,
        headRefName: .headRefName,
        reviewDecision: (.reviewDecision // "NONE"),
        mergeStateStatus: (.mergeStateStatus // "UNKNOWN"),
        ciStatus: ([.statusCheckRollup[]?.conclusion] | join(",")),
        hasCiFailure: ([.statusCheckRollup[]?.conclusion] | any(. == "FAILURE")),
        needsAttention: (
            ([.statusCheckRollup[]?.conclusion] | any(. == "FAILURE")) or
            (.reviewDecision == "CHANGES_REQUESTED") or
            (.reviewDecision == "NEEDS_WORK") or
            (.mergeStateStatus == "BEHIND")
        )
    } | select(.needsAttention)]'
