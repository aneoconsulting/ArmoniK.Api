#!/usr/bin/env bash
# Untimed syscall census for the h2 PR 903 conditions (2026-10-01): one probe process per
# (condition, transport, workload, cell) under `strace -f -c` (every thread, the whole process:
# connection setup and warm-up included), counts divided by the calls the process made
# (warm + timed). Under the bench lock:
#   flock /tmp/ak-physical-bench.lock bash gen/h2_pr903_strace.sh OUT_DIR WT CORES_PREFIX
set -Eeuo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?}; WT=${2:?}; CP=${3:?}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
B=$WT/ffi/poc/rust/target/release/stream_probe; H=$WT/ffi/poc/rust/target-pr903host/release/stream_probe
TUN=GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
SCR=$(mktemp -d); export AK_SERVE_STATE=$SCR/state
cleanup() { ./serve.sh stop > /dev/null 2>&1 || true; rm -rf "$SCR"; }
trap cleanup EXIT
AK_SERVER_TCP=0 AK_CPU_SERVER=5-8,15-18 AK_SERVER_THREADS=8 ./serve.sh start --out "$OUT/server" > /dev/null
./serve.sh warm 8 > /dev/null 2>&1
SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE"); TCP=$(sed -n 's/^tcp //p' "$AK_SERVE_STATE")
echo "# strace census $(date -u +%FT%TZ); strace $(strace -V | head -1); per process: rounds 2, calls 4 (k=1) or 1 batch (k=8), warm 2 (k=1) or 1 batch (k=8)" > "$OUT/header.txt"
for tr in uds tcp; do
  for cond in ${AK_ST_CONDS:-stock p4 prc prh pp}; do
    b=$B; core=$CP-ctl/release; ex=()
    case $cond in p4) core=$CP-p4/release; ex=(AK_H2_COALESCE=16);; pp) core=$CP-pr903p4/release; ex=(AK_H2_COALESCE=16);; prc) core=$CP-pr/release;; prh) b=$H; core=$CP-pr/release;; esac
    tg=(); [ $tr = tcp ] && tg=(AK_RPC_TARGET=http://$TCP AK_EXPECT_NODELAY=1)
    for w in "16MiB 1 4 2" "16MiB 8 1 1"; do
      read -r size k calls warm <<< "$w"
      for cell in A Cf; do
        o="$OUT/$cond-$tr-$size-k$k-$cell"
        env "${tg[@]}" "${ex[@]}" LD_LIBRARY_PATH="$core" AK_HOST_WORKERS=8 AK_CORE_WORKERS=8 AK_SPARES=6 AK_SPARE_LOCK=1 "$TUN" \
            AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT=pinned AK_OUT="$o.jsonl" AK_PROBE_CELLS=$cell AK_PROBE_SIZES=$size AK_PROBE_K=$k \
            AK_PROBE_ROUNDS=2 AK_PROBE_CALLS=$calls AK_PROBE_WARM=$warm AK_PROBE_ORDER=block AK_PROBE_PROC=0 \
            taskset -c 1-4,11-14 strace -f -c -o "$o.strace" "$b" 2> "$o.err"
        echo "$cond $tr $size k$k $cell calls $(( (2 * calls + warm) * k ))" >> "$OUT/calls.txt"
      done
    done
  done
done
