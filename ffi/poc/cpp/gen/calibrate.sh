#!/usr/bin/env bash
# R13: slices run on separate machines, so every slice calibrates its own.
#
# The 1.8 ns Rust crossing of README section 2 is a fact about the container the RUST slice
# ran in, not about this one. This builds `ffi/poc/rust` HERE and runs its own crossing
# benchmark, so every absolute this slice reports is also quotable as a multiple of this
# machine's Rust crossing and the cross-language table stays reconstructible.
#
# It writes nothing under ffi/poc/rust: CARGO_TARGET_DIR points into this slice's build
# directory, which is the slice's own and is gitignored.
set -eu
cd "$(dirname "$0")/.." || exit 2
HERE=$PWD
RUST=$PWD/../rust
export CARGO_TARGET_DIR=$HERE/build/rustcal
echo "== R13 calibration: the rust slice's crossing benchmark, on THIS machine =="
uname -srm
grep -m1 'model name' /proc/cpuinfo || true
rustc --version
echo
(cd "$RUST" && cargo build --release --bin bench >/dev/null 2>&1)
echo "# rust slice bench, AK_BENCH_ONLY=P1.1 (the crossing rows are always kept)"
(cd "$RUST" && AK_BENCH_ONLY=P1.1 "$CARGO_TARGET_DIR/release/bench" 2>&1) | grep -E 'crossing|^#|linkage|cdylib' | head -40
