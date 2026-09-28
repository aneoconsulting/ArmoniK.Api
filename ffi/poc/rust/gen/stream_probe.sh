#!/usr/bin/env bash
# The stream probe (direction d only, k = 1, one client process, chosen cells): CONTAINER
# INSTRUMENTATION. The shared server through serve.sh (AK_SERVER_THREADS workers pinned to
# AK_CPU_SERVER), the client (bin stream_probe) pinned to AK_CPU_CLIENT.
#
#   gen/stream_probe.sh OUT_DIR [NAME]      -> OUT_DIR/NAME.jsonl, NAME.txt (summary)
# Environment: AK_PROBE_CELLS / SIZES / ROUNDS / CALLS / WARM (bin stream_probe),
#   AK_PROBE_SHIM=1 loads gen/probe/allocprobe.c (allocation counts; 0 for timing A/B runs),
#   AK_RPC_TRANSPORT (default pinned), CARGO_TARGET_DIR (a build of an ablation).
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?usage: stream_probe.sh OUT_DIR [NAME]}; NAME=${2:-probe}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1} AK_CPU_SERVER=${AK_CPU_SERVER:-2,3} AK_SERVER_THREADS=${AK_SERVER_THREADS:-4}
T=${AK_RPC_TRANSPORT:-pinned}
TGT=${CARGO_TARGET_DIR:-$HERE/target}
[ "${AK_PROBE_NOBUILD:-0}" = 1 ] || cargo build --release -q -p campaign --bin stream_probe 2>/dev/null
cc -O2 -shared -fPIC -o "$HERE/target/allocprobe.so" gen/probe/allocprobe.c -ldl
SCRATCH=$(mktemp -d); export AK_SERVE_STATE="$SCRATCH/serve.state"
cleanup() { ./serve.sh stop > /dev/null 2>&1 || true; rm -rf "$SCRATCH"; }
trap cleanup EXIT
./serve.sh build > /dev/null
o=$(./serve.sh start --out "$SCRATCH/serve")
SOCK=$(echo "$o" | sed -n "s/^$T //p")
./serve.sh warm 8 > /dev/null
PRE=(); [ "${AK_PROBE_SHIM:-1}" = 1 ] && PRE=(env LD_PRELOAD="$HERE/target/allocprobe.so")
{
  echo "# stream probe run $NAME: commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + UNCOMMITTED'); $(date -u +%FT%TZ); $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); client CPU $AK_CPU_CLIENT, server CPUs $AK_CPU_SERVER ($AK_SERVER_THREADS workers), transport $T, target $TGT, shim ${AK_PROBE_SHIM:-1}"
} > "$OUT/$NAME.head"
s=$(date +%s)
"${PRE[@]}" env AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT="$T" AK_OUT="$OUT/$NAME.body" taskset -c "$AK_CPU_CLIENT" "$TGT/release/stream_probe" 2> "$OUT/$NAME.err"
cat "$OUT/$NAME.head" "$OUT/$NAME.body" > "$OUT/$NAME.jsonl"; rm -f "$OUT/$NAME.head" "$OUT/$NAME.body"
echo "# run time $(( $(date +%s) - s )) s" >> "$OUT/$NAME.jsonl"
python3 gen/stream_probe.py "$OUT/$NAME.jsonl" > "$OUT/$NAME.txt"
cat "$OUT/$NAME.txt"
