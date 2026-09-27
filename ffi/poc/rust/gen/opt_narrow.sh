#!/usr/bin/env bash
# A NARROWED opt_bench run for iterating inside one step (owner's pace note): the codec
# suite only, on the inputs a change can affect, with opt_bench v5's payload settings, then
# the same summary. No crossings, no calib, no RPC. Instrumentation, like opt_bench; the full
# opt_bench runs once per step, on the commit that is kept.
#
#   gen/opt_narrow.sh OUT_DIR ONLY [BUILDS]
#     ONLY    comma-separated input id prefixes (AK_ONLY), e.g. P2.2,P2.3,P5.
#     BUILDS  full | nounk | both (default both)
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd)
cd "$HERE"
[ $# -ge 2 ] || { echo "usage: $0 OUT_DIR ONLY [full|nounk|both]" >&2; exit 2; }
mkdir -p "$1"; OUT="$(cd "$1" && pwd)"
ONLY=$2; BUILDS=${3:-both}
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1}
SAMPLES=10; WARMUP_ITERS=20; WARMUP_MS=20; MEASURE_MS=60
export AK_NRESAMPLES=1000 AK_ZC=P5.
LAUNCH=1
SCRATCH=$(mktemp -d); trap 'rm -rf "$SCRATCH"' EXIT
LOG="$OUT/runner.log"; : > "$LOG"
say() { echo "$*" | tee -a "$LOG"; }
REV=$(git rev-parse --short HEAD)
DIRTY=""; git diff --quiet HEAD -- . ../codec || DIRTY=" + UNCOMMITTED CHANGES"
say "# rust slice NARROWED optimisation run (gen/opt_narrow.sh): CONTAINER INSTRUMENTATION, codec suite only"
say "# commit $REV$DIRTY; date $(date -u +%FT%TZ); AK_ONLY=$ONLY builds=$BUILDS; samples=$SAMPLES warmup_iters=$WARMUP_ITERS warmup_ms=$WARMUP_MS measure_ms=$MEASURE_MS (opt_bench v5's payload settings); AK_NRESAMPLES=$AK_NRESAMPLES AK_ZC=$AK_ZC launch=$LAUNCH cpu=$AK_CPU_CLIENT"
bench_exe() {
  cargo bench -q -p campaign "$@" --bench codec_suite --no-run --message-format=json 2>/dev/null \
    | python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1
}
run() {  # run TAG EXE
  local F="$OUT/$1.jsonl" C="$OUT/$1.criterion.log" t=$(date +%s)
  CRITERION_HOME="$SCRATCH/crit-$1" AK_LAUNCH=$LAUNCH AK_OUT="$F" AK_ONLY=$ONLY AK_SAMPLES=$SAMPLES \
    AK_WARMUP_ITERS=$WARMUP_ITERS AK_WARMUP_MS=$WARMUP_MS AK_MEASURE_MS=$MEASURE_MS \
    taskset -c "$AK_CPU_CLIENT" "$2" > "$C" 2>&1 || { say "codec $1 FAILED: $C"; exit 1; }
  say "  $1: $(grep -m1 '^# precheck:' "$C" | sed 's/^# //'); $(grep -vc '^#' "$F") sample rows; $(( $(date +%s) - t ))s"
}
if [ "$BUILDS" != nounk ]; then run codec-P "$(bench_exe)"; fi
if [ "$BUILDS" != full ]; then
  run codec-nounk-P "$(CARGO_TARGET_DIR="$HERE/target-nounk" bench_exe --no-default-features --features init-guard)"
fi
python3 gen/opt_summary.py "$OUT" | tee -a "$LOG"
