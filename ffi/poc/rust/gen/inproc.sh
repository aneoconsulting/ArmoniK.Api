#!/usr/bin/env bash
# In-process comparisons (2026-09-30, owner's goals 1 and 2): every cell of AK_IP_CELLS in ONE
# bin stream_probe process per (condition, workload, repetition), block order, the cell order
# rotated by one per repetition, conditions alternated inside each repetition. Own server
# (serve.sh, 8 workers, SERVER set, pinned socket; AK_IP_SERVER_THREADS), started and stopped
# here; run the whole script under the benchmark lock:
#   flock /tmp/ak-physical-bench.lock bash gen/inproc.sh OUT_DIR
# Conditions: AK_IP_CONDS, space-separated NAME=PROBE_BINARY[:ENV=V,ENV=V...] (PROBE_BINARY `main` =
# this tree's target/release/stream_probe; another path = e.g. a patched worktree's build).
# Workloads: AK_IP_WORKS from d16k1 d16k8 d4k1 d4k8 c54k1 c54k8. Tables: gen/inproc_tables.py.
set -Eeuo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?usage: inproc.sh OUT_DIR}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
[ -e "$OUT/header.txt" ] && { echo "inproc.sh: $OUT already holds a session" >&2; exit 2; }
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1-4,11-14} AK_CPU_SERVER=${AK_CPU_SERVER:-5-8,15-18}
export AK_HOST_WORKERS=${AK_HOST_WORKERS:-8} AK_CORE_WORKERS=${AK_CORE_WORKERS:-8}
CELLS=${AK_IP_CELLS:-A,A2,Df,Cf,Ff}
# the cells of the direction-c workloads (default: the same); stream-only cells are left out there
CELLS_C=${AK_IP_CELLS_C:-$CELLS}
WORKS=${AK_IP_WORKS:-d16k1 d16k8}
REPS=${AK_IP_REPS:-3}
CONDS=${AK_IP_CONDS:-main=main}
read -r -a CS <<< "$CONDS"
SCR=$(mktemp -d); export AK_SERVE_STATE=$SCR/serve.state
cleanup() { ./serve.sh stop > /dev/null 2>&1 || true; rm -rf "$SCR"; }
trap cleanup EXIT
AK_SERVER_THREADS=${AK_IP_SERVER_THREADS:-8} ./serve.sh start --out "$OUT/server" > /dev/null
./serve.sh warm 16 > "$OUT/server/warm.log" 2>&1
SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE"); SPID=$(sed -n 's/^pid //p' "$AK_SERVE_STATE")
bin_of() { local b=${1%%:*}; [ "$b" = main ] && b="$HERE/target/release/stream_probe"; echo "$b"; }
# workload -> size k rounds calls-per-round warm
wl() { case $1 in d16k1) echo "16MiB 1 8 8 4";; d16k8) echo "16MiB 8 6 2 2";; d4k1) echo "4MiB 1 8 16 8";; d4k8) echo "4MiB 8 6 4 2";;
                  c54k1) echo "P5.4 1 8 16 8";; c54k8) echo "P5.4 8 6 4 2";; *) echo "?"; exit 2;; esac; }
{
  echo "# in-process comparison: commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + UNCOMMITTED'); $(date -u +%FT%TZ); $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); kernel $(uname -r); no_turbo $(cat /sys/devices/system/cpu/intel_pstate/no_turbo); cpu1 min/max $(cat /sys/devices/system/cpu/cpu1/cpufreq/scaling_min_freq)/$(cat /sys/devices/system/cpu/cpu1/cpufreq/scaling_max_freq) kHz; smt $(cat /sys/devices/system/cpu/smt/control); isolation: taskset only"
  echo "# client $AK_CPU_CLIENT, host $AK_HOST_WORKERS / core $AK_CORE_WORKERS workers (unless a condition sets them); server $AK_CPU_SERVER, ${AK_IP_SERVER_THREADS:-8} workers, pid $SPID, pinned socket"
  echo "# cells $CELLS (direction c: $CELLS_C) (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads $WORKS (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2); repetitions $REPS"
  for c in "${CS[@]}"; do b=$(bin_of "${c#*=}"); echo "# condition ${c%%=*}: binary $b (sha256 $(sha256sum "$b" | cut -c1-16), core $(ldd "$b" | awk '/libak_core/{print $3}')), env ${c#*:}"; done
} > "$OUT/header.txt"
rot() { python3 -S -c 'import sys; c=sys.argv[1].split(","); n=int(sys.argv[2])%len(c); print(",".join(c[n:]+c[:n]))' "$1" "$2"; }
S0=$(date +%s)
for rep in $(seq 1 "$REPS"); do
  for w in $WORKS; do
    read -r size k rounds calls warm <<< "$(wl "$w")"
    wc=$CELLS; case $w in c*) wc=$CELLS_C;; esac
    n=${#CS[@]}
    for j in $(seq 0 $((n - 1))); do
      c=${CS[$(( (j + rep - 1) % n ))]}; name=${c%%=*}; rest=${c#*=}; b=$(bin_of "$rest")
      ev=(); [ "$rest" != "${rest#*:}" ] && IFS=, read -r -a ev <<< "${rest#*:}"
      env "${ev[@]}" AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT=pinned AK_OUT="$OUT/$name-$w-$rep.jsonl" AK_PROBE_CELLS="$(rot "$wc" "$rep")" \
          AK_PROBE_SIZES="$size" AK_PROBE_K="$k" AK_PROBE_ROUNDS="$rounds" AK_PROBE_CALLS="$calls" AK_PROBE_WARM="$warm" \
          AK_PROBE_ORDER=block AK_PROBE_PROC=0 taskset -c "$AK_CPU_CLIENT" "$b" 2> "$OUT/$name-$w-$rep.err"
      echo "$name $w $rep done at +$(( $(date +%s) - S0 )) s" >> "$OUT/progress.txt"
    done
  done
done
echo "# benchmark wall time $(( $(date +%s) - S0 )) s" >> "$OUT/header.txt"
python3 gen/inproc_tables.py "$OUT" > "$OUT/tables.md"
