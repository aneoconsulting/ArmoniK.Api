#!/usr/bin/env bash
# The python slice's campaign runner (design/CAMPAIGN.md requirement 31).
#
#   ./run_campaign.sh --suite codec|rpc|calib|gate --out <dir>   (CAMPAIGN.md 29: --out ffi/logs/python/campaign) [--launches 3] [--rounds 5]
#                     [--smoke] [--allow-dirty]
#
# Environment: AK_CPU_CLIENT, AK_CPU_SERVER (from ffi/campaign.machine via ffi/campaign.sh, or
#   read here from that file when unset; required for codec/rpc/calib unless --smoke),
#   AK_ISOLATION (the isolation mechanism, printed in every header), AK_PY (target, default
#   python3.12), AK_FLOOR_PY (floor, default build/py37/python3.7 when ./fetch_py37.sh ran),
#   AK_SNAPSHOT (commit to build the core from, default HEAD: a `git archive`, never the
#   working tree, so the run names a commit even while poc/codec is being edited).
#
# gate   build (every core WITH init-guard), then gate.sh at the target AND the floor
#        (payload-set byte identity on every arm, the whole corpus on every codec arm in both
#        unknown-field modes where built, the planted controls), plus the RPC runner's
#        must-fail control. Writes <out>/gate.ok on success.
# codec  (refuses without a gate.ok for this commit) the codec suite, families shapes,
#        unknown (the 92 U-* rows at the shapes core's roots) and unknown-corpus (extra), and
#        one pyperf invocation per family per build per launch
# rpc    (refuses without gate.ok) the RPC grid, one client process per launch, the server
#        in its own pinned process
# calib  (refuses without gate.ok) crossing counts (a gate: a difference stops it) and
#        crossing cost, host and rust
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
    # Req 4 as amended (R-H34): the CPU sets come from ffi/campaign.machine, which
    # ffi/campaign.sh sources and exports (and checks: sizes 4 and 4, SMT siblings). Run on
    # its own, this runner reads the same file; values already in the environment win.
    M="$HERE/../../campaign.machine"
    if [ -f "$M" ] && { [ -z "${AK_CPU_CLIENT:-}" ] || [ -z "${AK_CPU_SERVER:-}" ]; }; then . "$M"; export AK_CPU_CLIENT AK_CPU_SERVER; fi
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
    # CAMPAIGN.md 22a: the codec suite on pyperf (camp_pyperf.py). One pyperf invocation per
    # launch (order rotated by launch), `--processes 1 --values ROUNDS`: a fresh worker per
    # benchmark per launch, pinned by --affinity to AK_CPU_CLIENT; pyperf's warm-up and loop
    # calibration; every raw value exported to section 7 by camp_pyperf_export.py.
    need_gate
    mkdir -p "build/$TAG/pb2corpus"
    (cd ../../corpus/generated && "$PY" -W ignore -m grpc_tools.protoc -I. --python_out="$HERE/build/$TAG/pb2corpus" corpus.proto)
    [ -d build/pyperf/pyperf ] || "$PY" -m pip install -q --target build/pyperf "pyperf==2.10.0" 2>/dev/null
    AFF="${AK_CPU_CLIENT:-$("$PY" -c 'import os;print(",".join(map(str,sorted(os.sched_getaffinity(0)))))')}"
    if [ -n "$SMOKE" ]; then
      PP="--processes 1 --values 1 --warmups 1 --min-time 0.002"
      ONLY_UNKNOWN="--only U-root-all,U-nested-all,U-oneof-all,U-deep-all,U-enum-value-999"
      # A smoke builds each beyond-cache pool at 1 MiB of wire bytes (stated in the header);
      # the campaign uses the default, 13.75 MiB (req 11).
      export AK_POOL_BYTES="${AK_POOL_BYTES:-1048576}"
    else
      PP="--processes 1 --values $ROUNDS --warmups 3 --min-time 0.1"
      ONLY_UNKNOWN=""
    fi
    # WP5 step 10: the full build (drop, retain) and the no-unknown build (a separately built
    # extension over ak-core without unknown-fields) are separate pyperf invocations, in an
    # order alternated by launch; each invocation times the incumbent too. pyperf runs EVERY
    # benchmark in its own worker process, so no arm shares a process with its baseline: the
    # incumbent is a same-launch control, not an in-process one (R-H19).
    codec_run() {  # codec_run <launch> <full|nounk>
      local l=$1 v=$2 fam O1 ONLY sfx=""
      [ "$v" = nounk ] && sfx="-nounk"
      # shapes; unknown = the 92 rows at the shapes core's roots (req 7); unknown-corpus = the
      # corpus-schema core, a labelled extra
      for fam in shapes unknown unknown-corpus; do
        O1="$OUT/codec-$fam$sfx-launch$l"
        rm -rf "$O1.side" "$O1.pyperf.json"
        ONLY=""; [ $fam != shapes ] && ONLY="$ONLY_UNKNOWN"
        PYTHONPATH="$HERE/build/pyperf" "$PY" camp_pyperf.py --family $fam --launch "$l" --variant "$v" $ONLY --side "$O1.side" \
          -o "$O1.pyperf.json" $PP --affinity "$AFF" --copy-env --quiet > "$O1.pyperf.out" 2>&1 \
          || { tail -20 "$O1.pyperf.out"; echo "   pyperf failed: codec $fam ($v) launch $l"; exit 1; }
        "$PY" camp_pyperf_export.py --json "$O1.pyperf.json" --side "$O1.side" --launch "$l" --variant "$v" \
          --pyperf-args "$PP --affinity $AFF --copy-env --variant $v $ONLY" --out "$O1.jsonl" $SMOKE $DIRTY
        rm -rf "$O1.side"
        echo "   codec $fam ($v) launch $l (pyperf): $(grep -c '"phase": "value"' "$O1.jsonl" || true) values, $(grep -c '^{' "$O1.jsonl" || true) raw measurements"
      done
    }
    for l in $(seq 1 "$LAUNCHES"); do
      if [ $((l % 2)) = 1 ]; then codec_run "$l" full; codec_run "$l" nounk
      else codec_run "$l" nounk; codec_run "$l" full; fi
    done
    "$PY" camp_summary.py "$OUT" > "$OUT/summary.txt"
    ;;
  rpc)
    need_gate
    # The full build's client (A, B, C and D in retain and drop, E and F in host-gen's drop
    # and retain, the labelled extras) and the no-unknown build's (A, B, C-nounk, D-nounk; A
    # and B share its client process), one process each, in an order alternated by launch.
    # Req 13 as amended (R-H33): ONE server process per launch (camp_server.py, pinned to
    # AK_CPU_SERVER, both transport configurations on two Unix sockets), serving every cell
    # of both builds; each client warms it from each of its transports before round 1.
    rpc_run() {  # rpc_run <launch> <full|nounk> <server sockets>
      local l=$1 v=$2 S=$3 F="$OUT/rpc-launch$1"
      [ "$v" = nounk ] && F="$OUT/rpc-nounk-launch$1"
      "$PY" camp_rpc.py --launch "$l" --rounds "$ROUNDS" --calls "$CALLS" --variant "$v" --server "$S" \
        --out "$F.jsonl" $SMOKE $DIRTY 2>"$F.stderr"
      echo "   rpc ($v) launch $l: $(grep -c '^{' "$F.jsonl" || true) samples$(grep -q '^# ABORTED' "$F.jsonl" && echo ", $(grep '^# ABORTED' "$F.jsonl")")"
    }
    for l in $(seq 1 "$LAUNCHES"); do
      SD=$(mktemp -d /tmp/akrpc-srv.XXXXXX)
      coproc SRV { exec "$PY" camp_server.py --dir "$SD"; }
      read -r SLINE <&"${SRV[0]}"
      case "$SLINE" in SOCKETS*) ;; *) echo "   the server did not start: $SLINE"; exit 1;; esac
      SOCKS=$(echo "$SLINE" | tr ' ' '\n' | grep '=unix:' | paste -sd, -)
      echo "   launch $l server: pid $SRV_PID, $SLINE"
      if [ $((l % 2)) = 1 ]; then rpc_run "$l" full "$SOCKS"; rpc_run "$l" nounk "$SOCKS"
      else rpc_run "$l" nounk "$SOCKS"; rpc_run "$l" full "$SOCKS"; fi
      SPID=$SRV_PID
      eval "exec ${SRV[1]}>&-"
      wait "$SPID" || true
      rm -rf "$SD"
    done
    "$PY" camp_summary.py "$OUT" > "$OUT/summary.txt"
    ;;
  calib)
    need_gate     # requirement 26: calib too refuses to time without a gate.ok (R-H19)
    for l in $(seq 1 "$LAUNCHES"); do
      R=""; [ "$l" = 1 ] && R="--rust-log $OUT/calib-rust-crossing.log"
      "$PY" camp_calib.py --launch "$l" --rounds "$ROUNDS" --iters "$CITERS" --out "$OUT/calib-launch$l.jsonl" $R $SMOKE $DIRTY
      echo "   calib launch $l: $(grep -c '^{' "$OUT/calib-launch$l.jsonl" || true) samples"
    done
    "$PY" camp_summary.py "$OUT" > "$OUT/summary.txt"
    ;;
  *) echo "unknown suite $SUITE"; exit 2;;
esac
