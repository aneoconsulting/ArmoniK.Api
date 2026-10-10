#!/usr/bin/env bash
# s16 (owner, 2026-10-10): the reset-on-entry variant cores (ak-core feature `reset-on-entry`,
# default OFF; a measurement experiment), from a snapshot of the COMMITTED ffi/poc/codec (as
# gen/build_core.sh), each in its own target dir: the shapes core, the counting core and the
# corpus core, full and no-unknown (--no-default-features); plus the default shapes and corpus
# cores rebuilt from the same snapshot, whose sha256 must equal target-core's and
# target-core-corpus's (the feature leaves the default build byte-identical).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SLICE="$(cd "$HERE/.." && pwd)"
REPO="$(git -C "$SLICE" rev-parse --show-toplevel)"
SCRATCH="${SCRATCH:-$(mktemp -d)}"; mkdir -p "$SCRATCH"
SNAP="$SCRATCH/snap16"; rm -rf "$SNAP"; mkdir -p "$SNAP"
( cd "$REPO" && git archive HEAD ffi/poc/codec | tar -x -C "$SNAP" )
echo "# s16 cores: git archive HEAD ffi/poc/codec, last core commit $(git -C "$REPO" log -1 --format=%h -- ffi/poc/codec/crates); rustc $(rustc --version | awk '{print $2}')"
build() {  # name features [extra cargo flags]
  local dir="$SLICE/$1"
  ( cd "$SNAP/ffi/poc/codec" && CARGO_TARGET_DIR="$dir" cargo build --release -q -p ak-core ${3:-} --features "$2" 2>"$SCRATCH/cargo.err" ) || { cat "$SCRATCH/cargo.err"; exit 1; }
  echo "#   $1: --features $2 ${3:-} -> $(sha256sum "$dir/release/libak_core.so" | cut -c1-16) ($(nm -D --defined-only "$dir/release/libak_core.so" | grep -c ' T ak_') ak_* exports; marker: $(nm -D --defined-only "$dir/release/libak_core.so" | grep -c ' T ak_measure_reset_on_entry'))"
}
build target-core-roe "rpc,init-guard,reset-on-entry"
build target-core-count-roe "rpc,count,init-guard,reset-on-entry"
build target-core-corpus-roe "corpus,init-guard,reset-on-entry"
build target-core-nounk-roe "rpc,init-guard,reset-on-entry" --no-default-features
build target-core-count-nounk-roe "rpc,count,init-guard,reset-on-entry" --no-default-features
build target-core-corpus-nounk-roe "corpus,init-guard,reset-on-entry" --no-default-features
build target-core-s16default "rpc,init-guard"
build target-core-corpus-s16default "corpus,init-guard"
for p in "target-core-s16default target-core" "target-core-corpus-s16default target-core-corpus"; do
  set -- $p
  a=$(sha256sum "$SLICE/$1/release/libak_core.so" | cut -c1-64); b=$(sha256sum "$SLICE/$2/release/libak_core.so" | cut -c1-64)
  [ "$a" = "$b" ] && echo "# default build $1 == $2 (byte-identical: ${a:0:16})" || echo "# DIFFERS: $1 ${a:0:16} != $2 ${b:0:16}"
done
