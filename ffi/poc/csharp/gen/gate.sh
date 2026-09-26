#!/usr/bin/env bash
# THE C# CORRECTNESS GATE (FIX-PLAN WP5 step 4). Nothing is timed: every figure a container
# prints is instrumentation (README 1.1) and none is taken here.
#
#   1  generator: --check (drift + the one-generator guard, seen failing on a plant)
#   2  the core, from the committed poc/codec, every build WITH init-guard (R-G7), and the
#      layout probe (the RUST declaration, parsed from its source text, R-E6)
#   3  builds: net8.0 (target), net6.0 (floor, self-contained on the NuGet 6.0 runtime pack),
#      net48 (floor, COMPILE ONLY: no .NET Framework and no Mono here)
#   4  net8.0 harness: payload byte identity, unknown fields (drop + retain), groups, UTF-8
#      policy, map forms, layout by name + section 10, core-ffi every shape (plain, counting,
#      R5), and the controls that must FAIL (a planted layout swap; ak_init skipped)
#   5  net6.0 harness: the same, core-ffi included (it did not build before this unit)
#   6  akrpc (net8.0): plan.rpc's structs by name; R-D9's error path, counted
#   7  the corpus, net8.0 and net6.0: managed-drop, managed-retain, ffi-drop, ffi-retain, each
#      row in a child process under a timeout; then the controls that must FAIL; then the
#      oracle-probe rows (poc/rust/gen/probe_corpus.py)
#
#   SCRATCH=dir gen/gate.sh      (SCRATCH holds the core snapshot; default: mktemp -d)
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SLICE="$(cd "$HERE/.." && pwd)"
REPO="$(git -C "$SLICE" rev-parse --show-toplevel)"
export SCRATCH="${SCRATCH:-$(mktemp -d)}"
export DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
cd "$SLICE"
FAILS=0
H8="$SLICE/src/Harness/bin/Release/net8.0"
H6="$SLICE/src/Harness/bin/publish-net6"
C8="$SLICE/src/Corpus/bin/Release/net8.0"
C6="$SLICE/src/Corpus/bin/publish-net6"
R8="$SLICE/src/Rpc/bin/Release/net8.0"
LAY="$SLICE/target-core/layout.json"
LAYC="$SLICE/target-core-corpus/layout.json"

step() { echo; echo "################ $*"; }
# run NAME must-pass command...
run() {
  local name="$1"; shift
  echo "\$ $*"
  "$@"; local rc=$?
  echo "exit status: $rc"
  if [ $rc -ne 0 ]; then echo "GATE: $name FAILED"; FAILS=$((FAILS+1)); fi
}
# control NAME command...  (must exit non-zero)
control() {
  local name="$1"; shift
  echo "\$ $*   # a CONTROL: must fail"
  "$@" > "$SCRATCH/control.out" 2>&1; local rc=$?
  grep -E "FAIL|failure|THREW|PLANTED|CORPUS|arm-row|^## |   pass " "$SCRATCH/control.out" | head -16
  echo "exit status: $rc"
  if [ $rc -eq 0 ]; then echo "GATE: control $name PASSED -- the gate is blind to it"; FAILS=$((FAILS+1));
  else echo "control $name: failed, as required"; fi
}
core() {  # dir variant  -- put a core build next to a harness
  cp "$SLICE/$2/release/libak_core.so" "$1/libak_core.so"
  echo "# core in $(basename "$(dirname "$1")")/$(basename "$1"): $2 ($(sha256sum "$1/libak_core.so" | cut -c1-16))"
}

