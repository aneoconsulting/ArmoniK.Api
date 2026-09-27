#!/usr/bin/env bash
# A NARROWED RPC run (instrumentation) for comparing cells inside one client process: the
# campaign grid's client on chosen cells, directions and in-flight counts, several launches
# (each its own seeded cell order), one server per transport as opt_bench runs it. Then
# gen/rpc_pairs.py prints each framed cell (Xf) against its reference twin.
#
#   gen/rpc_narrow.sh OUT_DIR CELLS [DIRS] [INFLIGHT] [LAUNCHES] [ROUNDS] [CALLS]
#     CELLS     comma-separated cell names of the full build (the no-unknown client is not run)
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
[ $# -ge 2 ] || { echo "usage: $0 OUT_DIR CELLS [DIRS] [INFLIGHT] [LAUNCHES] [ROUNDS] [CALLS]" >&2; exit 2; }
mkdir -p "$1"; OUT="$(cd "$1" && pwd)"
CELLS=$2; DIRS=${3:-a,b}; KS=${4:-1}; LAUNCHES=${5:-3}; ROUNDS=${6:-10}; CALLS=${7:-48}
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1} AK_CPU_SERVER=${AK_CPU_SERVER:-2,3} AK_SERVER_THREADS=${AK_SERVER_THREADS:-2}
SCRATCH=$(mktemp -d); SP=""
cleanup() { [ -n "$SP" ] && kill "$SP" 2>/dev/null && wait "$SP" 2>/dev/null; rm -rf "$SCRATCH"; true; }
trap cleanup EXIT
LOG="$OUT/runner.log"; : > "$LOG"
echo "# rust slice NARROWED rpc run (gen/rpc_narrow.sh): CONTAINER INSTRUMENTATION; commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + UNCOMMITTED'); $(date -u +%FT%TZ); cells=$CELLS dirs=$DIRS inflight=$KS launches=$LAUNCHES rounds=$ROUNDS calls=$CALLS warmup=16 server-warm=16 server threads=$AK_SERVER_THREADS cpu client=$AK_CPU_CLIENT server=$AK_CPU_SERVER" | tee -a "$LOG"
cargo build --release -q -p campaign --bins 2>/dev/null
for T in shipped pinned; do
  SOCK="$SCRATCH/g-$T.sock"; RF="$SCRATCH/rf-$T"; rm -f "$SOCK" "$RF"
  taskset -c "$AK_CPU_SERVER" target/release/rpc_server --transport "$T" --socket "$SOCK" --ready-file "$RF" 2> "$OUT/server-$T.log" &
  SP=$!
  for _ in $(seq 100); do [ -s "$RF" ] && break; sleep 0.1; done
  for L in $(seq "$LAUNCHES"); do
    taskset -c "$AK_CPU_CLIENT" target/release/rpc_client --socket "$SOCK" --transport "$T" --launch "$L" \
      --rounds "$ROUNDS" --calls "$CALLS" --warmup 16 --server-warm 16 --cells "$CELLS" --dirs "$DIRS" --inflight "$KS" \
      --out "$OUT/rpc-$T-launch$L.jsonl" > "$OUT/rpc-$T-launch$L.log" 2>&1 || { echo "client FAILED ($T, launch $L)" | tee -a "$LOG"; exit 1; }
    echo "  $T launch $L: $(grep -vc '^#' "$OUT/rpc-$T-launch$L.jsonl") rows" | tee -a "$LOG"
  done
  kill $SP; wait $SP 2>/dev/null || true; SP=""
done
python3 gen/rpc_pairs.py "$OUT" | tee "$OUT/pairs.txt"
