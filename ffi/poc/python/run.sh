#!/usr/bin/env bash
# The whole slice, end to end and reproducible.
#
#   ./run.sh <target-python> [<other pythons>...]
#
# The target is CPython 3.12 (FIX-PLAN section 6, D1). AK_UPSTREAM and
# AK_CARGO_TARGET_BASE are passed through to build.sh (see there).
#
# PHASE (README 1.1): steps 5 to 9 print container TIMINGS, which are instrumentation. Run
# them to prove a harness executes; do not quote them. Steps 1 to 4 and 3b are the ones
# whose output is a result now (builds, byte identity, crossing counts, corpus, gates).
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
  # The dirty marker matters: a log whose header names a commit that does not contain the
  # code that produced it is the kind of thing that survives into a report unnoticed.
  echo "# commit:    $(git -C "$HERE" rev-parse --short HEAD)$(
        git -C "$HERE" diff --quiet HEAD -- "$HERE" || echo ' + UNCOMMITTED CHANGES in poc/python')"
  echo "# core:      ffi/poc/codec (README R0), ABI v1, libak_core.so via the dynamic linker"
  echo "# R13:       this machine's rust-slice crossing is 2.1 ns (fwd+reverse) to 2.8 ns"
  echo "#            (forward) -- 00-r13-rust-crossing.log. The composed arm also prices"
  echo "#            the boundary IN ITS OWN PROCESS at the end of the bench table, which"
  echo "#            is the better number to quote against because it shares a build."
  echo "# absolutes: instrumentation, not the deliverable. Signs and size classes only."
  echo
}

echo "===== 1. build ====="
# The incumbent's shapes_pb2.py is emitted by mech/build.sh (grpcio-tools carries protoc).
# From a clean clone nothing else writes it, and conformance would report upb ABSENT.
mech/build.sh "$@" > /dev/null 2>&1 || { echo "   mech/build.sh failed"; exit 1; }
./build.sh "$@" 2>&1 | tee "$LOGS/54-build-all-shapes.log" | tail -3

echo "===== 2. R14: derive the baseline from Protos/V1 ====="
{ hdr "python slice: R14, the baseline is the path ArmoniK runs"; "$TARGET" verify_r14.py; } \
  > "$LOGS/52-r14-baseline.log" 2>&1
tail -4 "$LOGS/52-r14-baseline.log"

echo "===== 3. conformance and crossing counts, every interpreter (R2, R5) ====="
{
  hdr "python slice: conformance and crossing counts, M1 through M7"
  for PY in "$@"; do
    echo "########## $("$PY" -c 'import sys;print(sys.version.split()[0])') ##########"
    "$PY" conformance.py
    echo
  done
} > "$LOGS/53-conformance-all-shapes.log" 2>&1
grep -c "ALL CHECKS PASS" "$LOGS/53-conformance-all-shapes.log" | sed 's/^/   interpreters passing: /'

echo "===== 3b. R-D3: the RPC shim through the same gate, and the RPC arm under injected failure ====="
# `_akffi_rpc` carries the codec too (cell C of the RPC grid decodes through it), so it is
# gated exactly like `_akffi`. Nothing ran this before FIX-PLAN R-D3.
{
  hdr "python slice: conformance on the RPC shim (_akffi_rpc)"
  for PY in "$@"; do
    echo "########## $("$PY" -c 'import sys;print(sys.version.split()[0])') ##########"
    AK_FFI_MODULE=_akffi_rpc "$PY" conformance.py
    echo
  done
} > "$LOGS/85-conformance-rpc-shim.log" 2>&1
grep -c "ALL CHECKS PASS" "$LOGS/85-conformance-rpc-shim.log" | sed 's/^/   interpreters passing on _akffi_rpc: /'
# What every RPC cell reports when the RPC fails: status, empty body, short body, closed
# socket. No timings in it. Target interpreter only (it needs grpcio).
{
  hdr "python slice: the RPC arm's gate under injected failure (R-D3)"
  "$TARGET" rpc_gate.py
} > "$LOGS/83-rpc-gate.log" 2>&1
echo "   rows aborted under injection: $(grep -c 'ABORTED, no figure' "$LOGS/83-rpc-gate.log"); rows timed under injection: $(grep -c 'FIGURE PRODUCED' "$LOGS/83-rpc-gate.log")"

echo "===== 4. the conformance corpus (W8), every row this scope can root ====="
{
  hdr "python slice: the conformance corpus, as far as walk.ROOTS reaches"
  "$TARGET" corpus.py
} > "$LOGS/70-corpus-subset.log" 2>&1
grep -m1 "rows root at" "$LOGS/70-corpus-subset.log" | sed 's/^ */   /'

echo "===== 5. the allocator control: why an encode above 128 KiB has two answers ====="
{
  hdr "python slice: the allocator state, not the codec, owns the large-payload encode"
  "$TARGET" allocator.py
} > "$LOGS/55-allocator.log" 2>&1
grep -c -- "  <-- " "$LOGS/55-allocator.log" | sed 's/^/   rows whose answer depends on it: /'

echo "===== 6. concurrency: ABI v1 obligation 12.5, and the GIL ====="
{
  hdr "python slice: the concurrency suite no slice in the branch had"
  "$TARGET" concurrency.py
} > "$LOGS/56-concurrency.log" 2>&1
tail -1 "$LOGS/56-concurrency.log" | sed 's/^/   /'

echo "===== 7. the collector: what disabling it was worth, and to whom ====="
{
  hdr "python slice: the GC bias the RPC arm found (defect D11)"
  "$TARGET" gcbias.py
} > "$LOGS/57-gc-bias.log" 2>&1
grep -c -- "  <--" "$LOGS/57-gc-bias.log" | sed 's/^/   rows the collector was discounting: /'

echo "===== 8. the composed arm, target interpreter, 3 processes ====="
{
  hdr "python slice: the composed arm, encode and decode, M1 through M7"
  for i in 1 2 3; do
    echo "########## process $i ##########"
    "$TARGET" bench.py
    echo
  done
} > "$LOGS/62-all-shapes-py$TTAG.log" 2>&1
echo "   $LOGS/62-all-shapes-py$TTAG.log"

echo "===== 9. the composed arm, every interpreter, 1 process ====="
{
  hdr "python slice, work unit 3: the composed arm across every interpreter here"
  for PY in "$@"; do
    echo "########## $("$PY" -c 'import sys;print(sys.version.split()[0])') ##########"
    "$PY" bench.py
    echo
  done
} > "$LOGS/63-all-shapes-every-interpreter.log" 2>&1
echo "   $LOGS/63-all-shapes-every-interpreter.log"
echo "done."
