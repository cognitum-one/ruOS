#!/bin/sh
# Install Claude Code (Anthropic's CLI for Claude)
# This is the agentic interface — the reason ruOS exists
#
# Gracefully fails if no internet (offline installs skip it).
# POSIX sh compatible.

set -eu

echo "==> Installing Claude Code..."

# Method 1: npm (if Node.js is available)
if command -v npm >/dev/null 2>&1; then
  if npm install -g @anthropic-ai/claude-code 2>/dev/null; then
    echo "  Installed via npm"
    exit 0
  fi
fi

# Method 2: Direct binary (no Node.js needed)
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  CLAUDE_ARCH="x64" ;;
  aarch64) CLAUDE_ARCH="arm64" ;;
  *)       echo "  Unsupported architecture: $ARCH"; exit 1 ;;
esac

CLAUDE_DIR="$HOME/.local/bin"
mkdir -p "$CLAUDE_DIR"

if command -v curl >/dev/null 2>&1; then
  if curl -fsSL --connect-timeout 10 \
    "https://github.com/anthropics/claude-code/releases/latest/download/claude-code-linux-${CLAUDE_ARCH}" \
    -o "$CLAUDE_DIR/claude" 2>/dev/null; then
    chmod +x "$CLAUDE_DIR/claude"
    echo "  Installed binary to $CLAUDE_DIR/claude"
    exit 0
  fi
fi

# Fallback: check if already installed
if command -v claude >/dev/null 2>&1; then
  echo "  Claude Code already installed: $(claude --version 2>/dev/null || echo 'present')"
  exit 0
fi

echo "  Claude Code not installed — install manually: npm i -g @anthropic-ai/claude-code"
echo "  Or download from: https://claude.ai/code"
exit 0
