#!/usr/bin/env bash
# ABI v1 conformance obligation 12.5: the concurrency suite, end to end.
#
# Two builds of the same sources, differing in one feature:
#   default              the learned-width table per context, as section 6 specifies
#   --features global-widths   one table for the whole process, which section 6 refuses
#
# Both produce correct bytes (`Mark` carries its width by value), so the difference
# between them is the table's location and nothing else. Separate CARGO_TARGET_DIRs so
# neither build is the other's stale artifact -- the hazard that would have linked a
# pre-move .so after W10, caught then and worth not repeating.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

echo "===== 0. configuration ====="
rustc --version
nproc
grep -m1 'model name' /proc/cpuinfo || true

echo
echo "===== 1. build both arms ====="
cargo build --release -q -p harness --bin concur
CARGO_TARGET_DIR=target-gw cargo build --release -q -p harness --features global-widths --bin concur
echo "# the shared object each binary actually loaded, from ldd and not from the build log:"
ldd target/release/concur | grep ak_core
ldd target-gw/release/concur | grep ak_core

echo
echo "===== 1b. the shared-site pair really does flip a width (counting build) ====="
echo "# A contention arm that never contends measures nothing, so the flipping is counted."
CARGO_TARGET_DIR=target-count cargo run --release -q -p harness --features count --bin concur 2>/dev/null \
  | sed -n '/does the shared-site/,/^# visible to/p' | head -14

echo
echo "===== 2. per-context table (ABI v1 section 6 as specified), three runs ====="
for i in 1 2 3; do
  echo
  echo "----- per-context run $i -----"
  ./target/release/concur 2>/dev/null
done

echo
echo "===== 3. process-global table (the arrangement section 6 refuses), three runs ====="
echo "# Correctness sections are repeated here on purpose: if a shared table could corrupt"
echo "# bytes, this is the build where it would, and the suite says it does not."
for i in 1 2 3; do
  echo
  echo "----- global run $i -----"
  ./target-gw/release/concur 2>/dev/null
done
