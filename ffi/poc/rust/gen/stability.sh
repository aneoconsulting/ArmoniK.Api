#!/usr/bin/env bash
# Is a ratio reproducible across BUILDS of the same source? (README R4, as sharpened.)
#
# The binary is bit-identical across rebuilds of unchanged source -- checked, and printed
# below -- so "rebuild" on its own varies nothing. What varied between the published table
# and today is that the crate GREW, which moves code and data addresses. So the instrument
# is a semantically neutral LAYOUT PERTURBATION: k exported no-op functions in the harness,
# which no arm calls and which change nothing any arm does, and which shift the addresses of
# everything emitted after them.
#
# Each group: write the pad, rebuild, record the binary's hash (it must CHANGE, or the
# perturbation is not in the build), run the bench twice. Two runs of one binary give the
# same-binary spread; different k give the across-build spread. The last group repeats k=0
# so a same-binary comparison separated by the whole sweep is also available.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
PAYLOADS=P1.1,P1.2,P1.3,P2.1,P2.2,P2.3,P2.4,P2.5

emit_pad() {
  local k="$1"
  {
    echo "//! A semantically neutral layout perturbation (gen/stability.sh). Nothing calls"
    echo "//! these; they exist to move the addresses of everything emitted after them, which"
    echo "//! is what growing a crate does and what R4 says a ratio must survive."
    for ((i=0; i<k; i++)); do
      echo "#[no_mangle]"
      echo "pub extern \"C\" fn ak_pad_$i(x: u64) -> u64 { x ^ $i }"
    done
  } > crates/harness/src/pad.rs
}

for K in 0 3 11 37 101 0; do
  emit_pad "$K"
  cargo build --release -q -p harness --bin bench 2>/dev/null
  H=$(sha256sum target/release/bench | cut -c1-16)
  SZ=$(stat -c %s target/release/bench)
  for R in 1 2; do
    echo
    echo "=============================================================================="
    echo "===== pad=$K  binary=$H  size=$SZ  run=$R ====="
    echo "=============================================================================="
    AK_BENCH_ONLY=$PAYLOADS cargo run --release -q -p harness --bin bench 2>/dev/null
  done
done
emit_pad 0
