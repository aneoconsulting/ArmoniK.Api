#!/usr/bin/env bash
# Build the python slice's COMPOSED arm: the shared core plus the generated CPython shim.
#
#   ./build.sh <python-exe> [<python-exe> ...]
#
# Work unit 1's microbenchmark arms are built by `mech/build.sh` and are separate; this
# builds the thing that is actually the design.
#
# Four steps, and three of them are checks rather than compiles:
#   1. R0   -- the core is the ONE core at poc/codec, reached by path
#   2. R1   -- the generated tree is current with shapes.json
#   3.         the core, as a shared library
#   4. R5   -- the shim's imports of ak_* are unresolved, so the boundary is real
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
POC="$(dirname "$HERE")"
CORE="$POC/codec"
cd "$HERE"

echo "== 1. R0: one core, at poc/codec, reached by path =="
"$CORE/gen/one_core.sh" | tail -3

echo
echo "== 2. R1: the generated tree is current with shapes.json =="
python3 gen/generate.py --check

echo
echo "== 3. the shared core =="
(cd "$CORE" && cargo build --release -q -p ak-core)
CORELIB="$CORE/target/release"
ls -la "$CORELIB/libak_core.so"
echo "   exported ak_* entry points: $(nm -D --defined-only "$CORELIB/libak_core.so" | grep -cE ' T ak_')"

# The counting core, into its own target directory so a timing never comes from a binary
# carrying counters (README R5).
(cd "$CORE" && CARGO_TARGET_DIR="$CORE/target-count" cargo build --release -q -p ak-core --features count)
COUNTLIB="$CORE/target-count/release"

OUT=build
mkdir -p "$OUT"
CFLAGS_COMMON="-O2 -fPIC -Wall -Wextra -Werror -Wno-unused-parameter -fvisibility=hidden"

for PY in "$@"; do
  TAG=$("$PY" -c 'import sys;print("%d.%d"%sys.version_info[:2])')
  SOABI=$("$PY" -c 'import sysconfig;print(sysconfig.get_config_var("EXT_SUFFIX"))')
  INC=$("$PY" -c 'import sysconfig;print(sysconfig.get_paths()["include"])')
  D="$OUT/py$TAG"
  mkdir -p "$D"
  echo
  echo "== python $TAG =="

  cc $CFLAGS_COMMON -shared -I"$INC" -Igen/out \
     -DAK_MODNAME_STR='"_akffi"' -DAK_INITFUNC=PyInit__akffi \
     -o "$D/_akffi$SOABI" native/binding.c \
     -L"$CORELIB" -lak_core -Wl,-rpath,"$CORELIB"

  cc $CFLAGS_COMMON -shared -I"$INC" -Igen/out -DAK_COUNT \
     -DAK_MODNAME_STR='"_akffi_count"' -DAK_INITFUNC=PyInit__akffi_count \
     -o "$D/_akffi_count$SOABI" native/binding.c \
     -L"$COUNTLIB" -lak_core -Wl,-rpath,"$COUNTLIB"

  echo "   built $D/_akffi$SOABI and the counting build beside it"

  # README R5: the boundary is proved from the built artifact, not claimed in a log.
  # A shim whose ak_* calls were folded away would be the no-boundary control wearing
  # the composed arm's name, which is the defect that cost the rust slice a stage.
  for so in "$D/_akffi$SOABI" "$D/_akffi_count$SOABI"; do
    u=$(nm -D --undefined-only "$so" | grep -cE ' ak_' || true)
    [ "$u" -ge 8 ] || { echo "   FAIL: $so imports only $u ak_* symbols"; exit 1; }
    n=$(readelf -d "$so" | grep -c 'libak_core' || true)
    [ "$n" -ge 1 ] || { echo "   FAIL: $so does not depend on libak_core.so"; exit 1; }
    echo "   $(basename "$so"): $u undefined ak_* imports, libak_core.so NEEDED"
  done
done

echo
echo "the composed arm is built. `conformance.py` gates it before anything is timed."
