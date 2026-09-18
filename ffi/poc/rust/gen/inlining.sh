#!/usr/bin/env bash
# Arm 1: separate "was not inlined" from "crossed a boundary and materialised a group".
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

echo "===== 0. generator is current (R1) ====="
python3 gen/generate.py --check

echo
echo "===== 1. is core-native inlined into the benchmark loop? From the artifact. ====="
./gen/inline_check.sh

echo
echo "===== 2. boundary-call counts, unchanged by any of this ====="
cargo run --release -q -p harness --features count --bin counts 2>/dev/null | sed -n '1,20p'

for i in 1 2 3; do
  echo
  echo "=============================================================================="
  echo "===== 3. run $i ====="
  echo "=============================================================================="
  cargo run --release -q -p harness --bin inlining 2>/dev/null
done
