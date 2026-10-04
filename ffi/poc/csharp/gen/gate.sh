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
#   8  the no-unknown variant (WP5 step 10): its own build, core, counts and corpus
#   9  the counting builds (WP7, CAMPAIGN req 19 as amended): every entry point per codec
#      case and per RPC call, against gen/counts*.txt and gen/rpc-counts*.txt; the upload
#      directions' check (WP8, req 14 as amended: c accepted, d count and SHA-256) and its plants
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
# AK_GATE_LEVELS (default "8 6"): "8" runs the net8.0 checks only (no net6.0 floor, no net48
# compile): a quick check during optimisation work, NOT the gate (the last line says so).
LEVELS="${AK_GATE_LEVELS:-8 6}"
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
# D11 as amended (2026-10-03): AK_H2=stock (default) or h2-batch selects the core variant of
# every core with the transport (target-core, -count, -nounk, -count-nounk -> their -h2b twins,
# gen/build_core.sh); the corpus cores have no transport and no twin.
AK_H2="${AK_H2:-stock}"
core() {  # dir variant  -- put a core build next to a harness
  local v="$2"
  if [ "$AK_H2" = h2-batch ] && [ -d "$SLICE/$2-h2b" ]; then v="$2-h2b"; fi
  cp "$SLICE/$v/release/libak_core.so" "$1/libak_core.so"
  echo "# core in $(basename "$(dirname "$1")")/$(basename "$1"): $v ($(sha256sum "$1/libak_core.so" | cut -c1-16); h2: $(strings "$1/libak_core.so" | grep -o '[^/]*/src/codec/framed_write\.rs' | sort -u | head -1))"
  return 0
  echo "# core in $(basename "$(dirname "$1")")/$(basename "$1"): $2 ($(sha256sum "$1/libak_core.so" | cut -c1-16))"
}

echo "# csharp slice gate (FIX-PLAN WP5 to WP8). CORRECTNESS ONLY: no timing is taken."
echo "# date:        $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "# levels:      $LEVELS (AK_GATE_LEVELS)"
echo "# h2 variant:  $AK_H2 (D11 as amended: every core with the transport is the $AK_H2 build)"
echo "# branch HEAD: $(git -C "$REPO" rev-parse --short HEAD)$(git -C "$REPO" diff --quiet HEAD -- ffi/poc/csharp ffi/poc/codec/gen/cs_*.py || echo ' + the uncommitted slice changes this log is committed with')"
echo "# dotnet:      SDK $(dotnet --version); runtimes: $(dotnet --list-runtimes | grep NETCore | awk '{print $2}' | tr '\n' ' ')+ Microsoft.NETCore.App.Runtime.linux-x64 6.0.36 from NuGet (self-contained)"
echo "# corpus:      ffi/corpus at $(git -C "$REPO" log -1 --format=%h -- ffi/corpus), $(python3 -S -c 'import json;print(len(json.load(open("'"$REPO"'/ffi/corpus/generated/manifest.json"))["vectors"]))') vectors"
echo "# machine:     container, $(nproc) vCPU, $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //'), $(uname -r)"

step "1. generator (rendered from a snapshot of the COMMITTED poc/codec/gen, like the core)"
rm -rf "$SCRATCH/cg"; mkdir -p "$SCRATCH/cg"
( cd "$REPO" && git archive HEAD ffi/poc/codec ffi/schema ffi/corpus | tar -x -C "$SCRATCH/cg" )
AK_CODECGEN="$SCRATCH/cg/ffi/poc/codec/gen" run "generate --check" python3 -S gen/generate.py --check

step "2. the core (every build with init-guard) and the layout probe"
if [ "$LEVELS" != "8 6" ] && [ "${AK_GATE_KEEP_CORE:-0}" = 1 ]; then
  echo "# AK_GATE_KEEP_CORE=1 (quick checks only): the cores already built are used, not rebuilt"
else
  run "build_core" gen/build_core.sh
fi

