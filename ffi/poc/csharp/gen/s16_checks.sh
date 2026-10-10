#!/usr/bin/env bash
# s16 (owner, 2026-10-10): the reset-on-entry VARIANT's checks (net8.0; CORRECTNESS ONLY, no
# timing): the C# host rendered with reset_on_entry=True (gen/generate.py roe_targets; builds
# /p:AkRoe=true [AkNounk, AkHostCount], and /p:AkRoeBench=true) against the variant cores
# (gen/s16_cores.sh). Exit 0 only if every run passes and every control fails.
#   gen/s16_checks.sh OUTDIR
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SLICE"
OUT="${1:?usage: gen/s16_checks.sh OUTDIR}"; mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
export SCRATCH="${SCRATCH:-$(mktemp -d)}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 MSBUILDDISABLENODEREUSE=1; mkdir -p "$SCRATCH"
RUST_EV="$SLICE/../../logs/rust/opt/d23-fsm/checks/events-counting.txt"
FAILS=0
run() { local name="$1"; shift; echo "\$ $*"; "$@"; local rc=$?; echo "exit status: $rc"; [ $rc -eq 0 ] || { echo "CHECK: $name FAILED"; FAILS=$((FAILS+1)); }; }
control() { local name="$1"; shift; echo "\$ $*   # a CONTROL: must fail"; "$@" > "$SCRATCH/ctl.out" 2>&1; local rc=$?; grep -m3 -E "FAIL|refuse|reset-on-entry" "$SCRATCH/ctl.out"; tail -n 1 "$SCRATCH/ctl.out"; echo "exit status: $rc"
  if [ $rc -ne 0 ]; then echo "control $name: failed, as required"; else echo "CONTROL $name DID NOT FAIL"; FAILS=$((FAILS+1)); fi; }
B="$SLICE/src/BenchDotNet/bin-roe/Release/net8.0"; BN="$SLICE/src/BenchDotNet/bin-roe-nounk/Release/net8.0"
BC="$SLICE/src/BenchDotNet/bin-roe-count/Release/net8.0"; BCN="$SLICE/src/BenchDotNet/bin-roe-count-nounk/Release/net8.0"
RB="$SLICE/src/BenchDotNet/bin-roebench/Release/net8.0"
CB="$SLICE/src/Corpus/bin-roe/Release/net8.0"; CBN="$SLICE/src/Corpus/bin-roe-nounk/Release/net8.0"
cores() {
  cp target-core-roe/release/libak_core.so "$B/"; cp target-core-nounk-roe/release/libak_core.so "$BN/"
  cp target-core-count-roe/release/libak_core.so "$BC/"; cp target-core-count-nounk-roe/release/libak_core.so "$BCN/"
  cp target-core-roe/release/libak_core.so "$RB/"
  cp target-core-corpus-roe/release/libak_core.so "$CB/"; cp target-core-corpus-nounk-roe/release/libak_core.so "$CBN/"
}
build_all() {
  for a in "-p:AkRoe=true" "-p:AkRoe=true -p:AkNounk=true" "-p:AkRoe=true -p:AkHostCount=true" "-p:AkRoe=true -p:AkHostCount=true -p:AkNounk=true" "-p:AkRoeBench=true"; do
    dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release $a > "$SCRATCH/build.log" 2>&1 || { cat "$SCRATCH/build.log"; exit 1; }
  done
  for a in "-p:AkRoe=true" "-p:AkRoe=true -p:AkNounk=true"; do
    dotnet build src/Corpus/Corpus.csproj -c Release -f net8.0 $a > "$SCRATCH/build.log" 2>&1 || { cat "$SCRATCH/build.log"; exit 1; }
  done
  cores
}
echo "# s16_checks.sh at $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec/gen || echo ' + uncommitted'); $(date -u +%FT%TZ); boot $(uptime -s)"
echo "## 0. generators current (the default outputs and the variant's); the default builds unchanged by the variant's files"
python3 gen/generate.py > "$SCRATCH/gen.out" 2>&1; grep "^wrote" "$SCRATCH/gen.out"; run "generators current" bash -c "! grep -q '^wrote' '$SCRATCH/gen.out'"
run "no thread-local storage" python3 -I gen/no_tls.py src
build_all
echo "# cores: $(for d in "$B" "$BN" "$BC" "$BCN" "$RB" "$CB" "$CBN"; do echo -n "$(basename "$(dirname "$(dirname "$(dirname "$d")")")")/$(basename "$(dirname "$(dirname "$d")")") $(sha256sum "$d/libak_core.so" | cut -c1-16); "; done)"
echo "## 1. the variant binding refuses a core without the feature (the marker export)"
control "variant on the default core" env AK_CORE_LIB="$SLICE/target-core/release/libak_core.so" dotnet "$B/BenchDotNet.dll" --verify
echo "## 2. Cases.Verify: byte identity of every arm, every string path (E0 E1 E2 ETH:16 E3 E3L E1R E1C ...), retain and drop, both builds"
run "verify roe full" dotnet "$B/BenchDotNet.dll" --verify
run "verify roe nounk" dotnet "$BN/BenchDotNet.dll" --verify
echo "## 3. --verify-mt (8 threads) per path, both builds; pin stress (1 thread) for the frame paths; the early-unpin control"
for m in E0 E1 E2 E3 E3L E1R E1C E1R:128; do
  AK_STR_ENC=$m run "verify-mt $m roe full" dotnet "$B/BenchDotNet.dll" --verify-mt
  AK_STR_ENC=$m run "verify-mt $m roe nounk" dotnet "$BN/BenchDotNet.dll" --verify-mt
