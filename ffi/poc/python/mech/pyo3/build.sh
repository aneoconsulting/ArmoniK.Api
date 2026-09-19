#!/usr/bin/env bash
# Build the PyO3 arm, once per interpreter (full C-API) plus one abi3 artifact.
#
#   pyo3/build.sh <python-exe> [...]
#
# mech/build.sh must have run first: this links libakmech_cabi.so, the shared
# callee that makes the mechanism rows comparable.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MECH="$(dirname "$HERE")"
CABI="$MECH/build"
[ -f "$CABI/libakmech_cabi.so" ] || { echo "run mech/build.sh first"; exit 1; }
export AKMECH_CABI_DIR="$CABI"
cd "$HERE"

for PY in "$@"; do
  TAG=$("$PY" -c 'import sys;print("%d.%d"%sys.version_info[:2])')
  D="$CABI/py$TAG"
  mkdir -p "$D"
  echo "== pyo3 for python $TAG =="
  PYO3_PYTHON="$PY" cargo build --release --target-dir "target/$TAG" -q
  cp "target/$TAG/release/lib_akmech_pyo3.so" "$D/_akmech_pyo3.so"
  u=$(nm -D --undefined-only "$D/_akmech_pyo3.so" | grep -c ' ak_' || true)
  [ "$u" -ge 3 ] || { echo "   FAIL: pyo3 arm does not import ak_* from the C library"; exit 1; }
  echo "   built $D/_akmech_pyo3.so ($u undefined ak_* imports)"
done

echo "== pyo3, abi3 (one artifact across 3.x) =="
PY0="$1"
PYO3_PYTHON="$PY0" cargo build --release --features abi3 --target-dir target/abi3 -q
mkdir -p "$CABI/abi3"
cp target/abi3/release/lib_akmech_pyo3.so "$CABI/abi3/_akmech_pyo3.abi3.so"
echo "   built $CABI/abi3/_akmech_pyo3.abi3.so"
