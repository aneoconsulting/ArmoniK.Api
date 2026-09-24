#!/usr/bin/env bash
# Build the ONE core (README R0) for this slice, from a snapshot of the COMMITTED
# ffi/poc/codec (so a concurrent edit in the working tree cannot leak into a gate), in the
# builds the gate needs. Every build has `init-guard` (R-G7: a binding that skips ak_init
# must fail, not pass silently).
#
#   target-core         --features rpc,init-guard          the shapes core (harness, akrpc)
#   target-core-count   --features rpc,count,init-guard    R5's counting build
#   target-core-corpus  --features corpus,init-guard       the core generated for the corpus
#                                                          reader schema (src/Corpus)
#   abi/ probe          default and --features corpus      the Rust declaration's layout,
#                                                          JSON into target-core*/layout.json
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SLICE="$(cd "$HERE/.." && pwd)"
REPO="$(git -C "$SLICE" rev-parse --show-toplevel)"
SCRATCH="${SCRATCH:-$(mktemp -d)}"
SNAP="$SCRATCH/snap"
rm -rf "$SNAP"; mkdir -p "$SNAP"
( cd "$REPO" && git archive HEAD ffi/poc/codec | tar -x -C "$SNAP" )
if git -C "$REPO" diff --quiet HEAD -- ffi/poc/codec/crates ffi/poc/codec/Cargo.toml ffi/poc/codec/Cargo.lock; then
  echo "# core: git archive HEAD ffi/poc/codec (crates identical to the working tree); last core commit $(git -C "$REPO" log -1 --format=%h -- ffi/poc/codec/crates)"
else
  echo "# core: git archive HEAD ffi/poc/codec; WARNING: the working tree's crates differ from HEAD (not built)"
fi
mkdir -p "$SNAP/ffi/poc/csharp"
cp -r "$SLICE/abi" "$SNAP/ffi/poc/csharp/abi"
rm -rf "$SNAP/ffi/poc/csharp/abi/target"
echo "# rustc $(rustc --version | awk '{print $2}'), cargo $(cargo --version | awk '{print $2}')"
build() {  # name features
  local dir="$SLICE/$1"
  ( cd "$SNAP/ffi/poc/codec" && CARGO_TARGET_DIR="$dir" cargo build --release -q -p ak-core --features "$2" 2>"$SCRATCH/cargo.err" ) \
    || { cat "$SCRATCH/cargo.err"; exit 1; }
  echo "#   $1: --features $2 -> $(sha256sum "$dir/release/libak_core.so" | cut -c1-16) ($(nm -D --defined-only "$dir/release/libak_core.so" | grep -c ' T ak_') ak_* exports)"
}
build target-core "rpc,init-guard"
build target-core-count "rpc,count,init-guard"
build target-core-corpus "corpus,init-guard"
( cd "$SNAP/ffi/poc/csharp/abi" && CARGO_TARGET_DIR="$SLICE/target-probe" cargo run --release -q 2>"$SCRATCH/cargo.err" > "$SLICE/target-core/layout.json" ) || { cat "$SCRATCH/cargo.err"; exit 1; }
( cd "$SNAP/ffi/poc/csharp/abi" && CARGO_TARGET_DIR="$SLICE/target-probe-corpus" cargo run --release -q --features corpus 2>"$SCRATCH/cargo.err" > "$SLICE/target-core-corpus/layout.json" ) || { cat "$SCRATCH/cargo.err"; exit 1; }
echo "#   layout probe: $(grep -c '"size"' "$SLICE/target-core/layout.json") structs (shapes), $(grep -c '"size"' "$SLICE/target-core-corpus/layout.json") structs (corpus)"
