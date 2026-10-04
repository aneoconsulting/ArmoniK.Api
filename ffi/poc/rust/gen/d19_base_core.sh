#!/usr/bin/env bash
# D19: build the PRE-D19 core (default: 1d18e637, "docs(ffi): D19-D21 to be built", the base
# the D19 unit started from) from `git archive` of its ffi/poc/codec, release, default
# features, into <dir>. The differential and the instrumentation load its libak_core.so
# beside the D19 core (AK_D19_BASE_CORE). Prints the library path last.
#
#   gen/d19_base_core.sh <dir> [commit]
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIR="${1:?usage: gen/d19_base_core.sh <dir> [commit]}"
REV="${2:-1d18e637}"
REPO="$(git -C "$HERE" rev-parse --show-toplevel)"
mkdir -p "$DIR/src"
git -C "$REPO" archive "$REV" ffi/poc/codec | tar -x -C "$DIR/src"
( cd "$DIR/src/ffi/poc/codec" && CARGO_TARGET_DIR="$DIR/target" cargo build --release -q -p ak-core 2>/dev/null )
SO="$DIR/target/release/libak_core.so"
if nm -D --defined-only "$SO" | grep -q ' T ak_tc_utf16_scalar$'; then echo "$REV is not a pre-D19 core"; exit 1; fi
echo "# pre-D19 core: $REV, $(nm -D --defined-only "$SO" | grep -c ' T ak_') ak_* exports, sha256 $(sha256sum "$SO" | cut -c1-16)"
echo "$SO"
