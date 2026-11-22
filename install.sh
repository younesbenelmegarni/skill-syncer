#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
cargo build --release
BIN="$ROOT/target/release/skill-sync"
install -d "${HOME}/.local/bin"
install -m 755 "$BIN" "${HOME}/.local/bin/skill-sync"
if [ -d /opt/homebrew/bin ]; then
  install -m 755 "$BIN" /opt/homebrew/bin/skill-sync
fi
echo "Installed skill-sync to ~/.local/bin/skill-sync"
command -v skill-sync >/dev/null && skill-sync --help | head -n1
