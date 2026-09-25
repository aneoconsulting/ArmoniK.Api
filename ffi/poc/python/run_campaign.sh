#!/usr/bin/env bash
# The python slice's campaign runner (design/CAMPAIGN.md requirement 31).
#
#   ./run_campaign.sh --suite codec|rpc|calib|gate --out <dir>   (CAMPAIGN.md 29: --out ffi/logs/python/campaign) [--launches 3] [--rounds 5]
#                     [--smoke] [--allow-dirty]
#
# Environment: AK_CPU_CLIENT, AK_CPU_SERVER (required for codec/rpc/calib unless --smoke),
#   AK_ISOLATION (the isolation mechanism, printed in every header), AK_PY (target, default
#   python3.12), AK_FLOOR_PY (floor, default build/py37/python3.7 when ./fetch_py37.sh ran),
#   AK_SNAPSHOT (commit to build the core from, default HEAD: a `git archive`, never the
#   working tree, so the run names a commit even while poc/codec is being edited).
#
# gate   build (every core WITH init-guard), then gate.sh at the target AND the floor
#        (payload-set byte identity on every arm, the whole corpus on every codec arm in both
#        unknown-field modes where built, the planted controls), plus the RPC runner's
#        must-fail control. Writes <out>/gate.ok on success.
# codec  (refuses without a gate.ok for this commit) the codec suite, families shapes and
#        unknown, one process per family per launch
# rpc    (refuses without gate.ok) the RPC grid, one client process per launch, the server
#        in its own pinned process
# calib  crossing counts (a gate: a difference stops it) and crossing cost, host and rust
# Every suite ends with camp_summary.py over <out> (requirement 30). The floor is gated,
# never timed.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE"
SUITE="" OUT="" LAUNCHES=3 ROUNDS=5 SMOKE="" DIRTY=""
while [ $# -gt 0 ]; do
  case "$1" in
    --suite) SUITE=$2; shift 2;;
    --out) OUT=$2; shift 2;;
    --launches) LAUNCHES=$2; shift 2;;
    --rounds) ROUNDS=$2; shift 2;;
    --smoke) SMOKE=--smoke; shift;;
    --allow-dirty) DIRTY=--allow-dirty; shift;;
    *) echo "unknown argument $1"; exit 2;;
  esac
done
[ -n "$SUITE" ] && [ -n "$OUT" ] || { echo "usage: $0 --suite codec|rpc|calib|gate --out <dir>"; exit 2; }
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
PY="${AK_PY:-python3.12}"
FLOOR="${AK_FLOOR_PY:-}"
[ -z "$FLOOR" ] && [ -x build/py37/python3.7 ] && FLOOR=build/py37/python3.7
export AK_SNAPSHOT="${AK_SNAPSHOT:-HEAD}"
SHA=$(git rev-parse --short "$AK_SNAPSHOT")
export AK_SNAPSHOT_DIR="$HERE/build/snap/$SHA"
export AK_CODECGEN="$AK_SNAPSHOT_DIR/ffi/poc/codec/gen"
# The gate stamp is keyed on the TREES this run reads (poc/python, poc/codec, schema, corpus at
# the snapshot), not on HEAD, so another slice's commit does not force a re-gate and a change
# to anything the build reads does.
STAMP="$(for d in ffi/poc/python ffi/poc/codec ffi/schema ffi/corpus; do git rev-parse "$SHA:$d"; done | sha256sum | cut -c1-16)"
TAG=$("$PY" -c 'import sys;print("py%d.%d"%sys.version_info[:2])')
if [ -n "$SMOKE" ]; then
  LAUNCHES=1; ROUNDS=1; TARGET_MS=2; CALLS=16; CITERS=200000
else
  TARGET_MS=50; CALLS=400; CITERS=2000000
  if [ "$SUITE" != gate ]; then
    [ -n "${AK_CPU_CLIENT:-}" ] && [ -n "${AK_CPU_SERVER:-}" ] || { echo "AK_CPU_CLIENT and AK_CPU_SERVER are required"; exit 2; }
  fi
