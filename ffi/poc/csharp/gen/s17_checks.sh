#!/usr/bin/env bash
# s17 (owner, 2026-10-10): the fast checks of each part (CLAUDE.md "Checks, not gates, per
# change"; net8.0; CORRECTNESS ONLY, no timing). The cores already built in the slice
# directory are used (target-core*), not rebuilt.
#   gen/s17_checks.sh p1 OUTDIR    part 1: the host tally out of the timed build
# Exit 0 only if every run passes and every control fails.
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SLICE"
PART="${1:?usage: gen/s17_checks.sh p1 OUTDIR}"
OUT="${2:?usage: gen/s17_checks.sh p1 OUTDIR}"; mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
export SCRATCH="${SCRATCH:-$(mktemp -d)}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 MSBUILDDISABLENODEREUSE=1; mkdir -p "$SCRATCH"
RUST_EV="$SLICE/../../logs/rust/opt/d23-fsm/checks/events-counting.txt"
FAILS=0
run() { local name="$1"; shift; echo "\$ $*"; "$@"; local rc=$?; echo "exit status: $rc"; [ $rc -eq 0 ] || { echo "CHECK: $name FAILED"; FAILS=$((FAILS+1)); }; }
control() { local name="$1"; shift; echo "\$ $*   # a CONTROL: must fail"; "$@" > "$SCRATCH/ctl.out" 2>&1; local rc=$?; tail -n 2 "$SCRATCH/ctl.out"; echo "exit status: $rc"
  if [ $rc -ne 0 ]; then echo "control $name: failed, as required"; else echo "CONTROL $name DID NOT FAIL"; FAILS=$((FAILS+1)); fi; }
B="$SLICE/src/BenchDotNet/bin/Release/net8.0"; BN="$SLICE/src/BenchDotNet/bin-nounk/Release/net8.0"
BC="$SLICE/src/BenchDotNet/bin-count/Release/net8.0"; BCN="$SLICE/src/BenchDotNet/bin-count-nounk/Release/net8.0"
HB="$SLICE/src/Harness/bin/Release/net8.0"; HC="$SLICE/src/Harness/bin-count/Release/net8.0"; HCN="$SLICE/src/Harness/bin-count-nounk/Release/net8.0"
C="$SLICE/src/Corpus/bin/Release/net8.0"; CN="$SLICE/src/Corpus/bin-nounk/Release/net8.0"
bld() { echo "\$ dotnet build $*"; dotnet build "$@" > "$SCRATCH/build.log" 2>&1 || { tail -n 30 "$SCRATCH/build.log"; echo "BUILD FAILED: $*"; exit 1; }; grep -cE " warning CS" "$SCRATCH/build.log" | sed 's/^/#   C# warnings: /'; }
cores() {
  cp target-core/release/libak_core.so "$B/"; cp target-core-nounk/release/libak_core.so "$BN/"
  cp target-core-count/release/libak_core.so "$BC/"; cp target-core-count-nounk/release/libak_core.so "$BCN/"
  cp target-core/release/libak_core.so "$HB/"
  cp target-core-count/release/libak_core.so "$HC/"; cp target-core-count-nounk/release/libak_core.so "$HCN/"
  cp target-core-corpus/release/libak_core.so "$C/"; cp target-core-corpus-nounk/release/libak_core.so "$CN/"
}
echo "# s17_checks.sh $PART at $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec/gen || echo ' + uncommitted'); $(date -u +%FT%TZ); boot $(uptime -s)"
echo "## 0. generators current; no thread-local storage; every build (the four codec builds, the corpus, RPC, the s16 variant builds)"
python3 gen/generate.py > "$SCRATCH/gen.out" 2>&1; grep "^wrote" "$SCRATCH/gen.out"; run "generators current" bash -c "! grep -q '^wrote' '$SCRATCH/gen.out'"
run "no thread-local storage" python3 -I gen/no_tls.py src
for a in "" "-p:AkNounk=true" "-p:AkHostCount=true" "-p:AkHostCount=true -p:AkNounk=true"; do bld src/BenchDotNet/BenchDotNet.csproj -c Release $a; done
for a in "" "-p:AkNounk=true"; do bld src/Corpus/Corpus.csproj -c Release -f net8.0 $a; done
for a in "" "-p:AkHostCount=true" "-p:AkNounk=true" "-p:AkHostCount=true -p:AkNounk=true"; do bld src/Rpc/akrpc.csproj -c Release $a; done
for a in "-p:AkRoe=true" "-p:AkRoe=true -p:AkHostCount=true" "-p:AkRoeBench=true"; do bld src/BenchDotNet/BenchDotNet.csproj -c Release $a; done
cores
echo "# cores: $(for d in "$B" "$BN" "$BC" "$BCN" "$HC" "$C" "$CN"; do echo -n "$(echo "$d" | sed "s|$SLICE/src/||; s|/Release/net8.0||") $(sha256sum "$d/libak_core.so" | cut -c1-16); "; done)"

