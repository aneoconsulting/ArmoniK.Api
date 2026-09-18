#!/usr/bin/env bash
# Arm 2: the zeroed-group variant of the element fill (ABI v1 open decision 9 candidate).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

echo "===== 0. generator is current (R1) ====="
python3 gen/generate.py --check

echo
echo "===== 1. the default path is untouched: conformance, all arms, all payloads ====="
cargo run --release -q -p harness --bin conformance 2>/dev/null | tail -2

echo
echo "===== 2. boundary-call counts, DEFAULT path, unchanged by the added arm ====="
cargo run --release -q -p harness --features count --bin counts 2>/dev/null | sed -n '5,32p'

for i in 1 2 3; do
  echo
  echo "=============================================================================="
  echo "===== 3. run $i ====="
  echo "=============================================================================="
  cargo run --release -q -p harness --bin zeroed 2>/dev/null
done