echo "# csharp slice gate (FIX-PLAN WP5 and WP6). CORRECTNESS ONLY: no timing is taken."
echo "# date:        $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "# branch HEAD: $(git -C "$REPO" rev-parse --short HEAD)$(git -C "$REPO" diff --quiet HEAD -- ffi/poc/csharp ffi/poc/codec/gen/cs_*.py || echo ' + the uncommitted slice changes this log is committed with')"
echo "# dotnet:      SDK $(dotnet --version); runtimes: $(dotnet --list-runtimes | grep NETCore | awk '{print $2}' | tr '\n' ' ')+ Microsoft.NETCore.App.Runtime.linux-x64 6.0.36 from NuGet (self-contained)"
echo "# corpus:      ffi/corpus at $(git -C "$REPO" log -1 --format=%h -- ffi/corpus), $(python3 -S -c 'import json;print(len(json.load(open("'"$REPO"'/ffi/corpus/generated/manifest.json"))["vectors"]))') vectors"
echo "# machine:     container, $(nproc) vCPU, $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //'), $(uname -r)"

step "1. generator (rendered from a snapshot of the COMMITTED poc/codec/gen, like the core)"
rm -rf "$SCRATCH/cg"; mkdir -p "$SCRATCH/cg"
( cd "$REPO" && git archive HEAD ffi/poc/codec ffi/schema ffi/corpus | tar -x -C "$SCRATCH/cg" )
AK_CODECGEN="$SCRATCH/cg/ffi/poc/codec/gen" run "generate --check" python3 -S gen/generate.py --check

step "2. the core (every build with init-guard) and the layout probe"
run "build_core" gen/build_core.sh

step "3. builds"
b() { echo "\$ dotnet $*"; dotnet "$@" > "$SCRATCH/build.out" 2>&1; local rc=$?; grep -E " error |Build succeeded|->" "$SCRATCH/build.out" | sed "s|$REPO/||" | grep -v "^ *$" | tail -4; echo "exit status: $rc"; [ $rc -eq 0 ] || { cat "$SCRATCH/build.out" | tail -30; FAILS=$((FAILS+1)); }; }
b build src/Harness/Harness.csproj -c Release -f net8.0
b publish src/Harness/Harness.csproj -c Release -f net6.0 -r linux-x64 --self-contained -o "$H6"
b build src/Corpus/Corpus.csproj -c Release -f net8.0
b publish src/Corpus/Corpus.csproj -c Release -f net6.0 -r linux-x64 --self-contained -o "$C6"
b build src/Rpc/Rpc.csproj -c Release
b build src/BenchDotNet/BenchDotNet.csproj -c Release
b build src/HarnessFloor/HarnessFloor.csproj -c Release
echo "# which import form each level compiled (the one generated Abi.cs, #if NET7_0_OR_GREATER):"
for d in "$H8/harness.dll" "$H6/harness.dll" "$SLICE/src/HarnessFloor/bin/Release/net48/harness48.exe"; do
  echo "#   $(echo "$d" | sed "s|$SLICE/||"): LibraryImportAttribute referenced $(grep -c -a LibraryImportAttribute "$d") time(s)"
done
echo "# net48: COMPILED ONLY (the P/Invoke binding's DllImport branch included; the host half is"
echo "#        #if NET5_0_OR_GREATER, no UnmanagedCallersOnly on .NET Framework). Nothing runs it here:"
echo "#        .NET Framework needs Windows, and this container has no Mono either."

