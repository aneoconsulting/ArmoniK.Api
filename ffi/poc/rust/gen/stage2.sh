#!/usr/bin/env bash
# Stage 2, end to end and reproducible.
#
#   gen/stage2.sh
#
# 1. the generator is up to date with shapes.json (R1);
# 2. byte identity across all four arms against the validated manifest (R2);
# 3. boundary-call counts from a counting build (R5);
# 4. the boundary is a real dynamic-linker call and not an inlined one;
# 5. timings, all arms, one process, guard on (R3, R4) -- three runs;
# 6. the same with the guard off, which is a SECOND process, so it carries prost as an
#    in-process control column in both.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

echo "===== 1. generator is current ====="
python3 gen/generate.py --check

echo
echo "===== 2. conformance: byte identity across the arms ====="
cargo run --release -q -p harness --bin conformance 2>/dev/null

echo
echo "===== 3. boundary-call counts (counting build) ====="
cargo run --release -q -p harness --features count --bin counts 2>/dev/null

echo
echo "===== 4. the boundary is real ====="
BIN=target/release/conformance
cargo build --release -q -p harness 2>/dev/null
echo "# ABI entry points the host imports from the core, resolved by the dynamic linker."
echo "# If these were statically linked rlib symbols rustc would inline them and the arm"
echo "# would be measuring the optimiser; it did, before this was found."
nm -D --undefined-only "$BIN" | grep ' ak_' || echo "NONE -- the boundary is gone"
echo
readelf -d "$BIN" | grep NEEDED | grep ak_core || echo "libak_core.so NOT a dependency"

echo
echo "===== 5. timings, guard ON, three runs ====="
for i in 1 2 3; do
  echo "--- run $i ---"
  cargo run --release -q -p harness --bin bench 2>/dev/null
done

echo
echo "===== 6. timings, guard OFF (a second process; prost is the control column) ====="
cargo run --release -q -p harness --no-default-features --bin bench 2>/dev/null

echo
echo "===== the NO-BOUNDARY CONTROL is a control (README R5) ====="
echo "# Printed from the artifact, both directions: the entry point's size against the"
echo "# calling closure's size. If a closure is larger than the traversal it calls, the"
echo "# control has been fused into the benchmark loop and its subtraction is not valid."
./gen/inline_check.sh
