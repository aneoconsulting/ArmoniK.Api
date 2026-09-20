#!/usr/bin/env bash
# The corpus gate. CI runs this; it is not a thing to run by hand and read.
#
#   ./run.sh          regenerate, validate, self-test, and check for drift
#   ./run.sh --check  the CI form: no regeneration of the committed tree
#
# It needs python3, `protoc` and a protobuf runtime. Install them with
#   pip install protobuf grpcio-tools
# which gives protoc (grpc_tools) and a upb-backed runtime -- a genuinely
# independent second implementation, which is the point: a corpus validated only
# by the writer that produced it is a hash of itself.
set -euo pipefail
cd "$(dirname "$0")"

CHECK_ONLY=0
[ "${1:-}" = "--check" ] && CHECK_ONLY=1

echo "== python and protobuf"
python3 - <<'PY'
import sys
sys.exit(0) if sys.version_info >= (3, 8) else sys.exit("python 3.8 or later is required")
PY
python3 -c "
import google.protobuf, sys
from google.protobuf.internal import api_implementation
print('protobuf', google.protobuf.__version__, api_implementation.Type(), 'backend')
" || { echo 'missing: pip install protobuf grpcio-tools'; exit 2; }
python3 -m grpc_tools.protoc --version || { echo 'missing: pip install grpcio-tools'; exit 2; }

if [ "$CHECK_ONLY" = "0" ]; then
  echo
  echo "== generate and validate"
  python3 emit/build.py
fi

echo
echo "== the committed tree is what the generator writes today"
python3 emit/build.py --check

echo
echo "== the guards, each one watched working"
python3 emit/selftest.py

echo
echo "== a clean clone at a different path rebuilds itself"
# The python slice's D4: a generated module carried the ffi root as an absolute
# path baked in at generate time, so a clone of the branch at any other path
# regenerated a different file and --check refused the whole tree. It was found
# by cloning the pushed branch and building it, not by reading the code, which is
# why this is a step of the gate and not a claim in a log.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
REPO="$(git -C . rev-parse --show-toplevel)"
git -C "$REPO" worktree list >/dev/null 2>&1 || true
git clone --quiet --no-hardlinks --shared "$REPO" "$TMP/clone" 2>/dev/null \
  || git clone --quiet "$REPO" "$TMP/clone"
git -C "$TMP/clone" checkout --quiet "$(git -C "$REPO" rev-parse HEAD)" 2>/dev/null || true
if [ -d "$TMP/clone/ffi/corpus" ]; then
  ( cd "$TMP/clone/ffi/corpus" && python3 emit/build.py --check )
else
  echo "the clone has no ffi/corpus yet (nothing committed); skipped"
fi

echo
echo "== the corpus gate passed"
