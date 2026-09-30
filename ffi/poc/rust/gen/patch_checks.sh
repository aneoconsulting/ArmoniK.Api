#!/usr/bin/env bash
# Checks for a core patch experiment (2026-09-30): in the tree this script lives in (a private
# worktree carrying the patch), build on the OS CPUs and run the codec pre-check on both builds
# (AK_PRECHECK_ONLY, 0 failures required), bin upload_check and bin rpc_semantics (full build).
#   gen/patch_checks.sh OUT_DIR       (checks.log there; rc 0 iff all pass)
set -uo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?usage: patch_checks.sh OUT_DIR}; mkdir -p "$OUT"; LOG="$OUT/checks.log"; : > "$LOG"
export PATH="$HERE/gen/cargo-shim:$PATH"
OSCPU=${AK_OS_CPUS:-0,9,10,19}
say() { echo "$*" | tee -a "$LOG"; }
rc=0
say "# patch checks in $HERE at $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + patch'), $(date -u +%FT%TZ)"
git diff --stat HEAD -- ../codec . >> "$LOG"
bench_exe() {
  taskset -c "$OSCPU" cargo bench -q -p campaign "$@" --bench codec_suite --no-run --message-format=json 2>/dev/null \
    | python3 -c 'import sys,json
for l in sys.stdin:
    try: o=json.loads(l)
    except Exception: continue
    if o.get("reason")=="compiler-artifact" and o.get("executable") and o["target"]["name"]=="codec_suite": print(o["executable"])' | tail -1
}
SCR=$(mktemp -d)
B=$(CARGO_TARGET_DIR="$HERE/target" bench_exe)
BN=$(CARGO_TARGET_DIR="$HERE/target-nounk" bench_exe --no-default-features --features init-guard)
for pair in "full:$B" "nounk:$BN"; do
  v=${pair%%:*} exe=${pair#*:}
  if [ -z "$exe" ]; then say "pre-check $v: BUILD FAILED"; rc=1; continue; fi
  line=$(CRITERION_HOME="$SCR/crit" AK_OUT="$SCR/pc.jsonl" AK_PRECHECK_ONLY=1 AK_ZC=P5. taskset -c "$OSCPU" "$exe" 2>&1 | tee -a "$LOG" | grep -m1 '^# precheck:')
  say "pre-check $v: ${line#\# }"
  case "$line" in *" 0 failures"*) ;; *) rc=1 ;; esac
done
rm -rf "$SCR"
taskset -c "$OSCPU" env CARGO_TARGET_DIR="$HERE/target" cargo build --release -q -p campaign --bin upload_check --bin rpc_semantics --bin stream_probe 2>/dev/null || { say "build FAILED"; exit 1; }
for b in upload_check rpc_semantics; do
  taskset -c "$OSCPU" "$HERE/target/release/$b" > "$OUT/$b.log" 2>&1
  r=$?; say "$b: rc $r, $(tail -1 "$OUT/$b.log")"; [ $r = 0 ] || rc=1
done
say "core loaded by stream_probe: $(ldd "$HERE/target/release/stream_probe" | awk '/libak_core/{print $3}')"
say "checks rc=$rc"
exit $rc
