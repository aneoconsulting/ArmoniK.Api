#!/usr/bin/env bash
# The HTTP/2 connection setup each cell's client actually sends (CONTAINER INSTRUMENTATION):
# the shared server through serve.sh, the probe's client through gen/probe/h2sniff.py.
#   gen/probe/h2settings.sh OUT_DIR CELL...
set -euo pipefail
HERE=$(cd "$(dirname "$0")/../.." && pwd); cd "$HERE"
OUT=${1:?}; shift; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
S=$(mktemp -d); export AK_SERVE_STATE="$S/serve.state"
SN=""
cleanup() { [ -n "$SN" ] && kill "$SN" 2>/dev/null; wait 2>/dev/null; ./serve.sh stop > /dev/null 2>&1 || true; rm -rf "$S"; }
trap cleanup EXIT
o=$(AK_CPU_SERVER=${AK_CPU_SERVER:-2,3} ./serve.sh start --out "$S/serve"); SOCK=$(echo "$o" | sed -n 's/^pinned //p')
LOG="$OUT/h2-settings.log"; : > "$LOG"
python3 gen/probe/h2sniff.py "$S/sniff.sock" "$SOCK" "$LOG" & SN=$!
for _ in $(seq 50); do [ -S "$S/sniff.sock" ] && break; sleep 0.1; done
for c in "$@"; do
  echo "== cell $c" >> "$LOG"
  AK_RPC_SOCKET="$S/sniff.sock" AK_RPC_TRANSPORT=pinned AK_OUT="$S/x.jsonl" AK_PROBE_CELLS="$c" AK_PROBE_SIZES=4MiB \
    AK_PROBE_ROUNDS=1 AK_PROBE_CALLS=2 AK_PROBE_WARM=1 AK_PROBE_PROC=0 timeout 120 target/release/stream_probe
  sleep 0.5
done
cat "$LOG"
