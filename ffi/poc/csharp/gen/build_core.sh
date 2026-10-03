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
#   target-core*-nounk  the same three with --no-default-features: the NO-UNKNOWN variant
#                       (WP5 step 10; unknown fields compiled out), each in its own target dir
#   abi/ probe          default, --features corpus, and both with --no-default-features:
#                                                          the Rust declaration's layout,
#                                                          JSON into target-core*/layout.json
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SLICE="$(cd "$HERE/.." && pwd)"
REPO="$(git -C "$SLICE" rev-parse --show-toplevel)"
SCRATCH="${SCRATCH:-$(mktemp -d)}"
mkdir -p "$SCRATCH"
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
build() {  # name features [extra cargo flags]
  local dir="$SLICE/$1"
  ( cd "$SNAP/ffi/poc/codec" && eval CARGO_TARGET_DIR="\"$dir\"" cargo build --release -q -p ak-core ${3:-} --features "\"$2\"" 2>"$SCRATCH/cargo.err" ) \
    || { cat "$SCRATCH/cargo.err"; exit 1; }
  echo "#   $1: --features $2 -> $(sha256sum "$dir/release/libak_core.so" | cut -c1-16) ($(nm -D --defined-only "$dir/release/libak_core.so" | grep -c ' T ak_') ak_* exports)"
}
build target-core "rpc,init-guard"
build target-core-count "rpc,count,init-guard"
build target-core-corpus "corpus,init-guard"
# WP5 step 10: the NO-UNKNOWN variant (ak-core without its default `unknown-fields`), each
# in its own target dir so it can never overwrite a full build's libak_core.so.
build target-core-nounk "rpc,init-guard" --no-default-features
build target-core-count-nounk "rpc,count,init-guard" --no-default-features
build target-core-corpus-nounk "corpus,init-guard" --no-default-features
( cd "$SNAP/ffi/poc/csharp/abi" && CARGO_TARGET_DIR="$SLICE/target-probe" cargo run --release -q 2>"$SCRATCH/cargo.err" > "$SLICE/target-core/layout.json" ) || { cat "$SCRATCH/cargo.err"; exit 1; }
( cd "$SNAP/ffi/poc/csharp/abi" && CARGO_TARGET_DIR="$SLICE/target-probe-corpus" cargo run --release -q --features corpus 2>"$SCRATCH/cargo.err" > "$SLICE/target-core-corpus/layout.json" ) || { cat "$SCRATCH/cargo.err"; exit 1; }
( cd "$SNAP/ffi/poc/csharp/abi" && CARGO_TARGET_DIR="$SLICE/target-probe-nounk" cargo run --release -q --no-default-features 2>"$SCRATCH/cargo.err" > "$SLICE/target-core-nounk/layout.json" ) || { cat "$SCRATCH/cargo.err"; exit 1; }
( cd "$SNAP/ffi/poc/csharp/abi" && CARGO_TARGET_DIR="$SLICE/target-probe-corpus-nounk" cargo run --release -q --no-default-features --features corpus 2>"$SCRATCH/cargo.err" > "$SLICE/target-core-corpus-nounk/layout.json" ) || { cat "$SCRATCH/cargo.err"; exit 1; }
echo "#   layout probe: $(grep -c '"size"' "$SLICE/target-core/layout.json") structs (shapes), $(grep -c '"size"' "$SLICE/target-core-corpus/layout.json") structs (corpus)"
echo "#   layout probe, no-unknown: $(grep -c '"size"' "$SLICE/target-core-nounk/layout.json") structs (shapes), $(grep -c '"size"' "$SLICE/target-core-corpus-nounk/layout.json") structs (corpus)"
# D11 as amended (2026-10-03): the h2-batch twins of the four cores with the transport (rpc),
# h2 0.4.19 + poc/codec/h2-batch/h2-batch.patch, built from the same snapshot: the first through
# h2-batch/build.sh (which materialises and checks the patched h2 source), the other three with
# that source through cargo's --config patch table (as poc/cpp/gen/h2batch_nounk.sh does). The
# corpus cores have no transport, so no h2 twin. Each core's compiled-in h2 is printed.
AK_CARGO_PREFIX="" bash "$SNAP/ffi/poc/codec/h2-batch/build.sh" h2-batch "$SLICE/target-core-h2b" "rpc,init-guard" > "$SCRATCH/h2b.out" 2>&1 || { cat "$SCRATCH/h2b.out"; exit 1; }
H2SRC="$SLICE/target-core-h2b/h2-batch-src"
H2CFG="--config 'patch.crates-io.h2.path=\"$H2SRC\"'"
build target-core-count-h2b "rpc,count,init-guard" "$H2CFG"
build target-core-nounk-h2b "rpc,init-guard" "--no-default-features $H2CFG"
build target-core-count-nounk-h2b "rpc,count,init-guard" "--no-default-features $H2CFG"
echo "#   target-core-h2b: --features rpc,init-guard via h2-batch/build.sh -> $(sha256sum "$SLICE/target-core-h2b/release/libak_core.so" | cut -c1-16)"
for d in target-core target-core-count target-core-nounk target-core-count-nounk; do
  for v in "" -h2b; do
    echo "#   $d$v: h2 compiled in: $(strings "$SLICE/$d$v/release/libak_core.so" | grep -o '[^/]*/src/codec/framed_write\.rs' | sort -u | tr '\n' ' ')"
  done
done
cp "$SLICE/target-core/layout.json" "$SLICE/target-core-h2b/layout.json"
cp "$SLICE/target-core-nounk/layout.json" "$SLICE/target-core-nounk-h2b/layout.json"
for d in target-core target-core-nounk; do
  echo "#   $d: $(nm -D --defined-only "$SLICE/$d/release/libak_core.so" | grep -c ' T ak_uencode_') ak_uencode_* exports"
done
