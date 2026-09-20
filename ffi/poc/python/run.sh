#!/usr/bin/env bash
# Work unit 2, end to end and reproducible.
#
#   ./run.sh <target-python> [<other pythons>...]
#
# Logs land in ffi/logs/python/.  Absolutes are instrumentation (README section 8, after
# R13), so the target interpreter gets three processes and the others one each: what the
# extra interpreters establish is that the SHAPE of the answer holds, not a number.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOGS="$(cd "$HERE/../.." && pwd)/logs/python"
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
  echo "# core:      ffi/poc/codec (README R0), ABI v1, libak_core.so via the dynamic linker"
  echo "# R13:       this machine's rust-slice crossing is 2.1 ns (fwd+reverse) to 2.8 ns"
  echo "#            (forward) -- 00-r13-rust-crossing.log. The composed arm also prices"
  echo "#            the boundary IN ITS OWN PROCESS at the end of the bench table, which"
  echo "#            is the better number to quote against because it shares a build."
  echo "# absolutes: instrumentation, not the deliverable. Signs and size classes only."
  echo
}

echo "===== 1. build ====="
./build.sh "$@" 2>&1 | tee "$LOGS/50-build-wu2.log" | tail -3

echo "===== 2. R14: derive the baseline from Protos/V1 ====="
{ hdr "python slice: R14, the baseline is the path ArmoniK runs"; "$TARGET" verify_r14.py; } \
  > "$LOGS/52-r14-baseline.log" 2>&1
tail -4 "$LOGS/52-r14-baseline.log"

echo "===== 3. conformance and crossing counts, every interpreter (R2, R5) ====="
{
  hdr "python slice, work unit 2: conformance and crossing counts on the composed arm"
  for PY in "$@"; do
    echo "########## $("$PY" -c 'import sys;print(sys.version.split()[0])') ##########"
    "$PY" conformance.py
    echo
  done
} > "$LOGS/51-conformance-wu2.log" 2>&1
grep -c "ALL CHECKS PASS" "$LOGS/51-conformance-wu2.log" | sed 's/^/   interpreters passing: /'

echo "===== 4. the composed arm, target interpreter, 3 processes ====="
{
  hdr "python slice, work unit 2: the composed arm, encode and decode, over M1"
  for i in 1 2 3; do
    echo "########## process $i ##########"
    "$TARGET" bench.py
    echo
  done
} > "$LOGS/60-composed-py$TTAG.log" 2>&1
echo "   $LOGS/60-composed-py$TTAG.log"

echo "===== 5. the composed arm, every interpreter, 1 process ====="
{
  hdr "python slice, work unit 2: the composed arm across every interpreter here"
  for PY in "$@"; do
    echo "########## $("$PY" -c 'import sys;print(sys.version.split()[0])') ##########"
    "$PY" bench.py
    echo
  done
} > "$LOGS/61-composed-all.log" 2>&1
echo "   $LOGS/61-composed-all.log"
echo "done."
