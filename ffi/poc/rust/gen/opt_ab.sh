#!/usr/bin/env bash
# Alternated narrowed runs, A = a worktree of the previous kept commit, B = this tree:
#   gen/opt_ab.sh OUT_DIR A_WORKTREE ONLY [PAIRS] [BUILDS]
# A1 B1 A2 B2 ... with gen/opt_narrow.sh in each tree, then gen/opt_ab.py. Instrumentation.
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd)
OUT=$1; A=$2; ONLY=$3; N=${4:-3}; BUILDS=${5:-full}
mkdir -p "$OUT"
cp "$HERE/gen/opt_narrow.sh" "$A/ffi/poc/rust/gen/opt_narrow.sh"
for r in $(seq "$N"); do
  (cd "$A/ffi/poc/rust" && bash gen/opt_narrow.sh "$OUT/A$r" "$ONLY" "$BUILDS" > /dev/null)
  (cd "$HERE" && bash gen/opt_narrow.sh "$OUT/B$r" "$ONLY" "$BUILDS" > /dev/null)
done
python3 "$HERE/gen/opt_ab.py" "$OUT" > "$OUT/ab-summary.txt"
echo "A = $(cd "$A" && git rev-parse --short HEAD), B = $(cd "$HERE" && git rev-parse --short HEAD)$(cd "$HERE" && git diff --quiet HEAD -- . ../codec || echo ' + UNCOMMITTED')" >> "$OUT/ab-summary.txt"
