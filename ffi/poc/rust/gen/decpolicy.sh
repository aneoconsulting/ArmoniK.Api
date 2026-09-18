#!/usr/bin/env bash
# ABI v1 open decision 3, third framing: the decode side's UTF-8 policy, priced.
#
# The policy is a BUILD-TIME choice in ak_rt::strings, so the three columns are three
# processes and R4 does not hold across them. Two things are done about that:
#
#   1. the in-process control is the `prost` row, which validates under every one of these
#      builds and which the feature flag cannot reach, so `/ prost` is the column that
#      travels;
#   2. the three binaries are built FIRST and then run ROUND ROBIN with a rotating order,
#      so no policy is always the first process on a cold machine. The first version of
#      this script built-and-ran each policy in turn and the lossy section -- always first
#      -- came out 9 points off its own value in a second invocation. That is an ordering
#      artefact, not a policy effect, and this is what removes it.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
STAGE="${TMPDIR:-/tmp}/ak-decpolicy"
rm -rf "$STAGE"; mkdir -p "$STAGE"

NAMES=(lossy reject-scalar reject-simd)
FLAGS=("" "--features dec-reject" "--features dec-reject-simd")

echo "===== 0. generator is current (R1) ====="
python3 gen/generate.py --check

for i in 0 1 2; do
  echo
  echo "===== build: ${NAMES[$i]}  ${FLAGS[$i]:-<default>} ====="
  cargo build --release -q -p harness ${FLAGS[$i]} --bin conformance --bin decpolicy 2>/dev/null
  mkdir -p "$STAGE/${NAMES[$i]}"
  cp target/release/decpolicy   "$STAGE/${NAMES[$i]}/decpolicy"
  cp target/release/conformance "$STAGE/${NAMES[$i]}/conformance"
  cp target/release/deps/libak_core.so "$STAGE/${NAMES[$i]}/libak_core.so"
  echo "--- conformance: byte identity against the validated manifest ---"
  LD_LIBRARY_PATH="$STAGE/${NAMES[$i]}" "$STAGE/${NAMES[$i]}/conformance" 2>/dev/null | tail -2
done

# Round robin, rotating the order, so ordering cannot be read as a policy effect.
for r in 0 1 2; do
  for k in 0 1 2; do
    i=$(( (k + r) % 3 ))
    echo
    echo "=============================================================================="
    echo "===== round $((r+1)), policy ${NAMES[$i]} ====="
    echo "=============================================================================="
    LD_LIBRARY_PATH="$STAGE/${NAMES[$i]}" "$STAGE/${NAMES[$i]}/decpolicy" 2>/dev/null
  done
done
