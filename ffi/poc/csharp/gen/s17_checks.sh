#!/usr/bin/env bash
# s17 (owner, 2026-10-10): the fast checks of each part (CLAUDE.md "Checks, not gates, per
# change"; net8.0; CORRECTNESS ONLY, no timing). The cores already built in the slice
# directory are used (target-core*), not rebuilt.
#   gen/s17_checks.sh p1 OUTDIR    part 1: the host tally out of the timed build
#   gen/s17_checks.sh p2 OUTDIR    part 2: the lazy nested encode render (/p:AkLazy=true)
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

if [ "$PART" = p2 ]; then
BL="$SLICE/src/BenchDotNet/bin-lazy/Release/net8.0"; BLN="$SLICE/src/BenchDotNet/bin-lazy-nounk/Release/net8.0"
BLC="$SLICE/src/BenchDotNet/bin-lazy-count/Release/net8.0"; BLCN="$SLICE/src/BenchDotNet/bin-lazy-count-nounk/Release/net8.0"
HL="$SLICE/src/Harness/bin-lazy/Release/net8.0"; HLC="$SLICE/src/Harness/bin-lazy-count/Release/net8.0"
CL="$SLICE/src/Corpus/bin-lazy/Release/net8.0"; CLN="$SLICE/src/Corpus/bin-lazy-nounk/Release/net8.0"
lazy_build() {
  for a in "-p:AkLazy=true" "-p:AkLazy=true -p:AkNounk=true" "-p:AkLazy=true -p:AkHostCount=true" "-p:AkLazy=true -p:AkHostCount=true -p:AkNounk=true"; do bld src/BenchDotNet/BenchDotNet.csproj -c Release $a; done
  for a in "-p:AkLazy=true" "-p:AkLazy=true -p:AkNounk=true"; do bld src/Corpus/Corpus.csproj -c Release -f net8.0 $a; done
  cp target-core/release/libak_core.so "$BL/"; cp target-core-nounk/release/libak_core.so "$BLN/"
  cp target-core-count/release/libak_core.so "$BLC/"; cp target-core-count-nounk/release/libak_core.so "$BLCN/"
  cp target-core/release/libak_core.so "$HL/"; cp target-core-count/release/libak_core.so "$HLC/"
  cp target-core-corpus/release/libak_core.so "$CL/"; cp target-core-corpus-nounk/release/libak_core.so "$CLN/"
}
lazy_build
echo "# lazy cores: $(for d in "$BL" "$BLN" "$BLC" "$BLCN" "$HL" "$HLC" "$CL" "$CLN"; do echo -n "$(echo "$d" | sed "s|$SLICE/src/||; s|/Release/net8.0||") $(sha256sum "$d/libak_core.so" | cut -c1-16); "; done)"
echo "## 1. the lazy render is the one built: Go stages no slot (no Arr.Ensure of an S_ / I_ array outside a loop callback)"
run "lazy host rendered" bash -c "python3 -I - src/Harness/GeneratedLazy/CoreFfi.cs src/Harness/Generated/CoreFfi.cs <<'PY'
import re, sys
def go_ensures(path):
    s = open(path).read(); n = 0
    for m in re.finditer(r'private int Go\(', s):
        body = s[m.end():s.index('\n    }\n', m.end())]
        n += body.count('Arr.Ensure(')
    return n
lz, eg = go_ensures(sys.argv[1]), go_ensures(sys.argv[2])
print('Arr.Ensure calls inside Go: lazy %d, eager %d' % (lz, eg))
sys.exit(0 if lz == 0 and eg > 0 else 1)
PY"
echo "## 2. byte identity: Cases.Verify (every arm, every string path E0 E1 E2 ETH:16 E3 E3L E1R E1R(K 3) E1C(K 3) E1R:16 E1R:16:na E3:16:na E1R:128 E1C:16(K 3), retain and drop, every payload, content set and U-* row), both builds and the counting ones"
run "verify lazy full" dotnet "$BL/BenchDotNet.dll" --verify
run "verify lazy nounk" dotnet "$BLN/BenchDotNet.dll" --verify
run "verify lazy counting full" dotnet "$BLC/BenchDotNet.dll" --verify
echo "## 3. --verify-mt (8 threads) per path, both builds; pin stress (1 thread) for the frame paths; the early-unpin control"
for m in E0 E1 E2 E3 E3L E1R E1C E1R:128; do
  AK_STR_ENC=$m run "verify-mt $m lazy full" dotnet "$BL/BenchDotNet.dll" --verify-mt
  AK_STR_ENC=$m run "verify-mt $m lazy nounk" dotnet "$BLN/BenchDotNet.dll" --verify-mt
