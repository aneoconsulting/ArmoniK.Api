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
    # Req 18 on the framework (WP9): each planted fault must abort the RPC run with no sample.
    # camp_rpc_pyperf.py without --server starts and warms (one call per transport) its own
    # server; with --only it runs the one named benchmark, whose worker's per-call checks must
    # meet the fault inside pyperf's loop (64 batches, so the short body, 1 in 50, is reached).
    rpc_control() {  # rpc_control <plant> <group> <benchmark> <grep for the reason>
      local P=$1 G=$2 B=$3 WHY=$4 F="$OUT/gate/rpc-control-$1"
      rm -rf "$F".*
      if AK_CAMP_PLANT="$P" AK_CAMPAIGN_SERVER_WARMUP=1 PYTHONPATH="$HERE/build/pyperf" "$PY" camp_rpc_pyperf.py \
           --variant full --group "$G" --launch 1 --side "$F.side" --transports shipped \
           --only "$B" -o "$F.json" --processes 1 --values 1 --warmups 1 --loops 64 \
           --quiet > "$F.out" 2>&1; then
        echo "   CONTROL PASSED: the RPC run did not abort on the planted $P"; exit 1
      fi
      [ ! -s "$F.json" ] || { echo "   CONTROL: the $P run aborted but wrote pyperf output"; exit 1; }
      grep -q "^benchmark $B: .*$WHY" "$F.out" || { echo "   CONTROL: the $P run failed for another reason: $(tail -2 "$F.out")"; exit 1; }
      rm -rf "$F.side"
      echo "   failed as required, in the benchmark's checks, no sample written: $(grep -m1 "$WHY" "$F.out" | cut -c1-160)"
    }
    echo "== the RPC runner's must-fail control: a server that returns one short body in 50 =="
    rpc_control short ab "rpc|full|shipped|a|P2.2|B|1" "want 540422"
    echo "== the RPC runner's must-fail control: a server that answers (d) with a wrong digest =="
    rpc_control digest d "rpc|full|shipped|d|4MiB|C-drop|1" "digest"
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
    # Req 24 as amended: every warm-up is a runner parameter. pyperf's warm-up values per
    # worker (AK_CAMPAIGN_PYPERF_WARMUPS) and its loop-calibration target (AK_CAMPAIGN_PYPERF_MIN_TIME,
    # seconds); campaign defaults 3 and 0.1, smoke 1 and 0.002; both are in every codec header
    # (pyperf_args).
    if [ -n "$SMOKE" ]; then
      PP="--processes 1 --values 1 --warmups ${AK_CAMPAIGN_PYPERF_WARMUPS:-1} --min-time ${AK_CAMPAIGN_PYPERF_MIN_TIME:-0.002}"
      ONLY_UNKNOWN="--only U-root-all,U-nested-all,U-oneof-all,U-deep-all,U-enum-value-999"
      # A smoke builds each beyond-cache pool at 1 MiB of wire bytes (stated in the header);
      # the campaign uses the default, 13.75 MiB (req 11).
      export AK_POOL_BYTES="${AK_POOL_BYTES:-1048576}"
    else
      PP="--processes 1 --values $ROUNDS --warmups ${AK_CAMPAIGN_PYPERF_WARMUPS:-3} --min-time ${AK_CAMPAIGN_PYPERF_MIN_TIME:-0.1}"
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
    if [ -n "$SMOKE" ]; then export AK_CAMPAIGN_SERVER_WARMUP="${AK_CAMPAIGN_SERVER_WARMUP:-8}"
    else export AK_CAMPAIGN_SERVER_WARMUP="${AK_CAMPAIGN_SERVER_WARMUP:-64}"; fi
    # WP9 (req 22a as amended): the grid runs on pyperf (camp_rpc_pyperf.py). Per launch: the
    # launch's ONE server (camp_server.py, AK_CPU_SERVER, both transports on two Unix sockets,
    # req 13), warmed with AK_CAMPAIGN_SERVER_WARMUP calls from each client transport (req 13,
    # 24); then, per build in an order alternated by launch, a precheck of every cell (req 26),
    # and one pyperf invocation per direction group (ab, c, d), each with its own --loops
    # (AK_CAMPAIGN_RPC_LOOPS_AB / _C / _D; campaign 25 / 8 / 3, smoke 2 / 1 / 1) and pyperf's
    # --warmups (AK_CAMPAIGN_RPC_WARMUPS; campaign 3, smoke 1). Any failure discards the
    # launch's output: no sample of an aborted launch is kept (req 18).
    if [ -n "$SMOKE" ]; then
      LAB=${AK_CAMPAIGN_RPC_LOOPS_AB:-2}; LC=${AK_CAMPAIGN_RPC_LOOPS_C:-1}; LD=${AK_CAMPAIGN_RPC_LOOPS_D:-1}; RW=${AK_CAMPAIGN_RPC_WARMUPS:-1}
    else
      LAB=${AK_CAMPAIGN_RPC_LOOPS_AB:-25}; LC=${AK_CAMPAIGN_RPC_LOOPS_C:-8}; LD=${AK_CAMPAIGN_RPC_LOOPS_D:-3}; RW=${AK_CAMPAIGN_RPC_WARMUPS:-3}
    fi
    AFF="${AK_CPU_CLIENT:-$("$PY" -c 'import os;print(",".join(map(str,sorted(os.sched_getaffinity(0)))))')}"
    discard() {  # discard <launch> <why>
      rm -rf "$OUT"/rpc-launch"$1".* "$OUT"/rpc-nounk-launch"$1".* "$OUT"/rpc-*-launch"$1".*
      echo "# ABORTED, NO FIGURE: $2" > "$OUT/rpc-launch$1.ABORTED"
      echo "   launch $1 DISCARDED: $2"
    }
    rpc_run() {  # rpc_run <launch> <full|nounk> <server sockets>; nonzero on any failure
      local l=$1 v=$2 S=$3 g L
      "$PY" camp_rpc_pyperf.py --precheck --variant "$v" --server "$S" > "$OUT/rpc-$v-precheck-launch$l.out" 2>&1 \
        || { tail -3 "$OUT/rpc-$v-precheck-launch$l.out"; return 1; }
      for g in ab c d; do
        case $g in ab) L=$LAB;; c) L=$LC;; d) L=$LD;; esac
        local F="$OUT/rpc-$g-launch$l"; [ "$v" = nounk ] && F="$OUT/rpc-nounk-$g-launch$l"
        local PP="--processes 1 --values $ROUNDS --warmups $RW --loops $L"
        rm -rf "$F.side" "$F.pyperf.json"
        PYTHONPATH="$HERE/build/pyperf" "$PY" camp_rpc_pyperf.py --variant "$v" --group $g --launch "$l" --server "$S" \
          --side "$F.side" -o "$F.pyperf.json" $PP --affinity "$AFF" --copy-env --quiet > "$F.pyperf.out" 2>&1 \
          || { tail -5 "$F.pyperf.out"; return 1; }
        "$PY" camp_rpc_pyperf_export.py --json "$F.pyperf.json" --side "$F.side" --launch "$l" --variant "$v" --group $g \
          --server "$S" --pyperf-args "$PP --affinity $AFF --copy-env" --out "$F.jsonl" $SMOKE $DIRTY || return 1
        rm -rf "$F.side"
        echo "   rpc ($v, $g) launch $l: $(grep -c '"phase": "value"' "$F.jsonl" || true) values, $(grep -c '^{' "$F.jsonl" || true) raw measurements"
      done
    }
    for l in $(seq 1 "$LAUNCHES"); do
      SD=$(mktemp -d /tmp/akrpc-srv.XXXXXX)
      coproc SRV { exec "$PY" camp_server.py --dir "$SD"; }
      read -r SLINE <&"${SRV[0]}"
      case "$SLINE" in SOCKETS*) ;; *) echo "   the server did not start: $SLINE"; exit 1;; esac
      SOCKS=$(echo "$SLINE" | tr ' ' '\n' | grep '=unix:' | paste -sd, -)
      echo "   launch $l server: pid $SRV_PID, $SLINE"
      OK=1
      "$PY" camp_rpc_pyperf.py --warm-server --server "$SOCKS" || OK=0
      if [ $OK = 1 ]; then
        if [ $((l % 2)) = 1 ]; then ORD="full nounk"; else ORD="nounk full"; fi
        for v in $ORD; do rpc_run "$l" "$v" "$SOCKS" || { OK=0; break; }; done
      fi
      SPID=$SRV_PID
      eval "exec ${SRV[1]}>&-"
      wait "$SPID" || true
      rm -rf "$SD"
      [ $OK = 1 ] || { discard "$l" "a benchmark, the precheck or the server warm-up failed"; exit 1; }
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