step "3. builds"
b() { echo "\$ dotnet $*"; dotnet "$@" > "$SCRATCH/build.out" 2>&1; local rc=$?; grep -E " error |Build succeeded|->" "$SCRATCH/build.out" | sed "s|$REPO/||" | grep -v "^ *$" | tail -4; echo "exit status: $rc"; [ $rc -eq 0 ] || { cat "$SCRATCH/build.out" | tail -30; FAILS=$((FAILS+1)); }; }
b build src/Harness/Harness.csproj -c Release -f net8.0
[[ " $LEVELS " == *" 6 "* ]] && b publish src/Harness/Harness.csproj -c Release -f net6.0 -r linux-x64 --self-contained -o "$H6"
b build src/Corpus/Corpus.csproj -c Release -f net8.0
[[ " $LEVELS " == *" 6 "* ]] && b publish src/Corpus/Corpus.csproj -c Release -f net6.0 -r linux-x64 --self-contained -o "$C6"
b build src/Rpc/akrpc.csproj -c Release
b build src/BenchDotNet/BenchDotNet.csproj -c Release
[[ " $LEVELS " == *" 6 "* ]] && b build src/HarnessFloor/HarnessFloor.csproj -c Release
echo "# which import form each level compiled (the one generated Abi.cs, #if NET7_0_OR_GREATER):"
for d in "$H8/harness.dll" "$H6/harness.dll" "$SLICE/src/HarnessFloor/bin/Release/net48/harness48.exe"; do
  echo "#   $(echo "$d" | sed "s|$SLICE/||"): LibraryImportAttribute referenced $(grep -c -a LibraryImportAttribute "$d") time(s)"
done
echo "# net48: COMPILED ONLY (the P/Invoke binding's DllImport branch included; the host half is"
echo "#        #if NET5_0_OR_GREATER, no UnmanagedCallersOnly on .NET Framework). Nothing runs it here:"
echo "#        .NET Framework needs Windows, and this container has no Mono either."

for lvl in $LEVELS; do
  if [ $lvl = 8 ]; then H="$H8"; HX=(dotnet "$H8/harness.dll"); else H="$H6"; HX=("$H6/harness"); fi
  step "$((lvl == 8 ? 4 : 5)). net${lvl}.0 harness"
  core "$H" target-core
  run "net$lvl conformance" "${HX[@]}" conformance
  run "net$lvl unknown" "${HX[@]}" unknown
  run "net$lvl groups" "${HX[@]}" groups
  run "net$lvl utf8" "${HX[@]}" utf8
  run "net$lvl mapforms" "${HX[@]}" mapforms
  # Step a2 (i): a host failure from a reverse call reaches the caller through ak_decode_*'s
  # return value alone (no ak_dec_err): the planted failure must come back as AK_ERR_HOST.
  run "net$lvl hostfail" "${HX[@]}" hostfail
  AK_GATE_PLANT_HOST_FAIL=apply run "net$lvl hostfail, apply throws (AK_ERR_HOST from the return value)" "${HX[@]}" hostfail
  AK_GATE_PLANT_HOST_FAIL=add run "net$lvl hostfail, add/new throws (AK_ERR_HOST from the return value)" "${HX[@]}" hostfail
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
# FIX-PLAN WP10: the RPC server is the Rust slice's tonic rpc_server for every slice
# (poc/rust/SERVER.md), built, started and stopped through poc/rust/serve.sh; its own state
# file here, so the gate never meets another run's server.
SERVE="$REPO/ffi/poc/rust/serve.sh"
export AK_SERVE_STATE="$SCRATCH/ak-rpc-server.state"
run "the campaign server (serve.sh build)" "$SERVE" build
# D10: the RPC checks run over TCP 127.0.0.1, the timed transport (the server's TCP listener,
# AK_SERVER_TCP=0: any free port, pinned server configuration, TCP_NODELAY on accept).
AK_SERVER_TCP=0 "$SERVE" start --out "$SCRATCH/srv" > "$SCRATCH/srv.start" 2>&1 || cat "$SCRATCH/srv.start"
SOCK="tcp:$(sed -n 's/^tcp //p' "$SCRATCH/srv.start")"
trap '"$SERVE" stop > /dev/null 2>&1' EXIT
echo "# the campaign server: $(cat "$SCRATCH/srv.start" | tr '\n' ' ')"
core "$R8" target-core-count
run "akrpc layout" dotnet "$R8/akrpc.dll" --layout "$LAY"
run "akrpc error path (against the campaign server: Fetch ok, StatusU13 a non-OK status)" dotnet "$R8/akrpc.dll" --error-path --sock "$SOCK"
# D7 (2026-10-04): every core delivery the delivery cells use (callback, callback with inline
# continuations, queue with one drainer) reports status, a non-OK status and a cancel as the
# blocking delivery does, unary (copy and move) and client streaming (copy and move sends).
core "$R8" target-core
run "akrpc delivery semantics (D7)" dotnet "$R8/akrpc.dll" --delivery-semantics --sock "$SOCK"
core "$R8" target-core-count

