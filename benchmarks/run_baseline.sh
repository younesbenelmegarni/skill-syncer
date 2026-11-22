#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/release/skill-sync"
PY="${PYTHON:-python3}"
RESULTS="$ROOT/benchmarks/results"
STAMP="$(date +%Y%m%d)"
OUT="$RESULTS/${STAMP}-rust-baseline.json"
mkdir -p "$RESULTS" "$ROOT/benchmarks/fixtures"
if [[ ! -x "$BIN" ]]; then
  (cd "$ROOT" && cargo build --release)
fi

run_case() {
  local name="$1"
  local skills="$2"
  local home="$ROOT/benchmarks/fixtures/${name}"
  "$PY" "$ROOT/benchmarks/generate_fixture.py" "$home" --skills "$skills"
  local tmp
  tmp="$(mktemp)"
  if command -v hyperfine >/dev/null 2>&1; then
    hyperfine --warmup 1 --min-runs 3 \
      --export-json "$tmp" \
      --command-name "dry-run" \
      "$BIN --home $home dry-run --json" \
      --command-name "sync-first" \
      "$BIN --home $home sync --json" \
      --command-name "sync-idempotent" \
      "$BIN --home $home dry-run --json"
  else
    echo '{"note":"hyperfine not installed"}' >"$tmp"
    /usr/bin/time -l "$BIN" --home "$home" dry-run --json >/dev/null 2>"$home.time.txt" || true
  fi
  "$PY" - <<PY
import json, pathlib, platform
out = pathlib.Path("$OUT")
data = json.loads(out.read_text()) if out.exists() else {"runs": [], "environment": {}}
data.setdefault("environment", {})
data["environment"].update({
    "platform": platform.platform(),
    "cli": "$BIN",
})
entry = {"fixture": "$name", "skills": $skills, "home": "$home", "hyperfine": json.loads(pathlib.Path("$tmp").read_text())}
data["runs"].append(entry)
out.write_text(json.dumps(data, indent=2) + "\\n")
PY
}

data='{"runs":[],"environment":{}}'
echo "$data" >"$OUT"
run_case "skills-100" 100
run_case "skills-1000" 1000
if [[ "${SKIP_10K:-}" != "1" ]]; then
  run_case "skills-10000" 10000
fi
echo "Wrote $OUT"
