#!/usr/bin/env bash
# FIX-PLAN D23: decode-read through the three decode families of the shared core, side by side
# in ONE process per launch: core-ffi (push), core-ffi-pull (pull, walk in place) and
# core-ffi-fsm (the FSM family, begin + next to the end event), unknown fields dropped and
# retained, on every input of the codec suite (the 16 shapes, the Latin-1 and wide content
# sets, every U-* row the shapes roots carry).
# CONTAINER INSTRUMENTATION: no figure from this script is a campaign result.
#
#   gen/d23_bench.sh OUT_DIR
#
# Settings: AK_FSM=1 AK_CAMPAIGN_GRID=full AK_CASE_ARMS=core-ffi,core-ffi-pull,core-ffi-fsm
# AK_CASE_DIRS=decode-read AK_SAMPLES=10 AK_WARMUP_MS=80 AK_MEASURE_MS=150, pinned to CPU
# $AK_D23_CPU (default 1) with taskset; $AK_D23_LAUNCHES (default 3) processes, launch n with
# seed n: the arm blocks and the cases inside each block in a seeded random order, so the arms
# alternate position across processes. Before each launch the 1-minute load average must
# fall under 1.0 (waits up to 120 s, recorded). Every process runs the suite's pre-check
# first (with AK_FSM=1 it includes the FSM differential and the FSM == push value check).
set -Eeuo pipefail
OUT=${1:?usage: gen/d23_bench.sh OUT_DIR}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; RUST=$(cd "$HERE/.." && pwd)
cd "$RUST"
. "$HERE/machine_header.sh"
CPU=${AK_D23_CPU:-1}; LAUNCHES=${AK_D23_LAUNCHES:-3}
exe_of() {
  python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1
}
E=$(CARGO_TARGET_DIR="$RUST/target" cargo bench -q -p campaign --bench codec_suite --no-run --message-format=json 2>/dev/null | exe_of)
cp "$E" "$(dirname "$E")/codec_suite-d23"; E="$(dirname "$E")/codec_suite-d23"
CORE=$(ldd "$E" | grep -o '/[^ ]*libak_core.so')
H="$OUT/header.txt"
{
  echo "# D23 decode families measurement, $(date -u +%FT%TZ). CONTAINER INSTRUMENTATION, not a campaign result."
  echo "# commit $(git rev-parse HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/rust or poc/codec')"
  echo "# $(rustc --version); nproc $(nproc)"
  machine_header
  echo "# exe $E (sha256 $(sha256sum "$E" | cut -c1-16)) loads $CORE (sha256 $(sha256sum "$CORE" | cut -c1-16); FSM entries exported: $(nm -D --defined-only "$CORE" | grep -c ' T ak_fsm_'))"
  echo "# settings: AK_FSM=1 AK_CAMPAIGN_GRID=full AK_CASE_ARMS=core-ffi,core-ffi-pull,core-ffi-fsm AK_CASE_DIRS=decode-read AK_SAMPLES=10 AK_WARMUP_MS=80 AK_MEASURE_MS=150; $LAUNCHES launches; CPU $CPU (taskset); nothing else of this session runs meanwhile"
} > "$H"
cat "$H"
T0=$(date +%s)
for l in $(seq 1 "$LAUNCHES"); do
  w=0
  while awk '{ exit !($1 >= 1.0) }' /proc/loadavg && [ $w -lt 120 ]; do sleep 5; w=$((w + 5)); done
  echo "# launch $l: loadavg before $(cut -d' ' -f1-3 /proc/loadavg) (waited $w s)" | tee -a "$H"
  CH=$(mktemp -d)
  CRITERION_HOME="$CH" AK_LAUNCH=$l AK_OUT="$OUT/codec-$l.jsonl" AK_FSM=1 AK_CAMPAIGN_GRID=full \
    AK_CASE_ARMS=core-ffi,core-ffi-pull,core-ffi-fsm AK_CASE_DIRS=decode-read \
    AK_SAMPLES=10 AK_WARMUP_MS=80 AK_MEASURE_MS=150 AK_NRESAMPLES=1000 \
    taskset -c "$CPU" "$E" --bench > "$OUT/codec-$l.out" 2> "$OUT/codec-$l.err"
  rm -rf "$CH"
  echo "launch $l: $(grep -m1 '^# precheck' "$OUT/codec-$l.err"); arm order: $(grep -m1 'arm order' "$OUT/codec-$l.jsonl" | cut -c1-120)" | tee -a "$H"
done
echo "# benchmark wall: $(( $(date +%s) - T0 )) s; loadavg after $(cut -d' ' -f1-3 /proc/loadavg)" | tee -a "$H"
echo "tables: python3 gen/d23_tables.py $OUT EVENTS_LOG > $OUT/tables.md"
