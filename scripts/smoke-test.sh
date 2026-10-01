#!/usr/bin/env bash
# Smoke-tests a built binary: --version must report the Cargo version, and the GUI must
# stay up for a few seconds without crashing. Usage: smoke-test.sh <binary> [--no-gui]
set -euo pipefail
BIN="$1"
EXPECTED="OpenBinauralBeats $(grep -m1 '^version' "$(dirname "$0")/../Cargo.toml" | cut -d'"' -f2)"

OUT=$("$BIN" --version)
if [[ "$OUT" != "$EXPECTED" ]]; then
  echo "version mismatch: got '$OUT', expected '$EXPECTED'" >&2
  exit 1
fi
echo "ok: $OUT"

[[ "${2:-}" == "--no-gui" ]] && exit 0

"$BIN" &
PID=$!
sleep 5
if ! kill -0 "$PID" 2>/dev/null; then
  wait "$PID" || true
  echo "app exited during startup" >&2
  exit 1
fi
kill "$PID"
echo "ok: GUI started and stayed up for 5s"
