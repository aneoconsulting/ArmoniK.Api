#!/usr/bin/env bash
# Stage 1, end to end and reproducible.
#
#   gen/stage1.sh <scratch dir>
#
# 1. regenerate all sixteen payloads from ffi/schema/emit (the committed vectors cover 7);
# 2. check the seven committed vectors are what the emitters produce today;
# 3. re-derive every payload in Rust from the value rules, encode with prost, compare BYTES.
#
# Nothing under ffi/schema/ is written. A slice does not own it.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRATCH="${1:?usage: gen/stage1.sh <scratch dir>}"
VECTORS="$HERE/../../../schema/generated/payloads"

mkdir -p "$SCRATCH"
python3 "$HERE/dump_payloads.py" "$SCRATCH/payloads" >"$SCRATCH/dump.txt"
cat "$SCRATCH/dump.txt"
echo
cargo run --release -q --manifest-path "$HERE/../Cargo.toml" -p stage1-validate -- \
    "$SCRATCH/payloads" --vectors "$VECTORS"