for lvl in $LEVELS; do
  if [ $lvl = 8 ]; then CX=("$C8/corpus"); C="$C8"; else CX=("$C6/corpus"); C="$C6"; fi
  step "7.$lvl the corpus, net${lvl}.0"
  core "$C" target-core-corpus
  run "net$lvl corpus layout" "${CX[@]}" --layout "$LAYC"
  # Since decision 11's port (WP5 step 9) a retain arm that writes the dropped form on a
  # non-disputed unknown-class row FAILS the gate (AK_CORPUS_RETAIN_STRICT=1).
  AK_CORPUS_RETAIN_STRICT=1 run "net$lvl corpus" "${CX[@]}"
  AK_CORPUS_RETAIN_STRICT=1 AK_CORPUS_PLANT=unkdrop control "net$lvl corpus unkdrop (ffi-retain in drop mode, strict retain)" "${CX[@]}" --only "U-"
  run "net$lvl decision 11 controls (discard per position, pull == push, wrong root)" "${CX[@]}" --unk-controls
  # D20 (2026-10-04): the core skips every string's UTF-8 check (utf8_skip all bits); the host's
  # strict decoder must refuse the malformed-UTF-8 rows (T-dec-*, AK_ERR_TRANSCODE). With a lossy
  # decoder planted, those rows are accepted and the corpus must fail.
  AK_GATE_PLANT_LOSSY=1 control "net$lvl corpus with a lossy host string decoder (D20: the host's check is live)" "${CX[@]}" --only "T-dec-"
  # D21 (2026-10-04): the string encode paths other than the default E0 (AK_STR_ENC): E1 (pinned
  # UTF-16, ak_tc_utf16) and E2 (the C# transcoder) over the whole corpus; planted one code
  # unit / one byte short (AK_GATE_PLANT_STR), each must fail (the path is live).
  for m in E1 E2; do
    AK_CORPUS_RETAIN_STRICT=1 AK_STR_ENC=$m run "net$lvl corpus, string path $m" "${CX[@]}"
    AK_GATE_PLANT_STR=1 AK_STR_ENC=$m control "net$lvl corpus, string path $m planted short (D21: the path is live)" "${CX[@]}" --only "S-"
  done
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
[[ " $LEVELS " == *" 6 "* ]] && b publish src/Harness/Harness.csproj -c Release -f net6.0 -r linux-x64 --self-contained -p:AkNounk=true -o "$HN6"
b build src/Corpus/Corpus.csproj -c Release -f net8.0 -p:AkNounk=true
[[ " $LEVELS " == *" 6 "* ]] && b publish src/Corpus/Corpus.csproj -c Release -f net6.0 -r linux-x64 --self-contained -p:AkNounk=true -o "$CN6"
b build src/Rpc/akrpc.csproj -c Release -p:AkNounk=true
b build src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkNounk=true
for lvl in $LEVELS; do
  if [ $lvl = 8 ]; then H="$HN8"; HX=(dotnet "$HN8/harness.dll"); C="$CN8"; CX=(dotnet "$CN8/corpus.dll");
  else H="$HN6"; HX=("$HN6/harness"); C="$CN6"; CX=("$CN6/corpus"); fi
  step "8.$lvl no-unknown, net${lvl}.0"
  core "$H" target-core-nounk
  run "net$lvl nounk layout (by name both ways, 240 section-10 facts)" "${HX[@]}" layout "$SLICE/target-core-nounk/layout.json"
  run "net$lvl nounk conformance (byte identity)" "${HX[@]}" conformance
  AK_GATE_PLANT_HOST_FAIL=apply run "net$lvl nounk hostfail, apply throws" "${HX[@]}" hostfail
  AK_GATE_PLANT_HOST_FAIL=add run "net$lvl nounk hostfail, add/new throws" "${HX[@]}" hostfail
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

