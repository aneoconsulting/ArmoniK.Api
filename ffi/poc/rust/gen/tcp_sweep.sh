#!/usr/bin/env bash
# The core worker count on TCP (2026-10-01, owner, before deciding the core worker count):
# AK_CORE_WORKERS = 1, 2, 4, 8 by h2 variant {stock, h2-batch}, the landed p1 (ring of 6, blocking
# return), host runtimes at 8 workers, pinned allocator. Two in-process sessions (gen/inproc.sh,
# TCP 127.0.0.1 with TCP_NODELAY read back, task-clock beside the process clock, server task-clock
# per call; each session takes the bench lock for its whole length):
#   low   d/16 MiB and c/P5.4 at k = 1 and 8: A, Cf, Cf-cb
#   high  the same at k = 16 and 32, plus the cells with 4 core clients (Cf-m4, Cf-cb-m4:
#         4 connections, each its own core runtime, call i on client i % 4)
# Every condition runs this tree's stream_probe with the core of its variant loaded by
# LD_LIBRARY_PATH: CORES/stock and CORES/h2-batch, both built by poc/codec/h2-batch/build.sh from
# the poc/codec workspace (so the two differ only by h2). A, D and F (tonic in the host) keep
# crates.io h2 in both.
#   gen/tcp_sweep.sh OUT_DIR CORES          (tables: gen/sweep_tcp_tables.py OUT_DIR)
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?usage: tcp_sweep.sh OUT_DIR CORES}; CORES=${2:?usage: tcp_sweep.sh OUT_DIR CORES}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
B=${AK_TS_PROBE:-$HERE/target/release/stream_probe}
TUN=GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
C=""
for cw in 1 2 4 8; do
  for v in stock h2-batch; do
    C="$C $v-w$cw=$B:LD_LIBRARY_PATH=$CORES/$v/release,AK_CORE_WORKERS=$cw,AK_HOST_WORKERS=8,$TUN"
  done
done
export AK_IP_CONDS="${C# }" AK_IP_REPS=${AK_TS_REPS:-3} AK_IP_TRANSPORT=tcp
if [ "${AK_TS_SKIP_LOW:-0}" != 1 ]; then
  flock /tmp/ak-physical-bench.lock env AK_IP_WORKS="d16k1 d16k8 c54k1 c54k8" AK_IP_CELLS=A,Cf,Cf-cb AK_IP_CELLS_C=A,Cf,Cf-cb bash gen/inproc.sh "$OUT/low"
fi
if [ "${AK_TS_SKIP_HIGH:-0}" != 1 ]; then
  flock /tmp/ak-physical-bench.lock env AK_IP_WORKS="d16k16 d16k32 c54k16 c54k32" AK_IP_CELLS=A,Cf,Cf-cb,Cf-m4,Cf-cb-m4 AK_IP_CELLS_C=A,Cf,Cf-cb,Cf-m4,Cf-cb-m4 bash gen/inproc.sh "$OUT/high"
fi
