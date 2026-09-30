#!/usr/bin/env bash
# The core worker count under concurrency (2026-09-30, owner): AK_CORE_WORKERS = 1, 2, 4, 8, with
# patch p9 (AK_CB_AT_TAKE) off and on, host runtimes at 8 workers, pinned allocator, the stack's
# stream_probe (AK_WS_STACK). Two in-process sessions (gen/inproc.sh; each takes the bench lock
# for its whole length):
#   low   d/16 MiB and c/P5.4 at k = 1 and 8: A, Cf, Cf-cb, Cf-zc (c: A, Cf, Cf-cb)
#   high  the same at k = 16 and 32, plus the cells with 4 core clients (Cf-m4, Cf-cb-m4,
#         Cf-zc-m4: 4 connections, each its own core runtime, call i on client i % 4)
# then an untimed attribution pass per condition (/proc per thread class: CPU, switches), d/16 at
# k = 1 and 32 and c/P5.4 at k = 32.
#   gen/workers_sweep.sh OUT_DIR
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?usage: workers_sweep.sh OUT_DIR}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
B=${AK_WS_STACK:?AK_WS_STACK: the stack build of stream_probe}
TUN=GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
C=""
for cw in 1 2 4 8; do
  for p9 in 0 1; do
    C="$C w$cw-p9$p9=$B:AK_CORE_WORKERS=$cw,AK_CB_AT_TAKE=$p9,AK_HOST_WORKERS=8,AK_SPARES=6,AK_SPARE_LOCK=1,$TUN"
  done
done
export AK_IP_CONDS="$C" AK_IP_REPS=${AK_WS_REPS:-3}
if [ "${AK_WS_SKIP_LOW:-0}" != 1 ]; then
  flock /tmp/ak-physical-bench.lock env AK_IP_WORKS="d16k1 d16k8 c54k1 c54k8" AK_IP_CELLS=A,Cf,Cf-cb,Cf-zc AK_IP_CELLS_C=A,Cf,Cf-cb bash gen/inproc.sh "$OUT/low"
fi
if [ "${AK_WS_SKIP_HIGH:-0}" != 1 ]; then
  flock /tmp/ak-physical-bench.lock env AK_IP_WORKS="d16k16 d16k32 c54k16 c54k32" AK_IP_CELLS=A,Cf,Cf-cb,Cf-zc,Cf-m4,Cf-cb-m4,Cf-zc-m4 AK_IP_CELLS_C=A,Cf,Cf-cb,Cf-m4,Cf-cb-m4 bash gen/inproc.sh "$OUT/high"
fi
# the attribution pass: /proc per thread class, untimed, own server, under the lock
attr() {
  export AK_SERVE_STATE; AK_SERVE_STATE=$(mktemp -u)
  AK_CPU_SERVER=5-8,15-18 AK_SERVER_THREADS=8 ./serve.sh start --out "$OUT/attr/server" > /dev/null
  local sock; sock=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE")
  for cw in 1 2 4 8; do
    for p9 in 0 1; do
      for w in "16MiB 1 A,Cf,Cf-cb,Cf-zc" "16MiB 32 A,Cf,Cf-cb,Cf-zc,Cf-m4,Cf-cb-m4,Cf-zc-m4" "P5.4 32 A,Cf,Cf-cb,Cf-m4,Cf-cb-m4"; do
        read -r size k cells <<< "$w"
        env AK_EXPECT_CPUS=1-4,11-14 AK_CORE_WORKERS=$cw AK_CB_AT_TAKE=$p9 AK_HOST_WORKERS=8 AK_SPARES=6 AK_SPARE_LOCK=1 "$TUN" AK_RPC_SOCKET="$sock" AK_RPC_TRANSPORT=pinned \
            AK_OUT="$OUT/attr/w$cw-p9$p9-$size-k$k.jsonl" AK_PROBE_CELLS="$cells" AK_PROBE_SIZES="$size" AK_PROBE_K="$k" \
            AK_PROBE_ROUNDS=3 AK_PROBE_CALLS=$([ "$k" = 1 ] && echo 4 || echo 1) AK_PROBE_WARM=1 AK_PROBE_ORDER=block AK_PROBE_PROC=1 \
            taskset -c 1-4,11-14 "$B" 2> "$OUT/attr/w$cw-p9$p9-$size-k$k.err"
      done
    done
  done
  ./serve.sh stop > /dev/null
}
mkdir -p "$OUT/attr"
flock /tmp/ak-physical-bench.lock bash -c "$(declare -f attr); OUT='$OUT' B='$B' TUN='$TUN' attr"
