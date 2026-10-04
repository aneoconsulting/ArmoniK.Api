#!/usr/bin/env bash
# D21 step 7 correctness checks (net8.0; CORRECTNESS ONLY, no timing): the string paths E3, E3L,
# E1R, E1C and the threshold / ASCII splits. Builds already made (gen/gate.sh's kept cores).
#   gen/s7_checks.sh [extra AK_STR_ENC specs to add to the corpus runs]
# Exit 0 only if every run passes and every control fails.
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SLICE"
export SCRATCH="${SCRATCH:-$(mktemp -d)}"
FAILS=0
run() { local name="$1"; shift; echo "\$ $*"; "$@"; local rc=$?; echo "exit status: $rc"; [ $rc -eq 0 ] || { echo "CHECK: $name FAILED"; FAILS=$((FAILS+1)); }; }
control() { local name="$1"; shift; echo "\$ $*   # a CONTROL: must fail"; "$@" > "${SCRATCH:-/tmp}/ctl.out" 2>&1; local rc=$?; tail -4 "${SCRATCH:-/tmp}/ctl.out"; echo "exit status: $rc"
  if [ $rc -ne 0 ]; then echo "control $name: failed, as required"; else echo "CONTROL $name DID NOT FAIL"; FAILS=$((FAILS+1)); fi; }
B="$SLICE/src/BenchDotNet/bin/Release/net8.0"; BN="$SLICE/src/BenchDotNet/bin-nounk/Release/net8.0"
BC="$SLICE/src/BenchDotNet/bin-count/Release/net8.0"; BCN="$SLICE/src/BenchDotNet/bin-count-nounk/Release/net8.0"
C="$SLICE/src/Corpus/bin/Release/net8.0/corpus"; CN="$SLICE/src/Corpus/bin-nounk/Release/net8.0/corpus"
# Every binary the checks run is rebuilt first (a stale corpus build was once checked: JOURNAL 76).
export MSBUILDDISABLENODEREUSE=1
for a in "" "-p:AkNounk=true"; do
  for pr in src/BenchDotNet/BenchDotNet.csproj src/Corpus/Corpus.csproj; do
    dotnet build $pr -c Release -f net8.0 $a > "$SCRATCH/build.log" 2>&1 || { cat "$SCRATCH/build.log"; exit 1; }
  done
  dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -f net8.0 -p:AkHostCount=true $a > "$SCRATCH/build.log" 2>&1 || { cat "$SCRATCH/build.log"; exit 1; }
done
echo "# s7_checks.sh at $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec/gen || echo ' + uncommitted'); $(date -u +%FT%TZ)"
echo "# cores: $(sha256sum "$B/libak_core.so" | cut -c1-16) (full) $(sha256sum "$BN/libak_core.so" | cut -c1-16) (nounk) $(sha256sum "$SLICE/src/Corpus/bin/Release/net8.0/libak_core.so" | cut -c1-16) (corpus) $(sha256sum "$SLICE/src/Corpus/bin-nounk/Release/net8.0/libak_core.so" | cut -c1-16) (corpus nounk)"
echo "## 1. Cases.Verify (every payload, content set and U-* row under E0 E1 E2 ETH:16 E3 E3L E1R E1R(K 3) E1C(K 3) E1R:16 E1R:16:na E3:16:na E1R:128 E1C:16(K 3))"
run "verify full" dotnet "$B/BenchDotNet.dll" --verify
run "verify nounk" dotnet "$BN/BenchDotNet.dll" --verify
echo "## 2. the corpus under each path (full and no-unknown); E1R and E1C with K 3 under a compacting GC"
echo "##    after every chunk (AK_GATE_PIN_STRESS: a string the core read after its frame's release would be"
echo "##    read moved; on the corpus only: on the shapes it is a GC per 3 strings, tens of thousands per encode)"
for m in E3 E3L E1R E1C E1R:128 "$@"; do
  AK_CORPUS_RETAIN_STRICT=1 AK_STR_ENC=$m run "corpus $m" "$C"
  AK_STR_ENC=$m run "corpus nounk $m" "$CN"
done
AK_CORPUS_RETAIN_STRICT=1 AK_STR_PINK=3 AK_GATE_PIN_STRESS=1 AK_STR_ENC=E1R run "corpus E1R K 3, pin stress" "$C"
AK_CORPUS_RETAIN_STRICT=1 AK_STR_PINK=3 AK_GATE_PIN_STRESS=1 AK_STR_ENC=E1C run "corpus E1C K 3, pin stress" "$C"
echo "## 3. controls: each path live (a planted short string), the stress check live (handles released before the call)"
for m in E3 E3L E1R E1C; do AK_GATE_PLANT_STR=1 AK_STR_ENC=$m control "corpus $m planted short" "$C" --only "S-"; done
AK_GATE_PLANT_EARLY_UNPIN=1 AK_STR_PINK=3 AK_STR_ENC=E1C control "corpus E1C K 3, handles released before the call + compacting GC" "$C" --only "S-"
echo "## 4. K bound: a K above Stage.MaxPinK is refused"
AK_STR_PINK=257 AK_STR_ENC=E1R control "K 257 refused" dotnet "$B/BenchDotNet.dll" --verify
echo "## 5. counts per path (= gen/counts-str-<path>[-nounk].txt)"
for m in E3 E3L E1R E1C E1R:128; do t=$(echo "$m" | tr -d ':' | tr 'A-Z' 'a-z')
  run "counts $m full" bash -c "AK_STR_ENC=$m dotnet '$BC/BenchDotNet.dll' --counts '$SCRATCH/c-$t.txt' > /dev/null && diff -q gen/counts-str-$t.txt '$SCRATCH/c-$t.txt'"
  run "counts $m nounk" bash -c "AK_STR_ENC=$m dotnet '$BCN/BenchDotNet.dll' --counts '$SCRATCH/c-$t-nounk.txt' > /dev/null && diff -q gen/counts-str-$t-nounk.txt '$SCRATCH/c-$t-nounk.txt'"
done
run "counts E0 full unchanged" bash -c "dotnet '$BC/BenchDotNet.dll' --counts '$SCRATCH/c-e0.txt' > /dev/null && diff -q gen/counts.txt '$SCRATCH/c-e0.txt'"
echo
[ $FAILS -eq 0 ] && echo "S7 CHECKS PASSED (net8.0; not the gate)" || echo "S7 CHECKS: $FAILS FAILED"
exit $FAILS
