#!/usr/bin/env bash
# s12 checks (net8.0; CORRECTNESS ONLY): the FSM consumer's [SuppressGCTransition] path.
#   gen/s12_checks.sh OUTDIR
# 1. gen/s10_checks.sh as it is (the plain path: nothing regressed), into OUTDIR/plain;
# 2. gen/s10_checks.sh with AK_FSM_SGT=1 (the FSM family of --verify-fsm, of the corpus's ffi
#    arms and of the four plants goes through TryFsmSgt), into OUTDIR/sgt;
# 3. --verify-fsm in the COUNTING builds with AK_FSM_SGT=1: how many FSM calls took the
#    attributed imports (the rest: retain, and each context's first decode).
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SLICE"
OUT="${1:?usage: gen/s12_checks.sh OUTDIR}"; mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
FAILS=0
echo "# s12_checks.sh at $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec/gen || echo ' + uncommitted'); $(date -u +%FT%TZ)"
echo "## 1. gen/s10_checks.sh, plain path"
gen/s10_checks.sh "$OUT/plain" > "$OUT/plain.log" 2>&1 || FAILS=$((FAILS+1)); tail -n 1 "$OUT/plain.log"
echo "## 2. gen/s10_checks.sh with AK_FSM_SGT=1"
AK_FSM_SGT=1 gen/s10_checks.sh "$OUT/sgt" > "$OUT/sgt.log" 2>&1 || FAILS=$((FAILS+1)); tail -n 1 "$OUT/sgt.log"
echo "## 3. the share of FSM calls on the attributed imports (counting builds, AK_FSM_SGT=1, --variants 48)"
RUST_EV="$SLICE/../../logs/rust/opt/d23-fsm/checks/events-counting.txt"
for b in bin-count bin-count-nounk; do
  core=target-core-count; [ $b = bin-count-nounk ] && core=target-core-count-nounk
  cp "$core/release/libak_core.so" "src/BenchDotNet/$b/Release/net8.0/"
  echo "\$ AK_FSM_SGT=1 BenchDotNet ($b) --verify-fsm"
  AK_FSM_SGT=1 dotnet "src/BenchDotNet/$b/Release/net8.0/BenchDotNet.dll" --verify-fsm --rust-events "$RUST_EV" | tail -n 4 || FAILS=$((FAILS+1))
done
echo
[ $FAILS -eq 0 ] && echo "S12 CHECKS PASSED (net8.0; not the gate)" || echo "S12 CHECKS: $FAILS FAILED"
exit $FAILS
