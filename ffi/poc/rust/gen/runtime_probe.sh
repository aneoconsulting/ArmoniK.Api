#!/usr/bin/env bash
# The runtime probe (CONTAINER INSTRUMENTATION, 2026-09-28): direction d, k = 1, bin
# stream_probe under runtime and feed-depth variants, ONE server session (serve.sh, 4 workers
# on AK_CPU_SERVER; the client on AK_CPU_CLIENT; pinned transport).
#
#   gen/runtime_probe.sh OUT_DIR [N]
#
# Variants: AK_RTP_VARIANTS, space-separated NAME=TARGET:ENV,ENV,... (TARGET `main` = target/,
# `rtp` = target-rtp/, a build of the core with the experiment patch
# logs/rust/opt/runtime-probe/core-depth.patch, which reads AK_CORE_CHAN_DEPTH). Each of the N
# iterations runs every variant once as its own process, the variant order rotating by one
# per iteration, block order, no /proc reads (timing). Then one attribution pass per variant
# (not timed): /proc reads (per-thread-class CPU, context switches, faults) and the
# allocation shim (gen/probe/allocprobe.c), AK_RTP_ATTR_ROUNDS x AK_RTP_ATTR_CALLS.
# target-rtp is not kept (the patch is reverted): to rebuild it, from the repository root
#   git apply ffi/logs/rust/opt/runtime-probe/core-depth.patch
#   (cd ffi/poc/rust && CARGO_TARGET_DIR=$PWD/target-rtp cargo build --release -p campaign --bin stream_probe)
#   git checkout ffi/poc/codec/crates/ak-core/src/rpc.rs
# AK_RTP_CELLS: the probe's cells; a variant with AK_HOST_WORKERS=ct drops Df-chan (see
# stream_probe.rs df_chan: nothing drives the body while the host thread blocks).
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?usage: runtime_probe.sh OUT_DIR [N]}; N=${2:-3}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1} AK_CPU_SERVER=${AK_CPU_SERVER:-2,3} AK_SERVER_THREADS=${AK_SERVER_THREADS:-4}
CELLS=${AK_RTP_CELLS:-A,Df,Df-chan,Cf,Cf-cb,Cf-split,Cf-cb-split}
VARIANTS=${AK_RTP_VARIANTS:-"base=main: h1=main:AK_HOST_WORKERS=1 hct=main:AK_HOST_WORKERS=ct c1=main:AK_CORE_WORKERS=1 b1=main:AK_HOST_WORKERS=1,AK_CORE_WORKERS=1 p1=rtp:AK_CORE_CHAN_DEPTH=1,AK_CHAN_DEPTH=1 d2=rtp:AK_CORE_CHAN_DEPTH=2,AK_CHAN_DEPTH=2"}
ROUNDS=${AK_PROBE_ROUNDS:-12}; CALLS=${AK_PROBE_CALLS:-8}; WARM=${AK_PROBE_WARM:-4}
AR=${AK_RTP_ATTR_ROUNDS:-4}; AC=${AK_RTP_ATTR_CALLS:-4}
read -r -a VS <<< "$VARIANTS"
cc -O2 -shared -fPIC -o "$HERE/target/allocprobe.so" gen/probe/allocprobe.c -ldl
for v in "${VS[@]}"; do
  t=${v#*=}; t=${t%%:*}; b="$HERE/target"; [ "$t" = rtp ] && b="$HERE/target-rtp"
  [ -x "$b/release/stream_probe" ] || { echo "no $b/release/stream_probe (variant $v)"; exit 1; }
done
SCRATCH=$(mktemp -d); export AK_SERVE_STATE="$SCRATCH/serve.state"
cleanup() { ./serve.sh stop > /dev/null 2>&1 || true; rm -rf "$SCRATCH"; }
trap cleanup EXIT
./serve.sh build > /dev/null
o=$(./serve.sh start --out "$SCRATCH/serve"); SOCK=$(echo "$o" | sed -n 's/^pinned //p')
./serve.sh warm 8 > /dev/null
echo "# runtime probe: commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + UNCOMMITTED'); $(date -u +%FT%TZ); $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); client CPU $AK_CPU_CLIENT, server CPUs $AK_CPU_SERVER ($AK_SERVER_THREADS workers), pinned; cells $CELLS; iterations $N (variant order rotating), block order, no /proc; rounds $ROUNDS calls $CALLS warm $WARM; attribution pass rounds $AR calls $AC with /proc and the allocation shim; variants: $VARIANTS" > "$OUT/header.txt"
run() { # name target envs mode(time|attr) file
  local name=$1 t=$2 envs=$3 mode=$4 file=$5 b="$HERE/target" cells=$CELLS
  [ "$t" = rtp ] && b="$HERE/target-rtp"
  local ev=(); IFS=, read -r -a ev <<< "$envs"
  case ",$envs," in *,AK_HOST_WORKERS=ct,*) cells=$(echo "$CELLS" | tr ',' '\n' | grep -vx 'Df-chan' | paste -sd, -);; esac
  if [ "$mode" = time ]; then
    env "${ev[@]}" AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT=pinned AK_OUT="$file" AK_PROBE_CELLS="$cells" \
        AK_PROBE_ORDER=block AK_PROBE_PROC=0 AK_PROBE_ROUNDS=$ROUNDS AK_PROBE_CALLS=$CALLS AK_PROBE_WARM=$WARM \
        taskset -c "$AK_CPU_CLIENT" "$b/release/stream_probe" 2> "${file%.jsonl}.err"
  else
    env "${ev[@]}" LD_PRELOAD="$HERE/target/allocprobe.so" AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT=pinned AK_OUT="$file" AK_PROBE_CELLS="$cells" \
        AK_PROBE_ORDER=rotate AK_PROBE_PROC=1 AK_PROBE_ROUNDS=$AR AK_PROBE_CALLS=$AC AK_PROBE_WARM=$WARM \
        taskset -c "$AK_CPU_CLIENT" "$b/release/stream_probe" 2> "${file%.jsonl}.err"
  fi
}
s0=$(date +%s)
nv=${#VS[@]}
for i in $(seq 1 "$N"); do
  for j in $(seq 0 $((nv - 1))); do
    v=${VS[$(( (j + i - 1) % nv ))]}
    name=${v%%=*}; rest=${v#*=}; t=${rest%%:*}; envs=${rest#*:}
    run "$name" "$t" "$envs" time "$OUT/$name-$i.jsonl"
    echo "iteration $i $name done at +$(( $(date +%s) - s0 )) s" >> "$OUT/progress.txt"
  done
done
if [ "$AR" -gt 0 ]; then
  for v in "${VS[@]}"; do
    name=${v%%=*}; rest=${v#*=}; t=${rest%%:*}; envs=${rest#*:}
    run "$name" "$t" "$envs" attr "$OUT/attr-$name.jsonl"
    echo "attr $name done at +$(( $(date +%s) - s0 )) s" >> "$OUT/progress.txt"
  done
fi
echo "# run time $(( $(date +%s) - s0 )) s" >> "$OUT/header.txt"
python3 gen/runtime_tables.py "$OUT" > "$OUT/tables.md"
cat "$OUT/tables.md"
