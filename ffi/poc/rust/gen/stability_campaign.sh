#!/usr/bin/env bash
# The stability campaign (owner, 2026-09-30): about 20 minutes of benchmark wall time, many short
# processes alternating the HEAD core and the optimisation stack, allocator pinned (glibc static
# thresholds), one own server (serve.sh, 8 workers). One bin stream_probe process per (repetition,
# workload, condition), the conditions alternated inside each workload and the cell order rotated
# per repetition (gen/inproc.sh). Run under the benchmark lock:
#   flock /tmp/ak-physical-bench.lock bash gen/stability_campaign.sh OUT_DIR
# Environment: AK_ST_STACK = the stack's stream_probe (a worktree build of p1+p2+p3+p5+p6+p7 on
# crates.io h2); AK_ST_REPS (default 28).
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?usage: stability.sh OUT_DIR}
STACK=${AK_ST_STACK:?AK_ST_STACK: the stack build of stream_probe}
TUN=GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
export AK_IP_CONDS="head=main:AK_PROBE_SKIP_MISSING=1,$TUN stack=$STACK:AK_PROBE_SKIP_MISSING=1,AK_SPARES=6,AK_SPARE_LOCK=1,$TUN"
export AK_IP_WORKS="d16k1 d16k8 d4k1 c54k1 c54k8"
export AK_IP_CELLS=A,A2,Cf,Cf-cb,Cf-zc,Cf-zcw
export AK_IP_CELLS_C=A,A2,Cf,Cf-cb
export AK_IP_REPS=${AK_ST_REPS:-28}
bash gen/inproc.sh "$OUT"
python3 gen/stability_tables.py "$OUT" > "$OUT/stability.md"
