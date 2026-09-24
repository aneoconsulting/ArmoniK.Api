#!/usr/bin/env bash
# Work unit 1, end to end and reproducible.
#
#   mech/run.sh <python-exe> [<python-exe> ...]      # first one is the TARGET
#
# The target interpreter gets three separate processes per benchmark; the others
# get one each, because their question is "does the shape of the answer change
# across interpreters" rather than "what is the number" (README R4: ratios
# inside one process travel, absolutes do not).
#
# Logs land in ffi/logs/python/.  Every figure quoted anywhere has to name one
# of them (README section 12).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOGS="$(cd "$HERE/../../.." && pwd)/logs/python"
mkdir -p "$LOGS"
cd "$HERE"

TARGET="$1"
TTAG=$("$TARGET" -c 'import sys;print("%d.%d"%sys.version_info[:2])')

hdr() {
  echo "# $1"
  echo "#"
  echo "# date:      $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# machine:   $(nproc) vCPU, $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | xargs)"
  echo "# commit:    $(git -C "$HERE" rev-parse --short HEAD)"
  echo "# R13:       this machine's Rust crossing is 2.1 ns (fwd+reverse) to 2.8 ns"
  echo "#            (forward), against 1.8 ns on the Rust slice's container."
  echo "#            See 00-r13-rust-crossing.log. Quote every absolute below"
  echo "#            against it as well as in nanoseconds."
  echo
}

echo "===== 1. build ====="
./build.sh "$@" 2>&1 | tee "$LOGS/10-build.log" | tail -4

# Step 2 (conformance of the codec arms) and steps 5-6 (the codec arms' timings) are RETIRED
# with those arms (FIX-PLAN WP5 step 6): they had their own generator with wire rules. The
# logs they wrote (20-conformance.log, 40-codec-*.log, 41-codec-all.log) are kept as history.

echo "===== 3. the mechanism microbenchmark, target interpreter, 3 processes ====="
{
  hdr "python slice, work unit 1: binding mechanism, facade storage, primitives"
  for i in 1 2 3; do
    echo "########## process $i ##########"
    "$TARGET" bench_mech.py
    echo
  done
} > "$LOGS/30-mechanism-py$TTAG.log" 2>&1
echo "   $LOGS/30-mechanism-py$TTAG.log"

echo "===== 4. the mechanism microbenchmark, every interpreter, 1 process ====="
{
  hdr "python slice, work unit 1: the same mechanisms across every interpreter here"
  echo "# Absolutes across these blocks are NOT comparable to each other in the way"
  echo "# a within-process ratio is: they are four processes. What they establish is"
  echo "# whether the SHAPE of the answer (which mechanism wins, by how much) holds"
  echo "# across 3.10 to 3.13, which is what README open question 4 needs."
  echo
  for PY in "$@"; do
    echo "########## $("$PY" -c 'import sys;print(sys.version.split()[0])') ##########"
    "$PY" bench_mech.py
    echo
  done
} > "$LOGS/31-mechanism-all.log" 2>&1
echo "   $LOGS/31-mechanism-all.log"

echo "done."
