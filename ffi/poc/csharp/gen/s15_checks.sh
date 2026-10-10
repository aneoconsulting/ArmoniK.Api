#!/usr/bin/env bash
# s15 (D26, owner 2026-10-10) checks: no thread-local storage, the host contexts. CORRECTNESS
# ONLY (net8.0), no timing; the full gate (gen/gate.sh, both h2 variants) follows separately.
#   gen/s15_checks.sh OUTDIR
#  1. no [ThreadStatic] / ThreadLocal / AsyncLocal / ConcurrentBag in the sources and generated
#     code (gen/no_tls.py), and its control (a planted file);
#  2. gen/s7_checks.sh: Cases.Verify (every payload, content set and U-* row under E0 E1 E2
#     ETH:16 E3 E3L E1R E1R(K 3) E1C(K 3) E1R:16 E1R:16:na E3:16:na E1R:128 E1C:16(K 3); retain,
#     drop and the no-unknown build), --verify-mt per path on both builds, the pin-stress runs,
#     the corpus per path, the planted short string, the early-unpin control, the K bound, the
#     counts of E3 E3L E1R E1C E1R:128 and E0;
#  3. the remaining count files: E1, E2, ETH:256 (both builds), and --verify-mt under E0;
#  4. gen/s10_checks.sh: layout, verify, verify-fsm against the Rust events, the corpus (both
#     builds, retain strict), the unknown-field controls, the counts, the four FSM plants.
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SLICE"
OUT="${1:?usage: gen/s15_checks.sh OUTDIR}"; mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
export SCRATCH="${SCRATCH:-$(mktemp -d)}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 MSBUILDDISABLENODEREUSE=1
FAILS=0
run() { local name="$1"; shift; echo "\$ $*"; "$@"; local rc=$?; echo "exit status: $rc"; [ $rc -eq 0 ] || { echo "CHECK: $name FAILED"; FAILS=$((FAILS+1)); }; }
control() { local name="$1"; shift; echo "\$ $*   # a CONTROL: must fail"; "$@" > "$SCRATCH/ctl.out" 2>&1; local rc=$?; tail -3 "$SCRATCH/ctl.out"; echo "exit status: $rc"
  if [ $rc -ne 0 ]; then echo "control $name: failed, as required"; else echo "CONTROL $name DID NOT FAIL"; FAILS=$((FAILS+1)); fi; }
echo "# s15_checks.sh at $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec/gen || echo ' + uncommitted'); $(date -u +%FT%TZ); boot $(uptime -s)"
echo "## 1. no thread-local storage"
run "no_tls src" python3 -I gen/no_tls.py src
mkdir -p "$SCRATCH/tlsplant" && printf 'class Planted {\n    [ThreadStatic] private static int _x;\n}\n' > "$SCRATCH/tlsplant/Planted.cs"
control "planted [ThreadStatic]" python3 -I gen/no_tls.py "$SCRATCH/tlsplant"
echo "## 2. gen/s7_checks.sh (log: s7-checks.log)"
run "s7_checks" bash -c "gen/s7_checks.sh > '$OUT/s7-checks.log' 2>&1; rc=\$?; grep -E 'FAILED|DID NOT FAIL|PASSED' '$OUT/s7-checks.log' | tail -5; exit \$rc"
B="$SLICE/src/BenchDotNet/bin/Release/net8.0"; BN="$SLICE/src/BenchDotNet/bin-nounk/Release/net8.0"
BC="$SLICE/src/BenchDotNet/bin-count/Release/net8.0"; BCN="$SLICE/src/BenchDotNet/bin-count-nounk/Release/net8.0"
echo "## 3. the other count files; --verify-mt under E0"
for m in E1 E2 ETH:256; do t=$(echo "$m" | tr -d ':' | tr 'A-Z' 'a-z')
  run "counts $m full" bash -c "AK_STR_ENC=$m dotnet '$BC/BenchDotNet.dll' --counts '$SCRATCH/c-$t.txt' > /dev/null && diff -q gen/counts-str-$t.txt '$SCRATCH/c-$t.txt'"
  run "counts $m nounk" bash -c "AK_STR_ENC=$m dotnet '$BCN/BenchDotNet.dll' --counts '$SCRATCH/c-$t-nounk.txt' > /dev/null && diff -q gen/counts-str-$t-nounk.txt '$SCRATCH/c-$t-nounk.txt'"
done
run "counts nounk E0 unchanged" bash -c "dotnet '$BCN/BenchDotNet.dll' --counts '$SCRATCH/c-e0n.txt' > /dev/null && diff -q gen/counts-nounk.txt '$SCRATCH/c-e0n.txt'"
run "verify-mt E0 full" dotnet "$B/BenchDotNet.dll" --verify-mt
run "verify-mt E0 nounk" dotnet "$BN/BenchDotNet.dll" --verify-mt
echo "## 4. gen/s10_checks.sh (log: s10/)"
run "s10_checks" bash -c "gen/s10_checks.sh '$OUT/s10' > '$OUT/s10-checks.log' 2>&1; rc=\$?; grep -E 'FAILED|NOT CAUGHT|PASSED' '$OUT/s10-checks.log' | tail -5; exit \$rc"
echo
[ $FAILS -eq 0 ] && echo "S15 CHECKS PASSED (net8.0; not the gate)" || echo "S15 CHECKS: $FAILS FAILED"
exit $FAILS
