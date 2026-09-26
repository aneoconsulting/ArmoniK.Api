#!/usr/bin/env bash
set -euo pipefail
S=/tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad
O=/home/user/ArmoniK.Api/ffi/logs/rust/opt/layout-exp
mkdir -p $O
n=0
for v in A B A B; do
  n=$((n+1))
  d=$O/run$n-$v; mkdir -p $d
  L=$S/lay$v
  LD_LIBRARY_PATH=$L AK_ORDER=interleave AK_SEED=1 AK_LAUNCH=1 AK_ONLY=P AK_SAMPLES=20 AK_WARMUP_ITERS=100 AK_WARMUP_MS=50 AK_MEASURE_MS=200 AK_NODROP=P1.2,P2.2,P4.1,P6.1 AK_OUT=$d/codec-P.jsonl \
    taskset -c 1 $L/codec_suite > $d/codec-P.console.log 2>&1
  python3 /home/user/ArmoniK.Api/ffi/poc/rust/gen/opt_summary.py $d > /dev/null
  echo "run$n $v done $(date +%T)"
done
