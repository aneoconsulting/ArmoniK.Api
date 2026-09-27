#!/usr/bin/env bash
# The per-step checks of the second optimisation unit (no gate, no timing):
#   1. the shared generator's --check (every slice's committed output current)
#   2. one_core.sh (R0: one core)
#   3. the codec suite's pre-check on both builds (AK_PRECHECK_ONLY; 0 failures required)
#   4. the counting builds against the committed crossing files (a difference is RECORDED,
#      as crossings-current.txt / .diff in OUT, not fatal: some steps change counts)
# Usage: gen/step_checks.sh OUT_DIR     (writes checks.log and any crossing diffs there)
set -uo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd)
CODEC=$(cd "$HERE/../codec" && pwd)
OUT=${1:?usage: step_checks.sh OUT_DIR}
mkdir -p "$OUT"
LOG="$OUT/checks.log"
: > "$LOG"
rc=0
say() { echo "$*" | tee -a "$LOG"; }
cd "$HERE"
say "# step checks at $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' (tree dirty)'), $(date -u +%FT%TZ)"
if python3 "$CODEC/gen/generate.py" --check >> "$LOG" 2>&1; then say "generate.py --check: ok"; else say "generate.py --check: FAILED"; rc=1; fi
if bash "$CODEC/gen/one_core.sh" >> "$LOG" 2>&1; then say "one_core.sh: ok"; else say "one_core.sh: FAILED"; rc=1; fi
bench_exe() {
  cargo bench -q -p campaign "$@" --bench codec_suite --no-run --message-format=json 2>/dev/null \
    | python3 -c 'import sys,json
for l in sys.stdin:
    try: o=json.loads(l)
    except Exception: continue
    if o.get("reason")=="compiler-artifact" and o.get("executable") and o["target"]["name"]=="codec_suite": print(o["executable"])' | tail -1
}
SCR=$(mktemp -d)
B=$(bench_exe)
BN=$(CARGO_TARGET_DIR="$HERE/target-nounk" bench_exe --no-default-features --features init-guard)
for pair in "full:$B" "nounk:$BN"; do
  v=${pair%%:*} exe=${pair#*:}
  if [ -z "$exe" ]; then say "pre-check $v: BUILD FAILED"; rc=1; continue; fi
  line=$(CRITERION_HOME="$SCR/crit" AK_OUT="$SCR/pc.jsonl" AK_PRECHECK_ONLY=1 AK_ZC=P5. "$exe" 2>&1 | tee -a "$LOG" | grep -m1 '^# precheck:')
  say "pre-check $v: ${line#\# }"
  case "$line" in *" 0 failures"*) ;; *) rc=1 ;; esac
done
rm -rf "$SCR"
got="$OUT/crossings-current.txt"
CARGO_TARGET_DIR="$HERE/target-count" cargo run --release -q -p campaign --features count --bin crossings 2>/dev/null > "$got"
if diff -u gen/crossings.txt "$got" > "$OUT/crossings.diff"; then
  say "crossings (full): $(wc -l < "$got") rows identical to gen/crossings.txt"; rm -f "$OUT/crossings.diff" "$got"
else
  say "crossings (full): DIFFER from gen/crossings.txt, $(grep -c '^[-+][^-+]' "$OUT/crossings.diff") changed lines, see crossings.diff"
fi
got="$OUT/crossings-nounk-current.txt"
CARGO_TARGET_DIR="$HERE/target-count-nounk" cargo run --release -q -p campaign --no-default-features \
  --features count,init-guard --bin crossings 2>/dev/null > "$got"
if diff -u gen/crossings-nounk.txt "$got" > "$OUT/crossings-nounk.diff"; then
  say "crossings (nounk): $(wc -l < "$got") rows identical to gen/crossings-nounk.txt"; rm -f "$OUT/crossings-nounk.diff" "$got"
else
  say "crossings (nounk): DIFFER from gen/crossings-nounk.txt, $(grep -c '^[-+][^-+]' "$OUT/crossings-nounk.diff") changed lines, see crossings-nounk.diff"
fi
say "checks rc=$rc"
exit $rc
