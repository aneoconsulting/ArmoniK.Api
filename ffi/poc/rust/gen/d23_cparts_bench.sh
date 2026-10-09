#!/usr/bin/env bash
# D23 C isolation: fsm_attrib on the six fix-C variants (gen/d23_cparts_build.sh), alternated.
# CONTAINER INSTRUMENTATION.   gen/d23_cparts_bench.sh OUT_DIR [LAUNCHES]
set -euo pipefail
OUT=${1:?out dir}; L=${2:-2}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
cd "$(dirname "$0")/.."
ROWS="P6.1 U-wire-ListMetricsResponse-batches-as-wt0 P7.1 P4.1 P2.2 U-root-before U-oneof-group U-wire-ListProbeResponse-probes-as-wt0 P1.1 PK-ticks PK-codes PK-flags PK-values EV-empty"
STAGES=(AB C0 C0v C0f C0i C)
for s in "${STAGES[@]}"; do
  echo "# $s: core $(sha256sum target-cp-$s/release/deps/libak_core.so | cut -c1-16)" | tee -a "$OUT/header.txt"
done
for l in $(seq 1 "$L"); do
  for i in $(seq 0 5); do
    s=${STAGES[$(( (i + 3 * (l - 1)) % 6 ))]}
    w=0; while awk '{ exit !($1 >= 1.0) }' /proc/loadavg && [ $w -lt 120 ]; do sleep 5; w=$((w + 5)); done
    echo "# launch $l stage $s: loadavg $(cut -d' ' -f1-3 /proc/loadavg) (waited $w s)" | tee -a "$OUT/header.txt"
    taskset -c 1 target-cp-$s/release/fsm_attrib bench --rounds 15 --round-ms 12 $ROWS > "$OUT/$s-drop-$l.txt" 2>&1
    taskset -c 1 target-cp-$s/release/fsm_attrib bench --rounds 15 --round-ms 12 --retain $ROWS > "$OUT/$s-retain-$l.txt" 2>&1
  done
done
echo "# done, loadavg $(cut -d' ' -f1-3 /proc/loadavg)" | tee -a "$OUT/header.txt"