done
for m in E1R E1C; do AK_GATE_PIN_STRESS=1 AK_STR_ENC=$m run "verify-mt $m 1 thread, pin stress" dotnet "$B/BenchDotNet.dll" --verify-mt --threads 1 --rounds 1; done
AK_GATE_PLANT_EARLY_UNPIN=1 AK_GATE_PIN_STRESS=1 AK_STR_ENC=E1C control "verify-mt E1C 1 thread, handles released before the call + compacting GC" dotnet "$B/BenchDotNet.dll" --verify-mt --threads 1 --rounds 1
echo "## 4. --verify-fsm: push = pull = FSM on every payload, content set and U-* row, drop and retain; FSM calls = the Rust slice's events"
run "verify-fsm roe full" bash -c "dotnet '$B/BenchDotNet.dll' --verify-fsm --events '$OUT/events-full.txt' --rust-events '$RUST_EV' | tee '$OUT/verify-fsm-full.log' | tail -n 3; exit \${PIPESTATUS[0]}"
run "verify-fsm roe nounk" bash -c "dotnet '$BN/BenchDotNet.dll' --verify-fsm --events '$OUT/events-nounk.txt' --rust-events '$RUST_EV' | tee '$OUT/verify-fsm-nounk.log' | tail -n 3; exit \${PIPESTATUS[0]}"
echo "## 5. the corpus (four arms, retain strict), both builds, E0 and E1R; E1R / E1C K 3 under pin stress; the push arms; the unknown-field controls"
for m in E0 E1R; do
  AK_STR_ENC=$m AK_CORPUS_RETAIN_STRICT=1 run "corpus roe full $m" bash -c "'$CB/corpus' > '$OUT/corpus-full-$m.log' 2>&1; rc=\$?; tail -n 3 '$OUT/corpus-full-$m.log'; exit \$rc"
  AK_STR_ENC=$m run "corpus roe nounk $m" bash -c "'$CBN/corpus' > '$OUT/corpus-nounk-$m.log' 2>&1; rc=\$?; tail -n 3 '$OUT/corpus-nounk-$m.log'; exit \$rc"
