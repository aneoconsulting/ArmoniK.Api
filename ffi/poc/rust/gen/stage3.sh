#!/usr/bin/env bash
# Stage 3, M2, end to end and reproducible.
#
# 1. the generator is up to date with shapes.json (R1);
# 2. byte identity across all four arms on M1 and M2, against the validated manifest (R2),
#    plus value identity across the three facade decoders on the same bytes;
# 3. boundary-call counts from a counting build, with the per-site length-prefix misses
#    that ABI v1 open decision 5 turns on, and the regression that catches an open-state
#    leak;
# 4. R5's two proofs from the artifact rather than claimed in the log: the FFI entry points
#    are unresolved imports, AND the no-boundary control is not fused into the benchmark
#    loop (an arm named 'no boundary' is a control only if it is not one);
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
echo "===== 1b. ABI v1 section 8: the generator-time refusal ====="
python3 gen/check_direct.py

echo
echo "===== 2b. shape coverage: explicit presence, the oneof, unknown fields ====="
cargo run --release -q -p harness --bin shapes 2>/dev/null

echo
echo "===== 3. boundary-call counts, and decision 5 by site ====="
cargo run --release -q -p harness --features count --bin counts 2>/dev/null

echo
echo "===== 4. the boundary is real (README R5) ====="
cargo build --release -q -p harness 2>/dev/null
BIN=target/release/conformance
echo "# ABI entry points the host imports from the core, resolved by the dynamic linker."
nm -D --undefined-only "$BIN" | grep ' ak_' || echo "NONE -- the boundary is gone"
echo
readelf -d "$BIN" | grep NEEDED | grep ak_core || echo "libak_core.so NOT a dependency"

echo
echo "===== 4b. the NO-BOUNDARY CONTROL is a control (README R5) ====="
echo "# An arm named 'no boundary' is one only if it is NOT fused into the benchmark loop,"
echo "# and that depends on LTO, on #[inline] on the entry point, and on whether the entry"
echo "# point is generic -- none of which appear in a configuration line."
./gen/inline_check.sh

echo
echo "===== 5. timings, guard ON, three runs ====="
for i in 1 2 3; do
  echo "--- run $i ---"
  cargo run --release -q -p harness --bin bench 2>/dev/null
done

echo
echo "===== 6. timings, guard OFF (a second process; prost is the control column) ====="
# R-D9: this row prices the ACCESSOR guard (`guard`). `init-guard` became a default
# feature too, so `--no-default-features` alone would drop both and conflate them; the
# init guard is put back so the row changes exactly one thing. Own target dir.
CARGO_TARGET_DIR=target-noguard cargo run --release -q -p harness --no-default-features --features init-guard --bin bench 2>/dev/null

echo
echo "===== 6b. M4 to M7 timings (filtered; the crossing and decision 5 rows come along) ====="
AK_BENCH_ONLY=P4.1,P5.3,P5.4,P6.1 cargo run --release -q -p harness --bin bench 2>/dev/null

echo
echo "===== 7. content sets on the string path (ABI v1 open decision 3) ====="
cargo run --release -q -p harness --bin content 2>/dev/null
