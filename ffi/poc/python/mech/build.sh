#!/usr/bin/env bash
# Build every mechanism arm, for every interpreter this machine offers.
#
#   mech/build.sh [python-exe ...]
#
# Artifacts land in mech/build/<tag>/ where <tag> is the interpreter's version,
# so two interpreters never share a .so and a stale artifact cannot be measured
# for a fresh one.
#
# Three things are built per interpreter:
#   libakmech_cabi.so   the plain C library: the callee EVERY mechanism reaches,
#                       so a mechanism row differs in the mechanism and nothing
#                       else (README R7). Built once, shared.
#   _akmech            the C extension, FULL C-API
#   _akmech3           the same source with -DPy_LIMITED_API=0x030A0000 (abi3)
#
# PyO3 is built separately by pyo3/build.sh because it needs one cargo profile
# per interpreter and takes minutes rather than seconds.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE"

CFLAGS_COMMON="-O2 -fPIC -Wall -Wextra -Werror -Wno-unused-parameter -fvisibility=hidden"
OUT=build
mkdir -p "$OUT"

# The incumbent's generated module. protobuf's Python output is pure Python and
# the runtime under it is upb, so ONE generated file serves every interpreter.
# grpcio-tools carries protoc; this container has no system protoc.
echo "== shapes_pb2.py for the incumbent arm =="
mkdir -p "$OUT/pb2"
GENPY=""
for PY in "$@"; do
  if "$PY" -c "import grpc_tools" 2>/dev/null; then GENPY="$PY"; break; fi
done
if [ -n "$GENPY" ]; then
  SCHEMA_GEN="$(cd "$HERE/../../../schema/generated" && pwd)"
  (cd "$SCHEMA_GEN" && "$GENPY" -m grpc_tools.protoc -I. --python_out="$HERE/$OUT/pb2" shapes.proto)
  echo "   wrote $OUT/pb2/shapes_pb2.py from $SCHEMA_GEN/shapes.proto"
else
  echo "   SKIPPED: no interpreter here has grpcio-tools. The incumbent arm will"
  echo "   report itself absent rather than being silently missing from a table."
fi

echo "== the plain C library (no Python in it) =="
cc $CFLAGS_COMMON -shared -o "$OUT/libakmech_cabi.so" native/cabi.c
nm -D --defined-only "$OUT/libakmech_cabi.so" | grep ' T ak_' | sed 's/^/   /'

PYS=("$@")
if [ ${#PYS[@]} -eq 0 ]; then
  PYS=(python3)
fi

for PY in "${PYS[@]}"; do
  TAG=$("$PY" -c 'import sys;print("%d.%d"%sys.version_info[:2])')
  SOABI=$("$PY" -c 'import sysconfig;print(sysconfig.get_config_var("EXT_SUFFIX"))')
  INC=$("$PY" -c 'import sysconfig;print(sysconfig.get_paths()["include"])')
  D="$OUT/py$TAG"
  mkdir -p "$D"
  echo "== python $TAG ($PY) =="
  echo "   include: $INC"

  # Full C-API.
  cc $CFLAGS_COMMON -shared -I"$INC" \
     -DAK_MODNAME_STR='"_akmech"' -DAK_INITFUNC=PyInit__akmech \
     -o "$D/_akmech$SOABI" native/_akmech.c \
     -L"$OUT" -lakmech_cabi -Wl,-rpath,"\$ORIGIN/.."
  echo "   built $D/_akmech$SOABI"

  # The limited API, pinned at 3.10 so one artifact covers 3.10 through 3.13.
  cc $CFLAGS_COMMON -shared -I"$INC" -DPy_LIMITED_API=0x030A0000 \
     -DAK_MODNAME_STR='"_akmech3"' -DAK_INITFUNC=PyInit__akmech3 \
     -o "$D/_akmech3.abi3.so" native/_akmech.c \
     -L"$OUT" -lakmech_cabi -Wl,-rpath,"\$ORIGIN/.."
  echo "   built $D/_akmech3.abi3.so"

  # The generated codec module (work unit 1 part 2), built twice: measured, and
  # a counting build (README R5). The counting build is a SEPARATE artifact so
  # that a timing never comes from a binary carrying counters.
  python3 gen/generate.py --check >/dev/null || { echo "   generated tree is stale"; exit 1; }
  cc $CFLAGS_COMMON -shared -I"$INC" \
     -DAK_MODNAME_STR='"_akcodec"' -DAK_INITFUNC=PyInit__akcodec \
     -o "$D/_akcodec$SOABI" native/_akcodec.c
  cc $CFLAGS_COMMON -shared -I"$INC" -DAK_COUNT \
     -DAK_MODNAME_STR='"_akcodec_count"' -DAK_INITFUNC=PyInit__akcodec_count \
     -o "$D/_akcodec_count$SOABI" native/_akcodec.c
  echo "   built $D/_akcodec$SOABI and the counting build beside it"

  # README R5, the half that applies here: prove the boundary into the plain C
  # library is a real dynamic-linker call in BOTH builds rather than something
  # the compiler folded away.
  for so in "$D/_akmech$SOABI" "$D/_akmech3.abi3.so"; do
    u=$(nm -D --undefined-only "$so" | grep -c ' ak_' || true)
    [ "$u" -ge 3 ] || { echo "   FAIL: $so does not import ak_* from the C library"; exit 1; }
    echo "   $(basename "$so"): $u undefined ak_* imports, NEEDED=$(readelf -d "$so" | grep -c akmech_cabi)"
  done
done

# One abi3 artifact, built against the OLDEST interpreter present, is what
# README 9.2's "one wheel across 3.x" means. It is published here and the driver
# loads this exact file from every interpreter, so the claim is run rather than
# asserted.
LOWEST=$(for PY in "${PYS[@]}"; do "$PY" -c 'import sys;print("%d.%d"%sys.version_info[:2])'; done | sort -V | head -1)
mkdir -p "$OUT/abi3"
cp "$OUT/py$LOWEST/_akmech3.abi3.so" "$OUT/abi3/_akmech3.abi3.so"
echo
echo "== the one abi3 artifact: built on python $LOWEST, in $OUT/abi3 =="
for PY in "${PYS[@]}"; do
  T=$("$PY" -c 'import sys;print("%d.%d"%sys.version_info[:2])')
  R=$(PYTHONPATH="$OUT/abi3" "$PY" -c 'import _akmech3;print(_akmech3.fwd_noop(5))' 2>&1)
  echo "   loaded by python $T -> fwd_noop(5) = $R"
done
