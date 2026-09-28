#!/usr/bin/env bash
# Alternated A/B of the stream probe (CONTAINER INSTRUMENTATION): N process pairs, A then B,
# A = the default build (target/), B = the build in TGT_B (an ablation), same cells and settings;
# AK_AB_ENV_B="NAME=VALUE ..." is set for the B processes only (an env-gated ablation, same binary).
#   gen/stream_ab.sh OUT_DIR TAG TGT_B [N]    -> OUT_DIR/TAG-{a,b}N.jsonl, TAG.txt (gen/stream_ab.py)
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?}; TAG=${2:?}; TGT_B=${3:?}; N=${4:-3}
export AK_PROBE_SHIM=${AK_PROBE_SHIM:-0}
for i in $(seq 1 "$N"); do
  CARGO_TARGET_DIR="$HERE/target" bash gen/stream_probe.sh "$OUT" "$TAG-a$i" > /dev/null
  env ${AK_AB_ENV_B:-} AK_PROBE_NOBUILD=1 CARGO_TARGET_DIR="$TGT_B" bash gen/stream_probe.sh "$OUT" "$TAG-b$i" > /dev/null
done
python3 gen/stream_ab.py "$OUT" "$TAG" | tee "$OUT/$TAG.txt"
