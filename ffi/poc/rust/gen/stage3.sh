#!/usr/bin/env bash
# Stage 3, M2, end to end and reproducible.
#
# 1. the generator is up to date with shapes.json (R1);
# 2. byte identity across all four arms on M1 and M2, against the validated manifest (R2),
#    plus value identity across the three facade decoders on the same bytes;
# 3. boundary-call counts from a counting build, with the per-site length-prefix misses
#    that ABI v1 open decision 5 turns on, and the regression that catches an open-state
#    leak;
# 4. R5's second half: the entry points are unresolved imports in the built artifact, shown
#    from the artifact rather than claimed in the log;
# 5. timings, all arms, one process, guard on (R3, R4), three runs, ending with the
#    decision 5 isolation;
# 6. the same with the guard off, a second process, prost carried as the control column.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

echo "===== 1. generator is current ====="
python3 gen/generate.py --check

echo
echo "===== 2. conformance: byte identity across the arms ====="
cargo run --release -q -p harness --bin conformance 2>/dev/null

echo
echo "===== 2b. shape coverage: explicit presence, the oneof, unknown fields ====="
cargo run --release -q -p harness --bin shapes 2>/dev/null

echo
echo "===== 3. boundary-call counts, and decision 5 by site ====="
cargo run --release -q -p harness --features count --bin counts 2>/dev/null

echo
echo "===== 4. the boundary is real (README R5, second half) ====="
cargo build --release -q -p harness 2>/dev/null
BIN=target/release/conformance
echo "# ABI entry points the host imports from the core, resolved by the dynamic linker."
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
echo "===== 7. content sets on the string path (ABI v1 open decision 3) ====="
cargo run --release -q -p harness --bin content 2>/dev/null