fi
echo "# python campaign runner: suite $SUITE, out $OUT, snapshot $SHA, launches $LAUNCHES, rounds $ROUNDS ${SMOKE:+(SMOKE: instrumentation only)}"

need_gate() {
  if [ "$(cat "$OUT/gate.ok" 2>/dev/null)" != "$STAMP" ]; then
    echo "   no gate.ok for this commit in $OUT: running the gate first (requirement 26)"
    "$0" --suite gate --out "$OUT" $SMOKE $DIRTY
  fi
}

case "$SUITE" in
  gate)
    rm -f "$OUT/gate.ok"
    AK_GATE_LOGS="$OUT/gate" ./gate.sh "$PY" $FLOOR
    echo "== the RPC runner's must-fail control: a server that returns one short body in 50 =="
    if AK_CAMP_PLANT=short "$PY" camp_rpc.py --rounds 1 --calls 16 --transports shipped \
         --out "$OUT/gate/rpc-control.jsonl" --allow-dirty --smoke > "$OUT/gate/rpc-control.out" 2>&1; then
      echo "   CONTROL PASSED: the RPC runner did not abort on a short body"; exit 1
    fi
    grep -q '^# ABORTED, NO FIGURE' "$OUT/gate/rpc-control.jsonl" && [ "$(grep -c '^{' "$OUT/gate/rpc-control.jsonl" || true)" -eq 0 ] \
      || { echo "   CONTROL: aborted but still wrote samples"; exit 1; }
    echo "   failed as required, no sample written: $(grep ABORTED "$OUT/gate/rpc-control.jsonl")"
    echo "$STAMP" > "$OUT/gate.ok"
    echo "GATE PASSED"
    ;;
  codec)
    need_gate
    mkdir -p "build/$TAG/pb2corpus"
    (cd ../../corpus/generated && "$PY" -W ignore -m grpc_tools.protoc -I. --python_out="$HERE/build/$TAG/pb2corpus" corpus.proto)
    for l in $(seq 1 "$LAUNCHES"); do
      for fam in shapes unknown; do
        "$PY" camp_codec.py --family $fam --launch "$l" --rounds "$ROUNDS" --target-ms "$TARGET_MS" \
          --out "$OUT/codec-$fam-launch$l.jsonl" $SMOKE $DIRTY
        echo "   codec $fam launch $l: $(grep -c '^{' "$OUT/codec-$fam-launch$l.jsonl" || true) samples"
      done
    done
    "$PY" camp_summary.py "$OUT" > "$OUT/summary.txt"
    ;;
  rpc)
    need_gate
    for l in $(seq 1 "$LAUNCHES"); do
      "$PY" camp_rpc.py --launch "$l" --rounds "$ROUNDS" --calls "$CALLS" --out "$OUT/rpc-launch$l.jsonl" $SMOKE $DIRTY 2>"$OUT/rpc-launch$l.stderr"
      echo "   rpc launch $l: $(grep -c '^{' "$OUT/rpc-launch$l.jsonl" || true) samples"
    done
    "$PY" camp_summary.py "$OUT" > "$OUT/summary.txt"
    ;;
  calib)
    for l in $(seq 1 "$LAUNCHES"); do
      R=""; [ "$l" = 1 ] && R="--rust-log $OUT/calib-rust-crossing.log"
      "$PY" camp_calib.py --launch "$l" --rounds "$ROUNDS" --iters "$CITERS" --out "$OUT/calib-launch$l.jsonl" $R $SMOKE $DIRTY
      echo "   calib launch $l: $(grep -c '^{' "$OUT/calib-launch$l.jsonl" || true) samples"
    done
    "$PY" camp_summary.py "$OUT" > "$OUT/summary.txt"
    ;;
  *) echo "unknown suite $SUITE"; exit 2;;
esac
