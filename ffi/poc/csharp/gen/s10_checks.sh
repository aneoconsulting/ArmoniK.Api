#!/usr/bin/env bash
# D23 checks (net8.0; CORRECTNESS ONLY, no timing): the FSM decode family's C# consumer.
#   gen/s10_checks.sh OUTDIR
# Cores: this slice's target-core* (gen/build_core.sh at the core commit), copied into every bin
# directory first; every binary rebuilt first. Exit 0 only if every run passes and every planted
# defect is caught.
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SLICE"
OUT="${1:?usage: gen/s10_checks.sh OUTDIR}"; mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
export SCRATCH="${SCRATCH:-$(mktemp -d)}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 MSBUILDDISABLENODEREUSE=1
RUST_EV="$SLICE/../../logs/rust/opt/d23-fsm/checks/events-counting.txt"
FAILS=0
run() { local name="$1"; shift; echo "\$ $*"; "$@"; local rc=$?; echo "exit status: $rc"; [ $rc -eq 0 ] || { echo "CHECK: $name FAILED"; FAILS=$((FAILS+1)); }; }
control() { local name="$1"; shift; echo "\$ $*   # a PLANT: must fail"; "$@" > "$SCRATCH/ctl.out" 2>&1; local rc=$?; grep -m3 "FAIL" "$SCRATCH/ctl.out"; tail -n 1 "$SCRATCH/ctl.out"; echo "exit status: $rc"
  if [ $rc -ne 0 ]; then echo "plant $name: caught"; else echo "PLANT $name NOT CAUGHT"; FAILS=$((FAILS+1)); fi; }
