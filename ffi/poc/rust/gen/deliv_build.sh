#!/usr/bin/env bash
# Builds for the response-delivery comparison (2026-10-01), under the bench lock:
#   flock /tmp/ak-physical-bench.lock gen/deliv_build.sh H2_BATCH_SRC
# target-deliv/          this tree's stream_probe (crates.io h2 in the host)
# target-deliv-h2batch/  the same with h2-batch in the host too (--config patch to H2_BATCH_SRC, the
#                        source poc/codec/h2-batch/build.sh materialised); Cargo.lock restored after.
# Prints each binary's sha256 and the h2 source compiled in.
set -Eeuo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
SRC=${1:?usage: deliv_build.sh H2_BATCH_SRC}
export PATH="$HERE/gen/cargo-shim:$PATH"
cp Cargo.lock "$HERE/target-deliv.Cargo.lock.saved" 2>/dev/null || true
trap 'cp "$HERE/target-deliv.Cargo.lock.saved" Cargo.lock; rm -f "$HERE/target-deliv.Cargo.lock.saved"' EXIT
CARGO_TARGET_DIR="$HERE/target-deliv" taskset -c 0,9,10,19 cargo build -q --release -p campaign --bin stream_probe
CARGO_TARGET_DIR="$HERE/target-deliv-h2batch" taskset -c 0,9,10,19 cargo build -q --release -p campaign --bin stream_probe \
    --config "patch.crates-io.h2.path=\"$SRC\""
for b in target-deliv target-deliv-h2batch; do
  f="$HERE/$b/release/stream_probe"
  echo "$(sha256sum "$f" | cut -c1-64)  $f"
  strings "$f" | grep -o '/[^ ]*/src/codec/framed_write\.rs' | grep -o '[^/]*/src/codec/framed_write\.rs$' | sort -u | sed 's/^/    h2 compiled in: /'
done
