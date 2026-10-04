#!/usr/bin/env bash
# D19 build impact, the two slices whose core builds are not plain cargo commands:
#
#   cpp     poc/cpp's CMake project (no AK_RPC: Google Benchmark 1.8.3 is not installed here),
#           configured against a git-archive snapshot of HEAD's poc/codec (AK_CORE_ROOT) with
#           its cargo targets in the scratch dir (AK_CORE_TGT): the core targets every arm
#           depends on (core, core_count, core_pad, core_global, core_both) and the
#           conformance arms a17 shared, a17 STATIC (libak_core.a, g++), c11 shared (C++11),
#           plus counts_a17_static_lto (-flto host); each conformance arm is RUN (exit 0
#           required). Build tree in the scratch dir; nothing written under poc/cpp.
#   csharp  poc/csharp/gen/build_core.sh as committed (git archive HEAD ffi/poc/codec; its
#           target-core* dirs under poc/csharp, removed afterwards by this script; the last
#           part builds ArmoniK.Api.Client with dotnet).
#
#   gen/d19_cpp_cs.sh <scratch dir> [cpp|csharp|both]
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
POC="$(cd "$HERE/../.." && pwd)"
REPO="$(git -C "$POC" rev-parse --show-toplevel)"
S="${1:?usage: gen/d19_cpp_cs.sh <scratch dir> [cpp|csharp|both]}"; mkdir -p "$S"; S="$(cd "$S" && pwd)"
WHAT="${2:-both}"
fails=0
echo "# commit $(git -C "$REPO" rev-parse --short HEAD); $(cmake --version | head -1); $(g++ --version | head -1); dotnet $(dotnet --version 2>/dev/null || echo none)"

if [ "$WHAT" != csharp ]; then
  echo "===== cpp: CMake core targets and conformance arms ====="
  SNAP="$S/cpp-snap"; rm -rf "$SNAP"; mkdir -p "$SNAP"
  git -C "$REPO" archive HEAD ffi/poc/codec | tar -x -C "$SNAP"
  B="$S/cpp-build"
  if cmake -S "$POC/cpp" -B "$B" -DCMAKE_BUILD_TYPE=Release -DAK_CORE_ROOT="$SNAP/ffi/poc/codec" -DAK_CORE_TGT="$S/cpp-core" > "$S/cpp-configure.log" 2>&1; then
    echo "  ok    configure"
  else echo "  FAIL  configure"; tail -20 "$S/cpp-configure.log"; fails=$((fails+1)); fi
  for t in core core_count core_pad core_global core_both conformance_a17_shared conformance_a17_static conformance_c11_shared counts_a17_static_lto; do
    if cmake --build "$B" --target "$t" -j"$(nproc)" > "$S/cpp-build-$t.log" 2>&1; then echo "  ok    build $t"
    else echo "  FAIL  build $t"; grep -E "error|undefined" "$S/cpp-build-$t.log" | head -10; fails=$((fails+1)); fi
  done
  so="$S/cpp-core/target/release/libak_core.so"
  echo "  core: $(readelf -d "$so" | grep -o 'libstdc++[^]]*') NEEDED; libak_core.a $(stat -c %s "$S/cpp-core/target/release/libak_core.a") B"
  for arm in conformance_a17_shared conformance_a17_static conformance_c11_shared; do
    exe=$(find "$B" -maxdepth 2 -type f -name "$arm" | head -1)
    [ -n "$exe" ] || { echo "  FAIL  $arm: no executable"; fails=$((fails+1)); continue; }
    link=$(ldd "$exe" | grep -c libak_core || true)
    if ( cd "$POC/cpp" && "$exe" ) > "$S/cpp-run-$arm.log" 2>&1; then
      echo "  ok    run $arm (dynamic libak_core: $link): $(tail -1 "$S/cpp-run-$arm.log")"
    else echo "  FAIL  run $arm: $(tail -3 "$S/cpp-run-$arm.log" | tr '\n' ' ')"; fails=$((fails+1)); fi
  done
fi

if [ "$WHAT" != cpp ]; then
  echo "===== csharp: gen/build_core.sh ====="
  if SCRATCH="$S/cs" bash "$POC/csharp/gen/build_core.sh" > "$S/cs-build_core.log" 2>&1; then
    echo "  ok    build_core.sh"; sed 's/^/        /' "$S/cs-build_core.log"
  else echo "  FAIL  build_core.sh"; tail -30 "$S/cs-build_core.log"; fails=$((fails+1)); fi
  for d in "$POC"/csharp/target-core*; do
    [ -f "$d/release/libak_core.so" ] && echo "  $(basename "$d"): $(nm -D --defined-only "$d/release/libak_core.so" | grep -cE ' T ak_(utf16_to_utf8|utf16_utf8_len|utf8_to_utf16|utf8_utf16_len|utf8_validate|utf16_validate|tc_utf16_scalar)$') D19 exports, $(readelf -d "$d/release/libak_core.so" | grep -c 'libstdc++') libstdc++ NEEDED"
  done
  rm -rf "$POC"/csharp/target-core* "$POC"/csharp/target-probe* "$POC"/csharp/target-armonik-client
fi
echo "failures: $fails"
[ "$fails" = 0 ] && echo "D19 CPP/CSHARP BUILD CHECK PASSED" || echo "D19 CPP/CSHARP BUILD CHECK FAILED"