done
for m in E1R E1C E1R:128; do AK_GATE_PIN_STRESS=1 AK_STR_ENC=$m run "verify-mt $m lazy 1 thread, pin stress" dotnet "$BL/BenchDotNet.dll" --verify-mt --threads 1 --rounds 1; done
for m in E1R E1C; do AK_GATE_PIN_STRESS=1 AK_STR_PINK=3 AK_STR_ENC=$m run "verify-mt $m K 3 lazy 1 thread, pin stress" dotnet "$BL/BenchDotNet.dll" --verify-mt --threads 1 --rounds 1; done
AK_GATE_PLANT_EARLY_UNPIN=1 AK_GATE_PIN_STRESS=1 AK_STR_ENC=E1C control "verify-mt E1C lazy, handles released before the call + compacting GC" dotnet "$BL/BenchDotNet.dll" --verify-mt --threads 1 --rounds 1
echo "## 4. the coreffi gate on the lazy harness: element chunks of 150 (the per-chunk staging at off > 0), E0 and E1R; the crossings on the lazy counting harness"
for m in E0 E1R E1C; do
  AK_CHUNK=150 AK_STR_ENC=$m run "coreffi lazy AK_CHUNK=150 $m" bash -c "dotnet '$HL/harness.dll' coreffi > '$OUT/coreffi-lazy-chunk150-$m.log' 2>&1; rc=\$?; grep -E '^host tally|failed|FAIL' '$OUT/coreffi-lazy-chunk150-$m.log' | head -n 5; exit \$rc"
done
run "coreffi lazy counting (R5, = gen/crossings.txt)" bash -c "AK_CROSSINGS_EXPECT='$SLICE/gen/crossings.txt' dotnet '$HLC/harness.dll' coreffi > '$OUT/coreffi-lazy-count.log' 2>&1; rc=\$?; grep -E '^Crossing|^R5' '$OUT/coreffi-lazy-count.log'; exit \$rc"
echo "## 5. the corpus (four arms, retain strict), both builds, per path; E1R / E1C K 3 under pin stress; the push arms; the unknown-field controls"
for m in E0 E1 E2 E3 E3L E1R E1C E1R:128; do t=$(echo "$m" | tr -d ':' | tr 'A-Z' 'a-z')
  AK_STR_ENC=$m AK_CORPUS_RETAIN_STRICT=1 run "corpus lazy full $m" bash -c "'$CL/corpus' > '$OUT/corpus-lazy-full-$t.log' 2>&1; rc=\$?; tail -n 2 '$OUT/corpus-lazy-full-$t.log'; exit \$rc"
done
for m in E0 E1R E1R:128; do t=$(echo "$m" | tr -d ':' | tr 'A-Z' 'a-z')
  AK_STR_ENC=$m AK_CORPUS_DROP_STRICT=1 run "corpus lazy nounk $m" bash -c "'$CLN/corpus' > '$OUT/corpus-lazy-nounk-$t.log' 2>&1; rc=\$?; tail -n 2 '$OUT/corpus-lazy-nounk-$t.log'; exit \$rc"
