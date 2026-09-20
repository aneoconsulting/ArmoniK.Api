#!/usr/bin/env bash
# ABI v1 section 7.1's two decode families, end to end. Open decision 2.
#
# 1. the generator is up to date (R1), and the core is still ONE core (R0);
# 2. the push family's own gate still passes, because this work added a field to the
#    shared decode context and an additive change is only additive if it measures so;
# 3. correctness: every payload of every root, four decoders, one value;
# 4. crossings for BOTH families from a counting build, plus the structural control
#    (records written == reverse calls push would have made);
# 5. R5, from the artifact: the parse entry points are unresolved imports;
# 6. timings, three runs, one process each, with the opaque-replay control.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

echo "===== 0. configuration ====="
rustc --version
echo "incumbent: prost $(grep -m1 'name = "prost"' -A1 Cargo.lock | grep version | cut -d'"' -f2)"
echo "core:      $(cd ../codec && git rev-parse --short HEAD 2>/dev/null || echo '-')"
nproc; grep -m1 'model name' /proc/cpuinfo || true

echo
echo "===== 1. generator is current, and there is one core ====="
python3 gen/generate.py --check
../codec/gen/one_core.sh

echo
echo "===== 2. the PUSH family's gate still passes ====="
echo "# This work added ak_parse_* beside ak_decode_* and one field to DecCtxImpl."
echo "# Additive is a claim about behaviour, so the existing gate is re-run rather than"
echo "# assumed: byte identity across all four arms on every payload."
cargo run --release -q -p harness --bin conformance 2>/dev/null | tail -20

echo
echo "===== 3+4. correctness, crossings and the structural control ====="
cargo run --release -q -p harness --features count --bin pullbench 2>/dev/null

echo
echo "===== 5. the boundary is real for the PULL entry points too (R5) ====="
cargo build --release -q -p harness 2>/dev/null
BIN=target/release/pullbench
echo "# ak_parse_* and ak_bdr_* as undefined dynamic imports, resolved by the linker:"
nm -D --undefined-only "$BIN" | grep -E ' ak_(parse|bdr)' || echo "NONE -- the boundary is gone"
echo
echo "# and the shared object that actually got loaded, from ldd and not from the build log:"
ldd "$BIN" | grep ak_core

echo
echo "===== 6. timings, three runs ====="
for i in 1 2 3; do
  echo
  echo "----- run $i -----"
  ./target/release/pullbench 2>/dev/null | sed -n '/## 4/,$p'
done
