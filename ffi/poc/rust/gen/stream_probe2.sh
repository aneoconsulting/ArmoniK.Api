#!/usr/bin/env bash
# Stream probe, second unit (CONTAINER INSTRUMENTATION): in ONE server session (serve.sh, 4
# workers on AK_CPU_SERVER; client on AK_CPU_CLIENT; pinned transport), N iterations of
#   grid-i      the grid's own client (benches/rpc_suite.rs) narrowed to direction d, k = 1,
#               the probe's cells, AK_SAMPLES=10 AK_WARMUP_MS=30 AK_MEASURE_MS=500 (the
#               same-machine run's G3 settings)
#   VARIANT-i   bin stream_probe, one per variant in AK_PROBE_VARIANTS (name:ORDER:PROC,
#               e.g. rot:rotate:1 blk:block:0)
#   gen/stream_probe2.sh OUT_DIR [N]
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?}; N=${2:-3}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1} AK_CPU_SERVER=${AK_CPU_SERVER:-2,3} AK_SERVER_THREADS=${AK_SERVER_THREADS:-4}
CELLS=${AK_PROBE_CELLS:-A,Df,Cf,C,D,B}
# the grid knows no probe-only cell (Df-chan)
GRID_CELLS=$(echo "$CELLS" | tr ',' '\n' | grep -vE -- '-(chan|split)$' | paste -sd, -)
VARIANTS=${AK_PROBE_VARIANTS:-rot:rotate:1 blk:block:0}
TGT=${CARGO_TARGET_DIR:-$HERE/target}
cargo build --release -q -p campaign --bin stream_probe 2>/dev/null
RPCB=$(cargo bench -q -p campaign --bench rpc_suite --no-run --message-format=json 2>/dev/null | python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="rpc_suite" and m.get("executable"): print(m["executable"])' | tail -1)
SCRATCH=$(mktemp -d); export AK_SERVE_STATE="$SCRATCH/serve.state"
cleanup() { ./serve.sh stop > /dev/null 2>&1 || true; rm -rf "$SCRATCH"; }
trap cleanup EXIT
./serve.sh build > /dev/null
o=$(./serve.sh start --out "$SCRATCH/serve"); SOCK=$(echo "$o" | sed -n 's/^pinned //p')
./serve.sh warm 8 > /dev/null
echo "# stream probe 2: commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + UNCOMMITTED'); $(date -u +%FT%TZ); $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); client CPU $AK_CPU_CLIENT, server CPUs $AK_CPU_SERVER ($AK_SERVER_THREADS workers), pinned; cells $CELLS; variants $VARIANTS; probe rounds ${AK_PROBE_ROUNDS:-15} calls ${AK_PROBE_CALLS:-8} warm ${AK_PROBE_WARM:-4}; grid AK_SAMPLES=10 AK_WARMUP_MS=30 AK_MEASURE_MS=500" > "$OUT/header.txt"
for i in $(seq 1 "$N"); do
  env AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT=pinned AK_OUT="$OUT/grid-$i.jsonl" CRITERION_HOME="$SCRATCH/crit-$i" \
      AK_SAMPLES=10 AK_WARMUP_MS=30 AK_MEASURE_MS=500 AK_NRESAMPLES=1000 AK_RPC_SERVER_WARMUP=0 \
      AK_RPC_CELLS="$GRID_CELLS" AK_RPC_DIRS=d AK_RPC_INFLIGHT=1 AK_LAUNCH=$i \
      taskset -c "$AK_CPU_CLIENT" "$RPCB" > "$OUT/grid-$i.criterion.log" 2>&1
  for v in $VARIANTS; do
    name=${v%%:*}; rest=${v#*:}; ord=${rest%%:*}; pr=${rest#*:}
    env AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT=pinned AK_OUT="$OUT/$name-$i.jsonl" AK_PROBE_CELLS="$CELLS" \
        AK_PROBE_ORDER="$ord" AK_PROBE_PROC="$pr" ${AK_PROBE_PRELOAD:+LD_PRELOAD=$AK_PROBE_PRELOAD} \
        taskset -c "$AK_CPU_CLIENT" "$TGT/release/stream_probe" 2> "$OUT/$name-$i.err"
  done
done
python3 gen/stream_probe2.py "$OUT" | tee "$OUT/summary.txt"
