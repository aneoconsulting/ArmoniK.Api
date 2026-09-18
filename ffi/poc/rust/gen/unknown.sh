#!/usr/bin/env bash
# ABI v1 open decision 11: what an optional bag of unknown fields costs.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

echo "===== 0. generator is current (R1) ====="
python3 gen/generate.py --check

echo
echo "===== 1. THE GATING QUESTION: does the bag break the batching predicate? ====="
python3 gen/unknown_predicate.py

echo
echo "===== 2. the default path is untouched: conformance, every arm, every payload ====="
cargo run --release -q -p harness --bin conformance 2>/dev/null | tail -2

echo
echo "===== 3. the boundary is real, and the no-boundary control is one (R5) ====="
./gen/inline_check.sh 2>/dev/null | sed -n '1,14p'

echo
echo "===== 4. boundary-call counts, DEFAULT path, unchanged by the added arm ====="
cargo run --release -q -p harness --features count --bin counts 2>/dev/null | sed -n '5,20p'

for i in 1 2 3; do
  echo
  echo "=============================================================================="
  echo "===== 5. run $i ====="
  echo "=============================================================================="
  cargo run --release -q -p harness --bin unknown 2>/dev/null
done