# ============================================================== WP7, CAMPAIGN req 19 (R-H31)
# The counts of EVERY exported entry point the timed code calls, resets included, from the
# counting builds (/p:AkHostCount=true: each generated import counts itself by name): per
# core-ffi case of the codec suite (the same closures BenchmarkDotNet times) and per call of
# the RPC cells B to E (A and F listed, making none), retain with no pre-placed buffer and the
# timed build's geometric grow (CAMPAIGN req 19 as amended). Committed: gen/counts.txt, gen/counts-nounk.txt, gen/rpc-counts.txt,
# gen/rpc-counts-nounk.txt; the whole file must be equal, row for row.
step "9. the counting builds (CAMPAIGN req 19 as amended): every entry point, per case and per RPC call"
BC="$SLICE/src/BenchDotNet/bin-count/Release/net8.0"; BCN="$SLICE/src/BenchDotNet/bin-count-nounk/Release/net8.0"
RC="$SLICE/src/Rpc/bin-count/Release/net8.0"; RCN="$SLICE/src/Rpc/bin-count-nounk/Release/net8.0"
RN8="$SLICE/src/Rpc/bin-nounk/Release/net8.0"
b build src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkHostCount=true
b build src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkHostCount=true -p:AkNounk=true
b build src/Rpc/akrpc.csproj -c Release -p:AkHostCount=true
b build src/Rpc/akrpc.csproj -c Release -p:AkHostCount=true -p:AkNounk=true
core "$BC" target-core-count; core "$BCN" target-core-count-nounk; core "$RC" target-core-count; core "$RCN" target-core-count-nounk
cmp_counts() {  # name committed produced
  if diff "$2" "$3" > "$SCRATCH/counts.diff"; then echo "$1: equal to $(basename "$2"), $(grep -vc '^#' "$2") rows"; return 0; fi
  echo "$1: DIFFERS from $(basename "$2"):"; head -20 "$SCRATCH/counts.diff"; return 1
}
run "codec counts, full build (= gen/counts.txt)" bash -c "dotnet '$BC/BenchDotNet.dll' --counts '$SCRATCH/counts.txt' && $(declare -f cmp_counts); SCRATCH='$SCRATCH' cmp_counts full '$SLICE/gen/counts.txt' '$SCRATCH/counts.txt'"
run "codec counts, no-unknown build (= gen/counts-nounk.txt)" bash -c "dotnet '$BCN/BenchDotNet.dll' --counts '$SCRATCH/counts-nounk.txt' && $(declare -f cmp_counts); SCRATCH='$SCRATCH' cmp_counts nounk '$SLICE/gen/counts-nounk.txt' '$SCRATCH/counts-nounk.txt'"
# D21: the string encode paths' counts (AK_STR_ENC): E1 and ETH:256 equal the base rows; E2's
# rows add `tc N` to rev (the core's calls into the C# transcoder). Committed: gen/counts-str-*.txt.
for m in E1 E2 ETH:256; do t=$(echo "$m" | tr -d ':' | tr 'A-Z' 'a-z')
  AK_STR_ENC=$m run "codec counts, string path $m, full build (= gen/counts-str-$t.txt)" bash -c "dotnet '$BC/BenchDotNet.dll' --counts '$SCRATCH/counts-str-$t.txt' && $(declare -f cmp_counts); SCRATCH='$SCRATCH' cmp_counts str-$t '$SLICE/gen/counts-str-$t.txt' '$SCRATCH/counts-str-$t.txt'"
  AK_STR_ENC=$m run "codec counts, string path $m, no-unknown build (= gen/counts-str-$t-nounk.txt)" bash -c "dotnet '$BCN/BenchDotNet.dll' --counts '$SCRATCH/counts-str-$t-nounk.txt' && $(declare -f cmp_counts); SCRATCH='$SCRATCH' cmp_counts str-$t-nounk '$SLICE/gen/counts-str-$t-nounk.txt' '$SCRATCH/counts-str-$t-nounk.txt'"