B="$SLICE/src/BenchDotNet/bin/Release/net8.0"; BN="$SLICE/src/BenchDotNet/bin-nounk/Release/net8.0"
BC="$SLICE/src/BenchDotNet/bin-count/Release/net8.0"; BCN="$SLICE/src/BenchDotNet/bin-count-nounk/Release/net8.0"
CB="$SLICE/src/Corpus/bin/Release/net8.0"; CBN="$SLICE/src/Corpus/bin-nounk/Release/net8.0"
H="$SLICE/src/Harness/bin/Release/net8.0"; HN="$SLICE/src/Harness/bin-nounk/Release/net8.0"
build_all() {
  for a in "" "-p:AkNounk=true"; do
    for pr in src/BenchDotNet/BenchDotNet.csproj src/Corpus/Corpus.csproj; do
      dotnet build $pr -c Release -f net8.0 $a > "$SCRATCH/build.log" 2>&1 || { cat "$SCRATCH/build.log"; exit 1; }
    done
    dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -f net8.0 -p:AkHostCount=true $a > "$SCRATCH/build.log" 2>&1 || { cat "$SCRATCH/build.log"; exit 1; }
  done
  for a in "" "-p:AkNounk=true"; do
    dotnet build src/Harness/Harness.csproj -c Release -f net8.0 $a > "$SCRATCH/build.log" 2>&1 || { cat "$SCRATCH/build.log"; exit 1; }
  done
  cp target-core/release/libak_core.so "$H/"; cp target-core-nounk/release/libak_core.so "$HN/"
  cp target-core/release/libak_core.so "$B/"; cp target-core-nounk/release/libak_core.so "$BN/"
  cp target-core-count/release/libak_core.so "$BC/"; cp target-core-count-nounk/release/libak_core.so "$BCN/"
  cp target-core-corpus/release/libak_core.so "$CB/"; cp target-core-corpus-nounk/release/libak_core.so "$CBN/"
}
echo "# s10_checks.sh at $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec/gen || echo ' + uncommitted'); $(date -u +%FT%TZ)"
echo "## 0. generators current: generate.py rewrites nothing"
python3 gen/generate.py > "$SCRATCH/gen.out" 2>&1; grep "^wrote" "$SCRATCH/gen.out"; run "generators current" bash -c "! grep -q '^wrote' '$SCRATCH/gen.out'"
build_all
echo "# cores: $(sha256sum "$B/libak_core.so" | cut -c1-16) (full) $(sha256sum "$BN/libak_core.so" | cut -c1-16) (nounk) $(sha256sum "$BC/libak_core.so" | cut -c1-16) (count) $(sha256sum "$BCN/libak_core.so" | cut -c1-16) (count nounk) $(sha256sum "$CB/libak_core.so" | cut -c1-16) (corpus) $(sha256sum "$CBN/libak_core.so" | cut -c1-16) (corpus nounk)"
echo "## 0b. layout: the C# declaration (ak_fsm_ev as amended included) against the probe of the Rust declaration, by name both ways + section 10; the planted swap must fail"
run "layout full" dotnet "$H/harness.dll" layout target-core/layout.json
control "layout plant" dotnet "$H/harness.dll" layout target-core/layout.json --plant
run "layout nounk" dotnet "$HN/harness.dll" layout target-core-nounk/layout.json
run "corpus layout" "$CB/corpus" --layout target-core-corpus/layout.json
run "corpus layout nounk" "$CBN/corpus" --layout target-core-corpus-nounk/layout.json
run "ak_fsm_ev in the probe" bash -c "grep -o '\"ak_fsm_ev\"[^]]*' target-core/layout.json"
echo "## 1. Cases.Verify (the pre-timing byte identity, P7.1's committed vector through every decode arm included), both builds"
run "verify full" dotnet "$B/BenchDotNet.dll" --verify
run "verify nounk" dotnet "$BN/BenchDotNet.dll" --verify
echo "## 2. --verify-fsm: push = pull = FSM graphs on every payload, content set and U-* row, drop and retain;"
echo "##    FSM calls = the Rust slice's events; malformed variants: FSM code = pull code, graphs where accepted"
run "verify-fsm full" bash -c "dotnet '$B/BenchDotNet.dll' --verify-fsm --events '$OUT/events-full.txt' --rust-events '$RUST_EV' | tee '$OUT/verify-fsm-full.log' | tail -n 3; exit \${PIPESTATUS[0]}"
run "verify-fsm nounk" bash -c "dotnet '$BN/BenchDotNet.dll' --verify-fsm --events '$OUT/events-nounk.txt' --rust-events '$RUST_EV' | tee '$OUT/verify-fsm-nounk.log' | tail -n 3; exit \${PIPESTATUS[0]}"
echo "## 3. the corpus (since D24 the ffi arms decode with the FSM and require push's and pull's code and re-encoding on every row), both builds; and the push arms alone (AK_CORPUS_PUSH=1)"
AK_CORPUS_RETAIN_STRICT=1 run "corpus full" bash -c "'$CB/corpus' > '$OUT/corpus-full.log' 2>&1; rc=\$?; grep -A12 '^## ffi-' '$OUT/corpus-full.log' | grep -v '^$' | head -30; tail -n 4 '$OUT/corpus-full.log'; exit \$rc"
run "corpus nounk" bash -c "'$CBN/corpus' > '$OUT/corpus-nounk.log' 2>&1; rc=\$?; grep -A12 '^## ffi-' '$OUT/corpus-nounk.log' | grep -v '^$' | head -15; tail -n 4 '$OUT/corpus-nounk.log'; exit \$rc"
AK_CORPUS_PUSH=1 AK_CORPUS_RETAIN_STRICT=1 run "corpus full with the push arms" bash -c "'$CB/corpus' > '$OUT/corpus-push-full.log' 2>&1; rc=\$?; tail -n 3 '$OUT/corpus-push-full.log'; exit \$rc"
run "corpus unk-controls" "$CB/corpus" --unk-controls
echo "## 4. counts: the committed counts unchanged (both builds); the eight-arm grid's counts per arm (AK_BDN_S10=1, core grid)"
run "counts full unchanged" bash -c "dotnet '$BC/BenchDotNet.dll' --counts '$SCRATCH/c.txt' > /dev/null && diff -q gen/counts.txt '$SCRATCH/c.txt'"
run "counts nounk unchanged" bash -c "dotnet '$BCN/BenchDotNet.dll' --counts '$SCRATCH/cn.txt' > /dev/null && diff -q gen/counts-nounk.txt '$SCRATCH/cn.txt'"
run "counts s10 grid" bash -c "AK_BDN_S10=1 AK_CAMPAIGN_GRID=core AK_BDN_DIRS=decode-read dotnet '$BC/BenchDotNet.dll' --counts '$OUT/counts-s10.txt' > /dev/null"
echo "## 5. planted defects in the generated FSM consumer (src/Harness/Generated/CoreFfi.cs), each must be caught by --verify-fsm"
G="src/Harness/Generated/CoreFfi.cs"
plant() {
  local name="$1" from="$2" to="$3"
  cp "$G" "$SCRATCH/CoreFfi.orig"
  python3 - "$G" "$from" "$to" <<'PY'
import sys
p, a, b = sys.argv[1:4]
s = open(p).read()
k = s.count(a)
assert k > 0, "plant anchor not found: " + a
open(p, "w").write(s.replace(a, b))
print("planted %d site(s)" % k)
PY
  dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -f net8.0 > "$SCRATCH/build.log" 2>&1 || { cat "$SCRATCH/build.log"; cp "$SCRATCH/CoreFfi.orig" "$G"; FAILS=$((FAILS+1)); return; }
  cp target-core/release/libak_core.so "$B/"
  control "$name" dotnet "$B/BenchDotNet.dll" --verify-fsm --variants 8
  cp "$SCRATCH/CoreFfi.orig" "$G"
}
plant "token ignored (every element group applied to element 0)" "[(int)ev->token], b); return;" "[0], b); return;"
plant "a run's last element lost" "int n = (int)ev->n;" "int n = (int)ev->n - (ev->n > 1 ? 1 : 0);"
plant "the root group not applied" "// The root group: always the last event, the end." "if (len >= 0) break;"
plant "an error read as the end" "rc = op < 0 ? op : 0;" "rc = 0;"
dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -f net8.0 > "$SCRATCH/build.log" 2>&1 || { cat "$SCRATCH/build.log"; exit 1; }
cp target-core/release/libak_core.so "$B/"
run "generated file restored" bash -c "python3 gen/generate.py | grep -c '^wrote' | grep -qx 0"
echo
[ $FAILS -eq 0 ] && echo "S10 CHECKS PASSED (net8.0; not the gate)" || echo "S10 CHECKS: $FAILS FAILED"
exit $FAILS
