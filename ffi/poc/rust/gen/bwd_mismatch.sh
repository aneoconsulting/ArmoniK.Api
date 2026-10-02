#!/usr/bin/env bash
# EXPERIMENT backward-encode: the MISMATCHED pair. This checkout's binaries (the committed
# binding, which delivers every repeated field FIRST to last) loading the BACKWARD core
# (LD_LIBRARY_PATH, ahead of their RUNPATH): the call signatures are the same, so it links and
# runs; the question is which existing check notices. Nothing timed.
#
#   gen/bwd_mismatch.sh WORKTREE OUT_DIR
#
# Checks run, each as committed: conformance (byte identity against the manifest, 16
# payloads), shapes (presence, oneof, unknown-field vectors), the codec suite's pre-check
# (every timed arm on every input, the U-* corpus rows included), the corpus through the C
# ABI (corpus harness of this checkout against the worktree's corpus core).
set -uo pipefail
WT=${1:?usage: bwd_mismatch.sh WORKTREE OUT_DIR}; OUT=${2:?out dir}
WT=$(cd "$WT" && pwd); mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; RUST=$(cd "$HERE/.." && pwd); WR="$WT/ffi/poc/rust"
export AK_NO_TIMING=1
C="${TMPDIR:-/tmp}/bwd-mismatch-cores"; mkdir -p "$C/full" "$C/corpus"   # binaries stay out of logs/; sha256 recorded below
cp "$WR/target/release/deps/libak_core.so" "$C/full/"
cp "$WR/target-corpus/release/deps/libak_core.so" "$C/corpus/"
L="$OUT/mismatch.log"
{
echo "# backward-encode MISMATCHED pair, $(date -u +%FT%TZ): this checkout's binaries (forward-delivering binding) + the worktree's backward core"
echo "# full core   $C/full/libak_core.so sha256 $(sha256sum "$C/full/libak_core.so" | cut -c1-16) (from $WR/target)"
echo "# corpus core $C/corpus/libak_core.so sha256 $(sha256sum "$C/corpus/libak_core.so" | cut -c1-16)"
( cd "$RUST" && CARGO_TARGET_DIR="$RUST/target" cargo build --release -q -p harness --bin conformance --bin shapes )
B=$( cd "$RUST" && CARGO_TARGET_DIR="$RUST/target" cargo bench -q -p campaign --bench codec_suite --no-run --message-format=json 2>/dev/null \
    | python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1)
run() {  # label, core dir, command...
  local label=$1 dir=$2; shift 2
  echo; echo "===== $label ====="
  echo "# loads: $(LD_LIBRARY_PATH="$dir" ldd "$1" | grep -o '/[^ ]*libak_core.so')"
  local rc=0; LD_LIBRARY_PATH="$dir" "$@" > "$OUT/$label.out" 2>&1 || rc=$?
  echo "# rc $rc"
  grep -E 'VERDICT|PRECHECK|precheck|MISMATCH|differ|FAIL|wrong|passed|failed' "$OUT/$label.out" | head -40
}
run conformance "$C/full" "$RUST/target/release/conformance"
run shapes "$C/full" "$RUST/target/release/shapes"
CH=$(mktemp -d); CRITERION_HOME="$CH" AK_PRECHECK_ONLY=1 run precheck "$C/full" "$B"; rm -rf "$CH"
( cd "$RUST/corpus" && CARGO_TARGET_DIR="$RUST/target-corpus" cargo build --release -q 2>/dev/null )
( cd "$RUST" && run corpus "$C/corpus" "$RUST/target-corpus/release/corpus" )
} 2>&1 | tee "$L"
