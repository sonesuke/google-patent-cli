#!/bin/bash
set -e

docker run -d \
  --name google-patent-cli \
  -v "$(pwd):/workspaces/google-patent-cli" \
  -v "${HOME}/.config/gh:/home/user/.config/gh" \
  -e Z_AI_API_KEY="${Z_AI_API_KEY}" \
  -e CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1 \
  google-patent-cli:latest \
  sleep infinity
