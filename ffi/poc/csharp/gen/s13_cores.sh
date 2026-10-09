#!/usr/bin/env bash
# s13 (owner, 2026-10-09): the measurement cores with the scalar UTF-16 transcoders, from a
# snapshot of the COMMITTED ffi/poc/codec (as gen/build_core.sh): ak-core features
# tc-scalar-naive / tc-scalar-word (default OFF), shapes core (rpc,init-guard) and corpus core
# (corpus,init-guard), full build; plus the default shapes core rebuilt from the same snapshot,
# whose sha256 must equal target-core's (the default build unchanged by the features).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SLICE="$(cd "$HERE/.." && pwd)"
REPO="$(git -C "$SLICE" rev-parse --show-toplevel)"
SCRATCH="${SCRATCH:-$(mktemp -d)}"; mkdir -p "$SCRATCH"
SNAP="$SCRATCH/snap13"; rm -rf "$SNAP"; mkdir -p "$SNAP"
( cd "$REPO" && git archive HEAD ffi/poc/codec | tar -x -C "$SNAP" )
echo "# s13 cores: git archive HEAD ffi/poc/codec, last core commit $(git -C "$REPO" log -1 --format=%h -- ffi/poc/codec/crates); rustc $(rustc --version | awk '{print $2}')"
build() {  # name features
  local dir="$SLICE/$1"
  ( cd "$SNAP/ffi/poc/codec" && CARGO_TARGET_DIR="$dir" cargo build --release -q -p ak-core --features "$2" 2>"$SCRATCH/cargo.err" ) || { cat "$SCRATCH/cargo.err"; exit 1; }
  echo "#   $1: --features $2 -> $(sha256sum "$dir/release/libak_core.so" | cut -c1-16) ($(nm -D --defined-only "$dir/release/libak_core.so" | grep -c ' T ak_') ak_* exports)"
}
build target-core-tcnaive "rpc,init-guard,tc-scalar-naive"
build target-core-tcword "rpc,init-guard,tc-scalar-word"
build target-core-corpus-tcnaive "corpus,init-guard,tc-scalar-naive"
build target-core-corpus-tcword "corpus,init-guard,tc-scalar-word"
build target-core-s13default "rpc,init-guard"
a=$(sha256sum "$SLICE/target-core-s13default/release/libak_core.so" | cut -c1-64); b=$(sha256sum "$SLICE/target-core/release/libak_core.so" | cut -c1-64)
[ "$a" = "$b" ] && echo "# default build from this snapshot == target-core (byte-identical: ${a:0:16})" || echo "# default build from this snapshot ${a:0:16} != target-core ${b:0:16} (target-core built from another commit?)"