for lvl in 8 6; do
  if [ $lvl = 8 ]; then H="$H8"; HX=(dotnet "$H8/harness.dll"); else H="$H6"; HX=("$H6/harness"); fi
  step "$((lvl == 8 ? 4 : 5)). net${lvl}.0 harness"
  core "$H" target-core
  run "net$lvl conformance" "${HX[@]}" conformance
  run "net$lvl unknown" "${HX[@]}" unknown
  run "net$lvl groups" "${HX[@]}" groups
  run "net$lvl utf8" "${HX[@]}" utf8
  run "net$lvl mapforms" "${HX[@]}" mapforms
  run "net$lvl layout" "${HX[@]}" layout "$LAY"
  control "net$lvl layout plant" "${HX[@]}" layout "$LAY" --plant
  AK_LAYOUT_PROBE="$LAY" run "net$lvl coreffi" "${HX[@]}" coreffi
  core "$H" target-core-count
  AK_CROSSINGS_EXPECT="$SLICE/gen/crossings.txt" run "net$lvl coreffi counting (R5, counts = gen/crossings.txt)" "${HX[@]}" coreffi
  # R-H3: the crossing gate must fail on a committed row this run does not produce, and on
  # an empty expect file.
  { cat "$SLICE/gen/crossings.txt"; echo "P9.9 1 1 1 1 1 1"; } > "$SCRATCH/cx-extra.txt"; : > "$SCRATCH/cx-empty.txt"
  AK_CROSSINGS_EXPECT="$SCRATCH/cx-extra.txt" control "net$lvl crossing gate, a committed row not produced (R-H3)" "${HX[@]}" coreffi
  AK_CROSSINGS_EXPECT="$SCRATCH/cx-empty.txt" control "net$lvl crossing gate, an empty expect file (R-H3)" "${HX[@]}" coreffi
  core "$H" target-core
  AK_GATE_PLANT_NO_INIT=1 control "net$lvl coreffi without ak_init (init-guard core)" "${HX[@]}" coreffi
done

step "6. akrpc (net8.0): the generated RPC binding"
core "$R8" target-core-count
run "akrpc layout" dotnet "$R8/akrpc.dll" --layout "$LAY"
run "akrpc error path" dotnet "$R8/akrpc.dll" --error-path

for lvl in 8 6; do
  if [ $lvl = 8 ]; then CX=("$C8/corpus"); C="$C8"; else CX=("$C6/corpus"); C="$C6"; fi
  step "7.$lvl the corpus, net${lvl}.0"
  core "$C" target-core-corpus
  run "net$lvl corpus layout" "${CX[@]}" --layout "$LAYC"
  # Since decision 11's port (WP5 step 9) a retain arm that writes the dropped form on a
  # non-disputed unknown-class row FAILS the gate (AK_CORPUS_RETAIN_STRICT=1).
  AK_CORPUS_RETAIN_STRICT=1 run "net$lvl corpus" "${CX[@]}"
  AK_CORPUS_RETAIN_STRICT=1 AK_CORPUS_PLANT=unkdrop control "net$lvl corpus unkdrop (ffi-retain in drop mode, strict retain)" "${CX[@]}" --only "U-"
  run "net$lvl decision 11 controls (discard per position, pull == push, wrong root)" "${CX[@]}" --unk-controls
  control "net$lvl decision 11 plant (the expectation's clearing skipped)" "${CX[@]}" --unk-controls --plant
  AK_GATE_PLANT_SKIP_RELEASE=1 control "net$lvl decision 11 skipped release (the undelivered check's twin, R-H9)" "${CX[@]}" --only U- --unk-controls
  SUB="S-Probe,U-root,X-lenwrap-lrr,E-map,T-dec-root"
  AK_CORPUS_PLANT=proj control "net$lvl corpus proj" "${CX[@]}" --only "$SUB"
  AK_CORPUS_PLANT=reenc control "net$lvl corpus reenc" "${CX[@]}" --only "$SUB"
  AK_CORPUS_PLANT=accept control "net$lvl corpus accept" "${CX[@]}" --only "$SUB"
  AK_CORPUS_PLANT=code control "net$lvl corpus code (C4 compares the refusal code, R-H14)" "${CX[@]}" --only "$SUB"
  AK_GATE_PLANT_NO_INIT=1 control "net$lvl corpus noinit" "${CX[@]}" --only "$SUB"
  # The oracle-probe rows (poc/rust/gen/probe_corpus.py, the majority reading of upb and
  # protobuf C++): the varint 10th byte, the field-number limit at the top and inside a
  # group (D38), the map-entry order.
  rm -rf "$SCRATCH/probe"; python3 -S "$REPO/ffi/poc/rust/gen/probe_corpus.py" "$SCRATCH/probe" > /dev/null
  run "net$lvl probe rows" "${CX[@]}" --manifest "$SCRATCH/probe/manifest.json"
