#!/usr/bin/env bash
# One optimisation step's measurement: gen/opt_bench.sh into logs/rust/opt/NAME, then
# gen/opt_compare.py against PREV (the previous kept step) and against baseline2, both
# calibrated with the A/A pair baseline2 / baseline2-aa. CONTAINER INSTRUMENTATION.
#   gen/opt_step.sh NAME PREV
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
O="$HERE/../../logs/rust/opt"
[ $# = 2 ] || { echo "usage: $0 NAME PREV" >&2; exit 2; }
bash "$HERE/gen/opt_bench.sh" "$O/$1"
cmp() { python3 "$HERE/gen/opt_compare.py" "$O/$1" "$O/$2" --aa "$O/baseline2" "$O/baseline2-aa" > "$O/$2/compare-vs-$1.txt"; }
cmp "$2" "$1"
[ "$2" = baseline2 ] || cmp baseline2 "$1"
echo "compared: $O/$1/compare-vs-$2.txt and $O/$1/compare-vs-baseline2.txt"
