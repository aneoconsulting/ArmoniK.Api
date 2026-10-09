#!/usr/bin/env bash
# D23 fixes A, B, C (owner-approved): fsm_attrib on the four stages of the FSM, alternated.
# CONTAINER INSTRUMENTATION. Each stage is its own build (target-fix0 = before the fixes, ebdb0f6a;
# target-fix1 = A, 55c2771c; target-fix2 = A+B, 01c73821; target-fix3 = A+B+C, 75f819f8); push and pull
# are the same code in all four (their generated code is unchanged), so they are the in-process controls.
#   gen/d23_fix_bench.sh OUT_DIR [LAUNCHES]
set -euo pipefail
OUT=${1:?out dir}; L=${2:-2}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
cd "$(dirname "$0")/.."
ROWS_DROP="P6.1 U-wire-ListMetricsResponse-batches-as-wt0 P7.1 P4.1 U-oneof-u-fixed32 U-oneof-group U-wire-UploadResultDataMessage-upload-as-wt0 U-wire-ListProbeResponse-probes-as-wt0 U-root-before P1.1 P2.2 PK-ticks PK-values PK-flags PK-codes EV-empty"
ROWS_RET="P6.1 U-wire-ListMetricsResponse P7.1 P4.1 U-oneof-u-fixed32 U-oneof-group U-wire-UploadResultDataMessage-upload-as-wt0 U-wire-ListProbeResponse-probes-as-wt0 U-root-before P1.1 P2.2 PK-ticks PK-values EV-empty"
STAGES=(fix0 fix1 fix2 fix3)
for s in "${STAGES[@]}"; do
  e=target-$s/release/fsm_attrib
  echo "# $s: $e sha256 $(sha256sum $e | cut -c1-16), core $(ldd $e | grep -o '/[^ ]*libak_core.so') sha256 $(sha256sum $(ldd $e | grep -o '/[^ ]*libak_core.so') | cut -c1-16)" | tee -a "$OUT/header.txt"
done
for l in $(seq 1 "$L"); do
  for i in 0 1 2 3; do
    s=${STAGES[$(( (i + l - 1) % 4 ))]}
    w=0; while awk '{ exit !($1 >= 1.0) }' /proc/loadavg && [ $w -lt 120 ]; do sleep 5; w=$((w + 5)); done
    echo "# launch $l stage $s: loadavg $(cut -d' ' -f1-3 /proc/loadavg) (waited $w s)" | tee -a "$OUT/header.txt"
    taskset -c 1 target-$s/release/fsm_attrib bench --rounds 21 --round-ms 15 $ROWS_DROP > "$OUT/$s-drop-$l.txt" 2>&1
    taskset -c 1 target-$s/release/fsm_attrib bench --rounds 21 --round-ms 15 --retain $ROWS_RET > "$OUT/$s-retain-$l.txt" 2>&1
    python3 gen/d23_attrib_tables.py "$OUT/$s-drop-$l.txt" > "$OUT/$s-drop-$l.derived.txt"
  done
done
echo "# done, loadavg $(cut -d' ' -f1-3 /proc/loadavg)" | tee -a "$OUT/header.txt"