if [ "$PART" = p1 ]; then
echo "## 1. the host tally is in the counting builds' IL only (an IL walk of the built host assembly)"
run "il-tally timed full" bash -c "dotnet '$B/BenchDotNet.dll' --il-tally --expect none | tee '$OUT/il-tally-timed.txt' | grep -E '^il-tally'; exit \${PIPESTATUS[0]}"
run "il-tally timed nounk" bash -c "dotnet '$BN/BenchDotNet.dll' --il-tally --expect none | tee '$OUT/il-tally-timed-nounk.txt' | grep -E '^il-tally'; exit \${PIPESTATUS[0]}"
run "il-tally counting full (the walk sees the tally where it is)" bash -c "dotnet '$BC/BenchDotNet.dll' --il-tally --expect some | tee '$OUT/il-tally-count.txt' | grep -E '^il-tally'; exit \${PIPESTATUS[0]}"
run "il-tally counting nounk" bash -c "dotnet '$BCN/BenchDotNet.dll' --il-tally --expect some | tee '$OUT/il-tally-count-nounk.txt' | grep -E '^il-tally'; exit \${PIPESTATUS[0]}"
control "il-tally: the counting build is not 'none'" dotnet "$BC/BenchDotNet.dll" --il-tally --expect none
echo "## 2. byte identity: Cases.Verify (every arm, every string path, retain and drop) in the timed and the counting builds"
run "verify full" dotnet "$B/BenchDotNet.dll" --verify
run "verify nounk" dotnet "$BN/BenchDotNet.dll" --verify
run "verify counting full" dotnet "$BC/BenchDotNet.dll" --verify
run "verify counting nounk" dotnet "$BCN/BenchDotNet.dll" --verify
echo "## 3. the counts, counting build, against the committed files (base and every string path's file)"
cmpf() { if diff "$1" "$2" > "$SCRATCH/d.txt"; then echo "equal to $(basename "$1"): $(grep -vc '^#' "$1") rows"; else echo "DIFFERS from $(basename "$1")"; head -n 12 "$SCRATCH/d.txt"; return 1; fi; }
run "counts full" bash -c "dotnet '$BC/BenchDotNet.dll' --counts '$OUT/counts.txt' > /dev/null && $(declare -f cmpf); SCRATCH='$SCRATCH' cmpf gen/counts.txt '$OUT/counts.txt'"
run "counts nounk" bash -c "dotnet '$BCN/BenchDotNet.dll' --counts '$OUT/counts-nounk.txt' > /dev/null && $(declare -f cmpf); SCRATCH='$SCRATCH' cmpf gen/counts-nounk.txt '$OUT/counts-nounk.txt'"
for m in E1 E2 ETH:256 E3 E3L E1R E1C E1R:128; do t=$(echo "$m" | tr -d ':' | tr 'A-Z' 'a-z')
  AK_STR_ENC=$m run "counts $m full" bash -c "dotnet '$BC/BenchDotNet.dll' --counts '$OUT/counts-str-$t.txt' > /dev/null && $(declare -f cmpf); SCRATCH='$SCRATCH' cmpf gen/counts-str-$t.txt '$OUT/counts-str-$t.txt'"
  AK_STR_ENC=$m run "counts $m nounk" bash -c "dotnet '$BCN/BenchDotNet.dll' --counts '$OUT/counts-str-$t-nounk.txt' > /dev/null && $(declare -f cmpf); SCRATCH='$SCRATCH' cmpf gen/counts-str-$t-nounk.txt '$OUT/counts-str-$t-nounk.txt'"