done

# ============================================================== WP5 step 10
# The NO-UNKNOWN variant: unknown fields COMPILED OUT of the core (ak-core
# --no-default-features, target-core*-nounk) and of this binding (/p:AkNounk=true:
# GeneratedNounk/, rendered from the plans relowered with unknown="drop"; bin-nounk/).
# Gated on its own: the loaded core is the variant (no u-family export), its layout (240
# facts shapes, 340 corpus), byte identity, its own crossing counts, the corpus with every
# unknown row in the dropped form, rule 6.
step "8. the NO-UNKNOWN variant (WP5 step 10): its own build, core and gate"
HN8="$SLICE/src/Harness/bin-nounk/Release/net8.0"; HN6="$SLICE/src/Harness/bin-nounk/publish-net6"
CN8="$SLICE/src/Corpus/bin-nounk/Release/net8.0"; CN6="$SLICE/src/Corpus/bin-nounk/publish-net6"
b build src/Harness/Harness.csproj -c Release -f net8.0 -p:AkNounk=true
b publish src/Harness/Harness.csproj -c Release -f net6.0 -r linux-x64 --self-contained -p:AkNounk=true -o "$HN6"
b build src/Corpus/Corpus.csproj -c Release -f net8.0 -p:AkNounk=true
b publish src/Corpus/Corpus.csproj -c Release -f net6.0 -r linux-x64 --self-contained -p:AkNounk=true -o "$CN6"
b build src/Rpc/Rpc.csproj -c Release -p:AkNounk=true
b build src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkNounk=true
for lvl in 8 6; do
  if [ $lvl = 8 ]; then H="$HN8"; HX=(dotnet "$HN8/harness.dll"); C="$CN8"; CX=(dotnet "$CN8/corpus.dll");
  else H="$HN6"; HX=("$HN6/harness"); C="$CN6"; CX=("$CN6/corpus"); fi
  step "8.$lvl no-unknown, net${lvl}.0"
  core "$H" target-core-nounk
  run "net$lvl nounk layout (by name both ways, 240 section-10 facts)" "${HX[@]}" layout "$SLICE/target-core-nounk/layout.json"
  run "net$lvl nounk conformance (byte identity)" "${HX[@]}" conformance
  run "net$lvl nounk coreffi (the loaded core is the variant)" "${HX[@]}" coreffi
  core "$H" target-core
  control "net$lvl nounk binding on the FULL core (the variant check must refuse it)" "${HX[@]}" coreffi
  core "$H" target-core-count-nounk
  AK_CROSSINGS_EXPECT="$SLICE/gen/crossings-nounk.txt" run "net$lvl nounk coreffi counting (counts = gen/crossings-nounk.txt)" "${HX[@]}" coreffi
  core "$H" target-core-nounk
  core "$C" target-core-corpus-nounk
  run "net$lvl nounk corpus variant" "${CX[@]}" --variant
  run "net$lvl nounk corpus layout" "${CX[@]}" --layout "$SLICE/target-core-corpus-nounk/layout.json"
  AK_CORPUS_DROP_STRICT=1 run "net$lvl nounk corpus (every unknown row in the dropped form)" "${CX[@]}"
  run "net$lvl nounk wrong root" "${CX[@]}" --wrong-root
done
echo "# crossing counts, no-unknown against the full build's (gen/crossings-nounk.txt vs gen/crossings.txt):"
diff <(grep -v '^#' "$SLICE/gen/crossings.txt") <(grep -v '^#' "$SLICE/gen/crossings-nounk.txt") | sed 's/^/#   /'

echo
if [ $FAILS -eq 0 ]; then echo "GATE PASSED"; else echo "GATE FAILED: $FAILS"; fi
exit $FAILS