done
# The grow count is live: with an exact-size grow (a control, never the timed policy) the
# retain rows that carry unknown fields count differently, and the comparison must fail.
AK_COUNT_GROW=exact control "codec counts with an exact-size grow (the counted grows must matter)" bash -c "dotnet '$BC/BenchDotNet.dll' --counts '$SCRATCH/counts-exact.txt' && diff -q '$SLICE/gen/counts.txt' '$SCRATCH/counts-exact.txt'"
echo "# rpc counts and the upload check: against the campaign server ($SOCK, started in step 6)"
run "rpc counts, full build (= gen/rpc-counts.txt)" bash -c "dotnet '$RC/akrpc.dll' campaign --suite rpc --sock '$SOCK' --transport shipped --counts '$SCRATCH/rpc-counts.txt' && $(declare -f cmp_counts); SCRATCH='$SCRATCH' cmp_counts rpc-full '$SLICE/gen/rpc-counts.txt' '$SCRATCH/rpc-counts.txt'"
run "rpc counts, no-unknown build (= gen/rpc-counts-nounk.txt)" bash -c "dotnet '$RCN/akrpc.dll' campaign --suite rpc --sock '$SOCK' --transport shipped --counts '$SCRATCH/rpc-counts-nounk.txt' && $(declare -f cmp_counts); SCRATCH='$SCRATCH' cmp_counts rpc-nounk '$SLICE/gen/rpc-counts-nounk.txt' '$SCRATCH/rpc-counts-nounk.txt'"
AK_RPC_COUNT_DELIVERIES=1 run "rpc delivery-cell counts, full build (= gen/rpc-delivery-counts.txt)" bash -c "dotnet '$RC/akrpc.dll' campaign --suite rpc --sock '$SOCK' --transport shipped --counts '$SCRATCH/rpc-delivery-counts.txt' && $(declare -f cmp_counts); SCRATCH='$SCRATCH' cmp_counts rpc-deliveries '$SLICE/gen/rpc-delivery-counts.txt' '$SCRATCH/rpc-delivery-counts.txt'"
AK_RPC_COUNT_DELIVERIES=1 run "rpc delivery-cell counts, no-unknown build (= gen/rpc-delivery-counts-nounk.txt)" bash -c "dotnet '$RCN/akrpc.dll' campaign --suite rpc --sock '$SOCK' --transport shipped --counts '$SCRATCH/rpc-delivery-counts-nounk.txt' && $(declare -f cmp_counts); SCRATCH='$SCRATCH' cmp_counts rpc-deliveries-nounk '$SLICE/gen/rpc-delivery-counts-nounk.txt' '$SCRATCH/rpc-delivery-counts-nounk.txt'"
# WP8 (CAMPAIGN req 14 as amended): the upload directions' correctness before timing, both
# builds: every c cell accepted, every d stream's count and SHA-256 equal (every cell and
# framed twin); a planted wrong SHA-256 must fail.
core "$R8" target-core; core "$RN8" target-core-nounk
run "upload check, full build (c and d, every cell and framed twin)" dotnet "$R8/akrpc.dll" campaign --suite rpc --sock "$SOCK" --transport shipped --upload-check
run "upload check, no-unknown build" dotnet "$RN8/akrpc.dll" campaign --suite rpc --sock "$SOCK" --transport shipped --upload-check
AK_CAMPAIGN_PLANT=digest control "upload check with a planted wrong SHA-256" dotnet "$R8/akrpc.dll" campaign --suite rpc --sock "$SOCK" --transport shipped --upload-check
AK_CAMPAIGN_PLANT=len AK_CAMPAIGN_PLANT_DIR=d control "upload check with a planted wrong byte count" dotnet "$RN8/akrpc.dll" campaign --suite rpc --sock "$SOCK" --transport shipped --upload-check
# D10 (WP13): the TCP_NODELAY readback is live: with Nagle left ON in both client transports
# (Grpc.Net's socket and the core's ak_client_opts.tcp_nagle) it must fail.
AK_CAMPAIGN_PLANT=nagle control "upload check with Nagle left on (the TCP_NODELAY readback must fail)" dotnet "$R8/akrpc.dll" campaign --suite rpc --sock "$SOCK" --transport shipped --upload-check
"$SERVE" stop
echo "# counts, no-unknown against full: the files differ in mode names and in every push/pull decode row (no ak_dec_reset_* in the no-unknown build); the committed files carry every row"

echo
if [ "$LEVELS" != "8 6" ]; then
  if [ $FAILS -eq 0 ]; then echo "CHECKS PASSED (levels $LEVELS only: NOT the gate)"; else echo "CHECKS FAILED: $FAILS (levels $LEVELS only)"; fi
elif [ $FAILS -eq 0 ]; then echo "GATE PASSED"; else echo "GATE FAILED: $FAILS"; fi
exit $FAILS
