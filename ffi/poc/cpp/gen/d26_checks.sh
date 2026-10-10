#!/usr/bin/env bash
# FIX-PLAN D26 (owner, 2026-10-10, as amended): no thread-local storage on a timed path; the
# binding's decode state lives in a host context (DecHost) per decode context. The cpp slice's
# fast checks of that change (ffi/CLAUDE.md "Checks, not gates, per change"); nothing timed.
# Writes logs/cpp/d26/{no-tls,generators,conformance,corpus,counts,concurrency}.log.
#   gen/d26_checks.sh [BUILD_DIR]     (configured with -DAK_RPC=ON; every target built)
#
#   1  gen/no_tls.py --plant: no thread-local outside the counting build's two tallies, and
#      the planted control is found
#   2  generators current (this slice's gen/generate.py --check)
#   3  gen/d27_checks.sh into logs/cpp/d26/: conformance (full, no-unknown), the corpus (six
#      arms, decision 11's controls and their plant, the no-unknown build), the crossing counts
#      (codec, core grid, RPC per call) against the committed count logs
#   4  gen/concurrency.sh: the concurrency suite, shipped builds pass, planted builds fail
set -u
cd "$(dirname "$0")/.." || exit 2
B=${1:-build}
L=../../logs/cpp; D=$L/d26; mkdir -p "$D"
fails=0
hdr() { echo "# $1 ($(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/cpp or poc/codec'); $(g++ --version | head -1); $(rustc --version))"; }
{ hdr "cpp D26 checks: no thread-local storage"; python3 gen/no_tls.py --plant; } > "$D/no-tls.log" 2>&1 \
  && echo ">>> ok: no_tls" || { echo ">>> FAIL: no_tls"; fails=$((fails+1)); }
{ hdr "cpp D26 checks: generators"; python3 gen/generate.py --check; } > "$D/generators.log" 2>&1 \
  && echo ">>> ok: generate.py --check" || { echo ">>> FAIL: generate.py --check"; fails=$((fails+1)); }
AK_CHECK_LOGDIR=$D bash gen/d27_checks.sh "$B" || fails=$((fails+1))
{ hdr "cpp D26 checks: concurrency"; B=$B bash gen/concurrency.sh; } > "$D/concurrency.log" 2>&1 \
  && echo ">>> ok: concurrency" || { echo ">>> FAIL: concurrency"; fails=$((fails+1)); }
echo "cpp D26 checks: $fails failing step(s)"; exit $((fails ? 1 : 0))
