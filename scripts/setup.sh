#!/bin/bash
set -e

# Configure git using GitHub noreply email and credential helper
if command -v gh >/dev/null 2>&1 && gh auth status &>/dev/null; then
  gh auth setup-git
  GH_USER=$(gh api user --jq .login)
  GH_ID=$(gh api user --jq .id)
  git config --global user.name "$GH_USER"
  git config --global user.email "${GH_ID}+${GH_USER}@users.noreply.github.com"
  echo "Git configured as $GH_USER (noreply email)"
else
  echo "Warning: GitHub CLI not authenticated, skipping git config"
fi

# Install Rust via rustup
if ! command -v rustup >/dev/null 2>&1; then
  echo "Installing Rust..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-tool stable --profile minimal
  export PATH="$HOME/.cargo/bin:$PATH"
  source "$HOME/.cargo/env"
else
  echo "Rust already installed: $(rustc --version)"
fi

# Add Rust components and tools
echo "Setting up Rust toolchain..."
rustup component add rustfmt clippy llvm-tools-preview
curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
cargo binstall -y cargo-audit cargo-llvm-cov

# Install Claude CLI
if ! command -v claude >/dev/null 2>&1; then
  echo "Installing Claude CLI..."
  curl -fsSL https://claude.ai/install.sh | bash
  export PATH="$HOME/.local/bin:$PATH"
else
  echo "Claude CLI already installed: $(claude --version)"
fi

# Configure Claude
if [ -n "$Z_AI_API_KEY" ]; then
  mkdir -p "$HOME/.claude"
  cat > "$HOME/.claude/settings.json" <<EOF
{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "$Z_AI_API_KEY",
    "ANTHROPIC_BASE_URL": "https://api.z.ai/api/anthropic",
    "API_TIMEOUT_MS": "3000000",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "glm-5.1",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "glm-5-turbo",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "glm-4.5-air"
  }
}
EOF
fi

# Configure zsh
AUTOSUGGESTIONS=$(find / -path "*/zsh-autosuggestions/zsh-autosuggestions.zsh" 2>/dev/null | head -1)
SYNTAX_HIGHLIGHTING=$(find / -path "*/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh" 2>/dev/null | head -1)

cat > "$HOME/.zshrc" <<OUTER
export PATH="\$HOME/.local/bin:\$HOME/.cargo/bin:\$PATH"
alias claude="claude --allow-dangerously-skip-permissions"
eval "\$(mise activate zsh)"

# Zsh plugins
${AUTOSUGGESTIONS:+source ${AUTOSUGGESTIONS}}
${SYNTAX_HIGHLIGHTING:+source ${SYNTAX_HIGHLIGHTING}}

# Prompt
setopt PROMPT_SUBST
parse_git_branch() {
  local branch
  branch=\$(git symbolic-ref --short HEAD 2>/dev/null) || return
  echo " (\$branch)"
}
PROMPT='%F{blue}%~%f%F{yellow}\$(parse_git_branch)%f
%F{green}❯%f '
OUTER

# Install mise
if ! command -v mise >/dev/null 2>&1; then
  echo "Installing mise..."
  curl -fsSL https://mise.run | bash
  export PATH="$HOME/.local/bin:$PATH"
else
  echo "mise already installed: $(mise --version)"
fi

cd /workspaces/google-patent-cli
mise trust
mise install
mise generate git-pre-commit

# Install skill-bench
echo "Installing skill-bench..."
curl -fsSL https://raw.githubusercontent.com/sonesuke/skill-bench/main/scripts/setup.sh | sh

# Configure google-patent-cli for Docker
mkdir -p "$HOME/.config/google-patent-cli"
cat > "$HOME/.config/google-patent-cli/config.toml" << 'EOF'
# Chrome browser path
browser_path = "/bin/chromium"

# Chrome arguments for Docker environment
chrome_args = [
    "--no-sandbox",
    "--disable-setuid-sandbox",
    "--disable-gpu"
]
EOF

# Configure gh auth for git
if command -v gh >/dev/null 2>&1; then
  echo "Configuring gh auth for git..."
  gh auth setup-git
fi

echo "Setup completed."