done
for m in E1R E1C; do AK_CORPUS_RETAIN_STRICT=1 AK_STR_PINK=3 AK_GATE_PIN_STRESS=1 AK_STR_ENC=$m run "corpus roe $m K 3, pin stress" bash -c "'$CB/corpus' --timeout-ms 300000 > '$OUT/corpus-pinstress-$m.log' 2>&1; rc=\$?; tail -n 2 '$OUT/corpus-pinstress-$m.log'; exit \$rc"; done
AK_CORPUS_PUSH=1 AK_CORPUS_RETAIN_STRICT=1 run "corpus roe full with the push arms" bash -c "'$CB/corpus' > '$OUT/corpus-push-full.log' 2>&1; rc=\$?; tail -n 3 '$OUT/corpus-push-full.log'; exit \$rc"
run "corpus roe unk-controls" bash -c "'$CB/corpus' --unk-controls > '$OUT/unk-controls.log' 2>&1; rc=\$?; tail -n 4 '$OUT/unk-controls.log'; exit \$rc"
echo "## 6. counts of the variant (both builds) against the committed ones: fwd = default fwd - the resets it no longer makes"
run "counts roe full" bash -c "dotnet '$BC/BenchDotNet.dll' --counts '$OUT/counts-roe.txt' > /dev/null && python3 -I gen/s16_counts.py gen/counts.txt '$OUT/counts-roe.txt' > '$OUT/counts-roe-change.md'; rc=\$?; tail -n 12 '$OUT/counts-roe-change.md'; exit \$rc"
run "counts roe nounk" bash -c "dotnet '$BCN/BenchDotNet.dll' --counts '$OUT/counts-roe-nounk.txt' > /dev/null && python3 -I gen/s16_counts.py gen/counts-nounk.txt '$OUT/counts-roe-nounk.txt' > '$OUT/counts-roe-nounk-change.md'; rc=\$?; tail -n 8 '$OUT/counts-roe-nounk-change.md'; exit \$rc"
echo "## 7. planted defects in the variant's generated FSM consumer (src/Harness/GeneratedRoe/CoreFfi.cs), each caught by --verify-fsm"
G="src/Harness/GeneratedRoe/CoreFfi.cs"
plant() {
  local name="$1" from="$2" to="$3"
  cp "$G" "$SCRATCH/CoreFfiRoe.orig"
  python3 - "$G" "$from" "$to" <<'PY'
import sys
p, a, b = sys.argv[1:4]
s = open(p).read()
k = s.count(a)
assert k > 0, "plant anchor not found: " + a
open(p, "w").write(s.replace(a, b))
print("planted %d site(s)" % k)
PY
  dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkRoe=true > "$SCRATCH/build.log" 2>&1 || { cat "$SCRATCH/build.log"; cp "$SCRATCH/CoreFfiRoe.orig" "$G"; FAILS=$((FAILS+1)); return; }
  cp target-core-roe/release/libak_core.so "$B/"
  control "$name" dotnet "$B/BenchDotNet.dll" --verify-fsm --variants 8
  cp "$SCRATCH/CoreFfiRoe.orig" "$G"
}
plant "token ignored (every element group applied to element 0)" "[(int)ev->token], b" "[0], b"
plant "a run's last element lost" "int n = (int)ev->n;" "int n = (int)ev->n - (ev->n > 1 ? 1 : 0);"
plant "the root group not applied" "// The root group: always the last event, the end." "if (len >= 0) break;"
plant "an error read as the end" "rc = op < 0 ? op : 0;" "rc = 0;"
echo "## 7b. a planted variant defect: the variant also skips the reset that CHANGES the options pointer (retain after drop decodes with the old pointer); --verify must fail"
plant_v() {
  cp "$G" "$SCRATCH/CoreFfiRoe.orig"
  python3 - "$G" <<'PY'
import sys
p = sys.argv[1]; s = open(p).read()
a = "        if (h.CoreArmed) return 0;\n"
k = s.count(a); assert k > 0
open(p, "w").write(s.replace(a, "        if (true) { h.CoreArmed = true; return 0; }\n"))
print("planted %d site(s)" % k)
PY
  dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkRoe=true > "$SCRATCH/build.log" 2>&1 || { cat "$SCRATCH/build.log"; cp "$SCRATCH/CoreFfiRoe.orig" "$G"; FAILS=$((FAILS+1)); return; }
  cp target-core-roe/release/libak_core.so "$B/"
  control "retain decode never armed" dotnet "$B/BenchDotNet.dll" --verify
  cp "$SCRATCH/CoreFfiRoe.orig" "$G"
}
plant_v
dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkRoe=true > "$SCRATCH/build.log" 2>&1 || { cat "$SCRATCH/build.log"; exit 1; }
cp target-core-roe/release/libak_core.so "$B/"
run "generated file restored" bash -c "python3 gen/generate.py | grep -c '^wrote' | grep -qx 0"
echo "## 8. the in-process bench's identity and reset bookkeeping on every case (one round, short blocks; no timing used)"
run "roebench identity" bash -c "AK_CAMPAIGN_GRID=core AK_BDN_DROP=1 dotnet '$RB/BenchDotNet.dll' --roebench '$OUT/roebench-smoke.tsv' --rounds 1 --block-ms 1 --warm-ms 5 | tail -n 1"
echo
[ $FAILS -eq 0 ] && echo "S16 CHECKS PASSED (net8.0; not the gate)" || echo "S16 CHECKS: $FAILS FAILED"
exit $FAILS