done
for m in E1R E1C; do AK_CORPUS_RETAIN_STRICT=1 AK_STR_PINK=3 AK_GATE_PIN_STRESS=1 AK_STR_ENC=$m run "corpus lazy $m K 3, pin stress" bash -c "'$CL/corpus' --timeout-ms 300000 > '$OUT/corpus-lazy-pinstress-$m.log' 2>&1; rc=\$?; tail -n 2 '$OUT/corpus-lazy-pinstress-$m.log'; exit \$rc"; done
AK_CORPUS_PUSH=1 AK_CORPUS_RETAIN_STRICT=1 run "corpus lazy full with the push arms" bash -c "'$CL/corpus' > '$OUT/corpus-lazy-push.log' 2>&1; rc=\$?; tail -n 2 '$OUT/corpus-lazy-push.log'; exit \$rc"
run "corpus lazy unk-controls" bash -c "'$CL/corpus' --unk-controls > '$OUT/corpus-lazy-unk-controls.log' 2>&1; rc=\$?; tail -n 3 '$OUT/corpus-lazy-unk-controls.log'; exit \$rc"
echo "## 6. the counts of the lazy render (counting build) against the committed ones (the eager render's): every file"
cmpf() { if diff "$1" "$2" > "$SCRATCH/d.txt"; then echo "equal to $(basename "$1"): $(grep -vc '^#' "$1") rows"; else echo "DIFFERS from $(basename "$1"): $(grep -c '^>' "$SCRATCH/d.txt") rows"; head -n 6 "$SCRATCH/d.txt"; return 1; fi; }
run "counts lazy full" bash -c "dotnet '$BLC/BenchDotNet.dll' --counts '$OUT/counts-lazy.txt' > /dev/null && $(declare -f cmpf); SCRATCH='$SCRATCH' cmpf gen/counts.txt '$OUT/counts-lazy.txt'"
run "counts lazy nounk" bash -c "dotnet '$BLCN/BenchDotNet.dll' --counts '$OUT/counts-lazy-nounk.txt' > /dev/null && $(declare -f cmpf); SCRATCH='$SCRATCH' cmpf gen/counts-nounk.txt '$OUT/counts-lazy-nounk.txt'"
for m in E1 E2 ETH:256 E3 E3L E1R:128 E1R E1C; do t=$(echo "$m" | tr -d ':' | tr 'A-Z' 'a-z')
  AK_STR_ENC=$m run "counts lazy $m full" bash -c "dotnet '$BLC/BenchDotNet.dll' --counts '$OUT/counts-lazy-str-$t.txt' > /dev/null && $(declare -f cmpf); SCRATCH='$SCRATCH' cmpf gen/counts-str-$t.txt '$OUT/counts-lazy-str-$t.txt'"
  AK_STR_ENC=$m run "counts lazy $m nounk" bash -c "dotnet '$BLCN/BenchDotNet.dll' --counts '$OUT/counts-lazy-str-$t-nounk.txt' > /dev/null && $(declare -f cmpf); SCRATCH='$SCRATCH' cmpf gen/counts-str-$t-nounk.txt '$OUT/counts-lazy-str-$t-nounk.txt'"
done
echo "## 7. planted defects in the lazy render (src/Harness/GeneratedLazy/CoreFfi.cs), each must fail --verify (or the chunked coreffi gate)"
G="src/Harness/GeneratedLazy/CoreFfi.cs"
plant() {  # name from to [check: verify | chunk]
  local name="$1" from="$2" to="$3" how="${4:-verify}"
  cp "$G" "$SCRATCH/CoreFfiLazy.orig"
  python3 -I - "$G" "$from" "$to" <<'PY'
import sys
p, a, b = sys.argv[1:4]
s = open(p).read(); k = s.count(a)
assert k > 0, "plant anchor not found: " + a
open(p, "w").write(s.replace(a, b)); print("planted %d site(s)" % k)
PY
  bld src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkLazy=true
  cp target-core/release/libak_core.so "$BL/"; cp target-core/release/libak_core.so "$HL/"
  if [ "$how" = chunk ]; then AK_CHUNK=150 control "$name" dotnet "$HL/harness.dll" coreffi
  else control "$name" dotnet "$BL/BenchDotNet.dll" --verify; fi
  cp "$SCRATCH/CoreFfiLazy.orig" "$G"
}
plant "an element's nested run read from element 0 (the token ignored)" "h.Root.Tasks[te]" "h.Root.Tasks[0]"
plant "a frame sub-chunk staged at the chunk's start (E1R / E1C, K 3)" "lst, a2, off + o2, k2" "lst, arr, off + o2, k2"
plant "a chunk's elements read from the list's start (off ignored; chunks of 150)" "lst[off + i], _st)" "lst[i], _st)" chunk
plant "Root not set (the callbacks read a null root)" "        __h.Root = src;" "        __h.Root = null;"
bld src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkLazy=true
cp target-core/release/libak_core.so "$BL/"; cp target-core/release/libak_core.so "$HL/"
run "generated file restored" bash -c "python3 gen/generate.py | grep -c '^wrote' | grep -qx 0"
run "verify lazy full after the plants (restored build)" dotnet "$BL/BenchDotNet.dll" --verify
fi

echo
[ $FAILS -eq 0 ] && echo "S17 $PART CHECKS PASSED (net8.0; fast checks, not the gate)" || echo "S17 $PART CHECKS: $FAILS FAILED"
exit $FAILS
