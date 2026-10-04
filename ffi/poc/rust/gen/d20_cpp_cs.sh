#!/usr/bin/env bash
# D20 (owner, 2026-10-04): the C++ and C# slices on the D20 core and their regenerated
# bindings. Nothing timed; nothing written under poc/cpp or logs/<other slice>.
#
#   cpp     poc/cpp's CMake project (no AK_RPC: Google Benchmark 1.8.3 is not installed here),
#           configured against a git-archive snapshot of HEAD's poc/codec (AK_CORE_ROOT) with
#           its cargo targets in the scratch dir (AK_CORE_TGT), as gen/d19_cpp_cs.sh. Built:
#           the core targets, the conformance arms a17 shared, a17 STATIC, c11 shared and
#           nounk a17, and the counting arms counts_a17_shared / counts_a17_static. Each
#           conformance arm is RUN (exit 0 required); the counting arms are run as
#           gen/wp5_gate.sh runs them (payloads + the U-* rows of gen/u_rows.py) and their
#           count rows compared with logs/cpp/counts-baseline.log (shared and static).
#   csharp  poc/csharp/gen/build_core.sh as committed (git archive HEAD ffi/poc/codec), then
#           the slice's quick gate `AK_GATE_LEVELS=8 AK_GATE_KEEP_CORE=1 gen/gate.sh` (net8.0
#           checks on the cores just built); the target-core* dirs under poc/csharp are
#           removed afterwards.
#
#   gen/d20_cpp_cs.sh <scratch dir> [cpp|csharp|both]
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
POC="$(cd "$HERE/../.." && pwd)"
REPO="$(git -C "$POC" rev-parse --show-toplevel)"
S="${1:?usage: gen/d20_cpp_cs.sh <scratch dir> [cpp|csharp|both]}"; mkdir -p "$S"; S="$(cd "$S" && pwd)"
WHAT="${2:-both}"
fails=0
echo "# commit $(git -C "$REPO" rev-parse --short HEAD)$(git -C "$REPO" diff --quiet HEAD -- ffi/poc/codec ffi/poc/cpp ffi/poc/csharp || echo ' + uncommitted changes'); $(cmake --version | head -1); $(g++ --version | head -1); dotnet $(dotnet --version 2>/dev/null || echo none)"

if [ "$WHAT" != csharp ]; then
  echo "===== cpp: CMake core targets, conformance arms, crossing counts ====="
  SNAP="$S/cpp-snap"; rm -rf "$SNAP"; mkdir -p "$SNAP"
  git -C "$REPO" archive HEAD ffi/poc/codec | tar -x -C "$SNAP"
  B="$S/cpp-build"
  if cmake -S "$POC/cpp" -B "$B" -DCMAKE_BUILD_TYPE=Release -DAK_CORE_ROOT="$SNAP/ffi/poc/codec" -DAK_CORE_TGT="$S/cpp-core" > "$S/cpp-configure.log" 2>&1; then
    echo "  ok    configure"
  else echo "  FAIL  configure"; tail -20 "$S/cpp-configure.log"; fails=$((fails+1)); fi
  ARMS="conformance_a17_shared conformance_a17_static conformance_c11_shared conformance_nounk_a17"
  for t in core core_count core_nounk $ARMS counts_a17_shared counts_a17_static; do
    if cmake --build "$B" --target "$t" -j"$(nproc)" > "$S/cpp-build-$t.log" 2>&1; then echo "  ok    build $t"
    else echo "  FAIL  build $t"; grep -E "error|undefined" "$S/cpp-build-$t.log" | head -10; fails=$((fails+1)); fi
  done
  so="$S/cpp-core/target/release/libak_core.so"
  echo "  core: $(nm -D --defined-only "$so" | grep -c ' T ak_dec_set_pvt_') D20 pvt setters exported"
  for arm in $ARMS; do
    exe=$(find "$B" -maxdepth 2 -type f -name "$arm" | head -1)
    [ -n "$exe" ] || { echo "  FAIL  $arm: no executable"; fails=$((fails+1)); continue; }
    link=$(ldd "$exe" | grep -c libak_core || true)
    if ( cd "$POC/cpp" && "$exe" ) > "$S/cpp-run-$arm.log" 2>&1; then
      echo "  ok    run $arm (dynamic libak_core: $link): $(tail -1 "$S/cpp-run-$arm.log")"
    else echo "  FAIL  run $arm: $(tail -3 "$S/cpp-run-$arm.log" | tr '\n' ' ')"; fails=$((fails+1)); fi
  done
  PAY="$REPO/ffi/schema/generated"; COR="$REPO/ffi/corpus/generated"
  ( cd "$POC/cpp" && python3 gen/u_rows.py "$COR" "$S/rows.tsv" 2>/dev/null )
  grep -E '^  [PU]' "$REPO/ffi/logs/cpp/counts-baseline.log" > "$S/cwant"
  for v in shared static; do
    exe="$B/counts_a17_$v"
    if ( cd "$PAY" && "$exe" --corpus "$COR" --rows "$S/rows.tsv" ) > "$S/cpp-counts-$v.log" 2>&1; then
      grep -E '^  [PU]' "$S/cpp-counts-$v.log" > "$S/cgot-$v"
      if diff "$S/cwant" "$S/cgot-$v" > "$S/cdiff-$v"; then
        echo "  ok    counts_a17_$v: $(wc -l < "$S/cgot-$v") count rows identical to logs/cpp/counts-baseline.log"
      else echo "  FAIL  counts_a17_$v differ from the baseline:"; head -8 "$S/cdiff-$v"; fails=$((fails+1)); fi
    else echo "  FAIL  run counts_a17_$v: $(tail -3 "$S/cpp-counts-$v.log" | tr '\n' ' ')"; fails=$((fails+1)); fi
  done
fi

if [ "$WHAT" != cpp ]; then
  echo "===== csharp: gen/build_core.sh, then the net8.0 quick gate ====="
  if SCRATCH="$S/cs" bash "$POC/csharp/gen/build_core.sh" > "$S/cs-build_core.log" 2>&1; then
    echo "  ok    build_core.sh"; sed 's/^/        /' "$S/cs-build_core.log"
  else echo "  FAIL  build_core.sh"; tail -30 "$S/cs-build_core.log"; fails=$((fails+1)); fi
  if ( cd "$POC/csharp" && AK_GATE_LEVELS=8 AK_GATE_KEEP_CORE=1 bash gen/gate.sh ) > "$S/cs-gate.log" 2>&1; then
    echo "  ok    AK_GATE_LEVELS=8 AK_GATE_KEEP_CORE=1 gen/gate.sh"
  else echo "  FAIL  AK_GATE_LEVELS=8 AK_GATE_KEEP_CORE=1 gen/gate.sh"; fails=$((fails+1)); fi
  sed 's/^/        /' "$S/cs-gate.log"
  rm -rf "$POC"/csharp/target-core* "$POC"/csharp/target-probe* "$POC"/csharp/target-armonik-client
fi
echo "failures: $fails"
[ "$fails" = 0 ] && echo "D20 CPP/CSHARP CHECK PASSED" || echo "D20 CPP/CSHARP CHECK FAILED"
