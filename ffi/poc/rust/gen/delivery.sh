#!/usr/bin/env bash
# The response-delivery comparison (2026-10-01, owner), TCP, two in-process sessions (gen/inproc.sh,
# each under the bench lock for its whole length; 3 processes per condition, conditions alternated):
#   a   cell A (tonic) in its four forms: A (tasks spawned on the cell's runtime, awaited by
#       block_on: the probe's reference A), A-blk, A-cb, A-q (stream_probe's delivery_batch):
#       a-stock   this tree's probe (crates.io h2 in the host), core CORES/stock
#       a-hosttoo the probe built with h2-batch in the host (target-deliv-h2batch), core CORES/h2-batch
#   cf  the core cells Cf (blocking), Cf-cb (callback), Cf-q (completion queue), and A beside them:
#       stock-w8, h2batch-w8 (core workers 8, the main configuration), stock-w1, h2batch-w1
# Workloads d/16 MiB k=1 and k=8, d/4 MiB k=1, c/P5.4 k=1 and k=8; host runtimes 8 workers, pinned
# allocator. Builds: gen/deliv_build.sh. Tables: gen/deliv_tables.py OUT_DIR.
#   gen/delivery.sh OUT_DIR CORES
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?usage: delivery.sh OUT_DIR CORES}; CORES=${2:?}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
B=$HERE/target-deliv/release/stream_probe; BH=$HERE/target-deliv-h2batch/release/stream_probe
TUN=GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
W="d16k1 d16k8 d4k1 c54k1 c54k8"
export AK_IP_REPS=${AK_DV_REPS:-3} AK_IP_TRANSPORT=tcp AK_IP_WORKS="$W"
if [ "${AK_DV_SKIP_A:-0}" != 1 ]; then
  flock /tmp/ak-physical-bench.lock env AK_IP_CELLS=A,A-blk,A-cb,A-q AK_IP_CELLS_C=A,A-blk,A-cb,A-q \
      AK_IP_CONDS="a-stock=$B:LD_LIBRARY_PATH=$CORES/stock/release,AK_CORE_WORKERS=8,AK_HOST_WORKERS=8,$TUN a-hosttoo=$BH:LD_LIBRARY_PATH=$CORES/h2-batch/release,AK_CORE_WORKERS=8,AK_HOST_WORKERS=8,$TUN" \
      bash gen/inproc.sh "$OUT/a"
fi
if [ "${AK_DV_SKIP_CF:-0}" != 1 ]; then
  C=""
  for cw in 8 1; do
    C="$C stock-w$cw=$B:LD_LIBRARY_PATH=$CORES/stock/release,AK_CORE_WORKERS=$cw,AK_HOST_WORKERS=8,$TUN"
    C="$C h2batch-w$cw=$B:LD_LIBRARY_PATH=$CORES/h2-batch/release,AK_CORE_WORKERS=$cw,AK_HOST_WORKERS=8,$TUN"
  done
  flock /tmp/ak-physical-bench.lock env AK_IP_CELLS=A,Cf,Cf-cb,Cf-q AK_IP_CELLS_C=A,Cf,Cf-cb,Cf-q AK_IP_CONDS="${C# }" bash gen/inproc.sh "$OUT/cf"
fi
