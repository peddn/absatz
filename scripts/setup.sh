#!/usr/bin/env bash
# Installs the development tools. Idempotent and fast when everything exists.
# Runs automatically at the start of a Claude Code session (.claude/settings.json).
set -euo pipefail

TRUNK_VERSION=0.22.1
JUST_VERSION=1.59.0
BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$BIN"

fetch() { # url, binary name
  local tmp; tmp="$(mktemp -d)"
  curl -sSfL "$1" | tar xz -C "$tmp"
  install -m 755 "$(find "$tmp" -name "$2" -type f | head -1)" "$BIN/$2"
  rm -rf "$tmp"
}

# Rust toolchain incl. wasm target comes from rust-toolchain.toml.
(cd "$ROOT" && { rustup toolchain install >/dev/null 2>&1 || rustup show >/dev/null; })

if ! "$BIN/trunk" --version 2>/dev/null | grep -q "$TRUNK_VERSION"; then
  echo "installing trunk $TRUNK_VERSION"
  fetch "https://github.com/trunk-rs/trunk/releases/download/v$TRUNK_VERSION/trunk-x86_64-unknown-linux-gnu.tar.gz" trunk
fi

if ! "$BIN/just" --version 2>/dev/null | grep -q "$JUST_VERSION"; then
  echo "installing just $JUST_VERSION"
  fetch "https://github.com/casey/just/releases/download/$JUST_VERSION/just-$JUST_VERSION-x86_64-unknown-linux-musl.tar.gz" just
fi

if [ ! -d "$ROOT/e2e/node_modules" ]; then
  echo "installing e2e dependencies"
  (cd "$ROOT/e2e" && npm ci --silent)
fi

# Browser for Playwright: CI installs Playwright's Chromium; a Claude Code
# session reuses an installed Chrome; otherwise install Playwright's Chromium.
if [ -n "${CI:-}" ]; then
  (cd "$ROOT/e2e" && npx playwright install --with-deps chromium)
elif [ -n "${CHROME_PATH:-}" ]; then
  :
elif [ -n "${CLAUDE_ENV_FILE:-}" ] && command -v google-chrome >/dev/null; then
  echo "export CHROME_PATH=$(command -v google-chrome)" >> "$CLAUDE_ENV_FILE"
else
  (cd "$ROOT/e2e" && npx playwright install chromium)
fi

echo "setup ok: $(trunk --version), $(just --version)"