done
echo "## 4. the coreffi crossings and R5 on the counting harness; the timed harness refuses the check"
run "coreffi counting full (R5, = gen/crossings.txt)" bash -c "AK_CROSSINGS_EXPECT='$SLICE/gen/crossings.txt' dotnet '$HC/harness.dll' coreffi > '$OUT/coreffi-count.log' 2>&1; rc=\$?; grep -E '^Crossing|^R5' '$OUT/coreffi-count.log'; exit \$rc"
run "coreffi counting nounk (= gen/crossings-nounk.txt)" bash -c "AK_CROSSINGS_EXPECT='$SLICE/gen/crossings-nounk.txt' dotnet '$HCN/harness.dll' coreffi > '$OUT/coreffi-count-nounk.log' 2>&1; rc=\$?; grep -E '^Crossing|^R5' '$OUT/coreffi-count-nounk.log'; exit \$rc"
{ cat gen/crossings.txt; echo "P9.9 1 1 1 1 1 1"; } > "$SCRATCH/cx-extra.txt"
AK_CROSSINGS_EXPECT="$SCRATCH/cx-extra.txt" control "crossing check, a committed row not produced (R-H3)" dotnet "$HC/harness.dll" coreffi
AK_CROSSINGS_EXPECT="$SLICE/gen/crossings.txt" control "the timed harness refuses the crossing check" dotnet "$HB/harness.dll" coreffi
run "coreffi timed (byte identity, value, push, pull, retain)" bash -c "dotnet '$HB/harness.dll' coreffi > '$OUT/coreffi-timed.log' 2>&1; rc=\$?; grep -E '^host tally|^R5' '$OUT/coreffi-timed.log'; exit \$rc"
echo "## 5. verify-fsm: the graphs on the timed builds; the counts and the Rust events on the counting builds"
run "verify-fsm timed full" bash -c "dotnet '$B/BenchDotNet.dll' --verify-fsm | tail -n 2; exit \${PIPESTATUS[0]}"
run "verify-fsm timed nounk" bash -c "dotnet '$BN/BenchDotNet.dll' --verify-fsm | tail -n 2; exit \${PIPESTATUS[0]}"
control "verify-fsm --rust-events on the timed build" dotnet "$B/BenchDotNet.dll" --verify-fsm --variants 0 --rust-events "$RUST_EV"
run "verify-fsm counts full (= the Rust events)" bash -c "dotnet '$BC/BenchDotNet.dll' --verify-fsm --variants 0 --events '$OUT/events-full.txt' --rust-events '$RUST_EV' | tail -n 2; exit \${PIPESTATUS[0]}"
run "verify-fsm counts nounk" bash -c "dotnet '$BCN/BenchDotNet.dll' --verify-fsm --variants 0 --events '$OUT/events-nounk.txt' --rust-events '$RUST_EV' | tail -n 2; exit \${PIPESTATUS[0]}"
fi

echo
[ $FAILS -eq 0 ] && echo "S17 $PART CHECKS PASSED (net8.0; fast checks, not the gate)" || echo "S17 $PART CHECKS: $FAILS FAILED"
exit $FAILS
