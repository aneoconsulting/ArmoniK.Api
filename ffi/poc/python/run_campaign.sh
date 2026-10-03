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
# poc/rust too since WP10: the RPC server is the Rust slice's rpc_server, built from the snapshot.
STAMP="$(for d in ffi/poc/python ffi/poc/codec ffi/poc/rust ffi/schema ffi/corpus packages/rust; do git rev-parse "$SHA:$d"; done | sha256sum | cut -c1-16)"
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

# pyperf, the framework of the codec suite and the RPC grid (and of the gate's RPC controls)
need_pyperf() {
  [ -d build/pyperf/pyperf ] || "$PY" -m pip install -q --target build/pyperf "pyperf==2.10.0" 2>/dev/null
  [ -d build/pyperf/pyperf ] || { echo "   pyperf 2.10.0 could not be installed into build/pyperf"; exit 1; }
}
# FIX-PLAN WP10: THE RPC server is the Rust slice's rpc_server (poc/rust/SERVER.md), started
# through the snapshot's poc/rust/serve.sh (built by build.sh, gate step 90). Its state file is
# private to this runner, so another slice's serve.sh start does not collide with it.
export AK_SERVE_SH="$AK_SNAPSHOT_DIR/ffi/poc/rust/serve.sh"
export AK_SERVE_STATE="$HERE/build/serve-state-$$"
# WP13. D14: every pool sized to AK_WORKERS (campaign.machine: 8): the server's tokio runtime,
# the core runtime (camp_rpc.py), grpc-core (the sysconf shim below). D10: the server also
# listens on TCP 127.0.0.1 (AK_SERVER_TCP=0, any free port, pinned server configuration), and
# every timed cell dials it. D11: both h2 variants of the rpc core, AK_CAMPAIGN_H2 (default both).
if [ -z "${AK_WORKERS:-}" ] && [ -f "$HERE/../../campaign.machine" ]; then AK_WORKERS=$(. "$HERE/../../campaign.machine"; echo "$AK_WORKERS"); fi
export AK_WORKERS="${AK_WORKERS:-8}"
export AK_SERVER_THREADS="${AK_SERVER_THREADS:-$AK_WORKERS}" AK_SERVER_TCP=0
H2S="${AK_CAMPAIGN_H2:-stock h2-batch}"
# D9 as amended (owner, 2026-10-03): the RPC grid's main figures run with glibc's DEFAULT
# allocator (no GLIBC_TUNABLES, no mallopt), as production does. AK_CAMPAIGN_ALLOC=default|pinned
# (owner, default `default`) picks the mode of a run; `pinned` is the labelled diagnostic under
# D9's tunables, files suffixed -allocpinned, samples `allocator: pinned`, the minor faults per
# call beside every sample. Every measured process reads its allocator back at startup (a 16 MiB
# malloc, mallinfo2) and refuses to run if it disagrees (camp_meas.alloc_check).
export AK_CAMPAIGN_ALLOC="${AK_CAMPAIGN_ALLOC:-default}"
case "$AK_CAMPAIGN_ALLOC" in default|pinned) ;; *) echo "AK_CAMPAIGN_ALLOC must be default or pinned"; exit 2;; esac
ALLOCS="$AK_CAMPAIGN_ALLOC"
# CAMPAIGN section 4.0 (D18, owner 2026-10-03): AK_CAMPAIGN_GRID=core (default) runs the campaign
# grid, about 1 h; `full` runs every row of 4.1 and 4.2 (the rest are labelled extras). Under
# `core`: codec families shapes and unknown (7 named U-* rows), both builds, default allocator;
# RPC full build, the `shipped` client configuration only, stock h2 plus Cf on h2-batch for c and
# d, and the pinned allocator pass on its subset (A and Cf on c and d at k = 1), which the
# drivers select (camp_pyperf.core_filter, camp_rpc_pyperf.core_keep).
export AK_CAMPAIGN_GRID="${AK_CAMPAIGN_GRID:-core}"
case "$AK_CAMPAIGN_GRID" in core|full) ;; *) echo "AK_CAMPAIGN_GRID must be core or full"; exit 2;; esac
[ "$AK_CAMPAIGN_GRID" = core ] && ALLOCS="default pinned"
D9_TUNABLES="glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432"

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
    need_pyperf
    # Req 18 on the framework (WP9) and on the shared server (WP10): each planted fault must
    # abort the RPC run with no sample. The server is shared, so the plant is selected on the
    # client (SERVER.md): `short` calls FetchShort, `count` expects one byte more from
    # UploadStream, `digest` a wrong digest from UploadStreamCheck, each from a cell's 2nd call.
    # camp_rpc_pyperf.py without --server starts its own through serve.sh and warms it with one
    # call per direction; with --only it runs the one named benchmark, whose worker's checks
    # must meet the fault (short and count: in pyperf's timed loop; digest: the setup's check).
    rpc_control() {  # rpc_control <plant> <group> <benchmark> <grep for the reason>
      local P=$1 G=$2 B=$3 WHY=$4 F="$OUT/gate/rpc-control-$1"
      rm -rf "$F".*
      # the controls' benchmarks belong to the full grid (direction a, the shipped configuration)
      if AK_CAMPAIGN_GRID=full AK_CAMP_PLANT="$P" PYTHONPATH="$HERE/build/pyperf" "$PY" camp_rpc_pyperf.py \
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
    echo "== the RPC runner's must-fail control: FetchShort (P2.2 one byte short) from a cell's 2nd call =="
    rpc_control short ab "rpc|full|shipped|a|P2.2|B|1" "a call failed: .*want 540422"
    echo "== the RPC runner's must-fail control: (d) expects one byte more than UploadStream counts =="
    rpc_control count d "rpc|full|shipped|d|4MiB|C-drop|1" "a call failed: .*want 4194305"
    echo "== the RPC runner's must-fail control: (d) expects a wrong digest from UploadStreamCheck =="
    rpc_control digest d "rpc|full|shipped|d|4MiB|C-drop|1" "check: .*not the bytes and digest sent"
    # WP13: the RPC checks over TCP 127.0.0.1 for both h2 variants and both builds: every cell of
    # every direction on both client configurations against the shared server's TCP listener,
    # TCP_NODELAY read back on every live socket, and the write-count marker that shows which h2
    # is running (camp_rpc_pyperf.py --precheck).
    GS="$OUT/gate/rpc-tcp-server"; mkdir -p "$GS"
    SL=$(bash "$AK_SERVE_SH" start --out "$GS") || { echo "   serve.sh start failed: $(tail -2 "$GS/rpc-server.log")"; exit 1; }
    GSOCKS="shipped=unix:$(echo "$SL" | sed -n 's/^shipped //p'),pinned=unix:$(echo "$SL" | sed -n 's/^pinned //p'),tcp=$(echo "$SL" | sed -n 's/^tcp //p'),pid=$(echo "$SL" | sed -n 's/^pid //p')"
    bash "$AK_SERVE_SH" warm 1 > "$GS/warm.out" 2>&1 || { bash "$AK_SERVE_SH" stop >/dev/null; echo "   serve.sh warm failed"; exit 1; }
    for h in stock h2-batch; do
      for v in full nounk; do
        f="$OUT/gate/rpc-tcp-precheck-$v-$h.out"
        echo "== the RPC grid's checks over TCP 127.0.0.1: build $v, h2 $h =="
        if ! "$PY" camp_rpc_pyperf.py --precheck --variant "$v" --h2 "$h" --server "$GSOCKS" --transports shipped,pinned,armonik > "$f" 2>&1; then
          tail -3 "$f"; bash "$AK_SERVE_SH" stop >/dev/null; echo "   RPC TCP CHECK FAILED ($v, $h)"; exit 1
        fi
        grep "write syscalls\|TCP_NODELAY" "$f"
      done
    done
    bash "$AK_SERVE_SH" stop > /dev/null
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
    need_pyperf
    AFF="${AK_CPU_CLIENT:-$("$PY" -c 'import os;print(",".join(map(str,sorted(os.sched_getaffinity(0)))))')}"
    # Req 24 as amended: every warm-up is a runner parameter. pyperf's warm-up values per
    # worker (AK_CAMPAIGN_PYPERF_WARMUPS) and its loop-calibration target (AK_CAMPAIGN_PYPERF_MIN_TIME,
    # seconds); campaign defaults 3 and 0.1, smoke 1 and 0.002; both are in every codec header
    # (pyperf_args).
    if [ -n "$SMOKE" ]; then
      PP="--processes 1 --values 1 --warmups ${AK_CAMPAIGN_PYPERF_WARMUPS:-1} --min-time ${AK_CAMPAIGN_PYPERF_MIN_TIME:-0.002}"
      ONLY_UNKNOWN="--only U-root-all,U-nested-all,U-oneof-all,U-deep-all,U-enum-value-999"
      # the core grid already times only its 7 named U-* rows; the smoke subset applies to `full`
      [ "$AK_CAMPAIGN_GRID" = core ] && ONLY_UNKNOWN=""
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
      local l=$1 v=$2 fam O1 ONLY sfx="" AE
      [ "$v" = nounk ] && sfx="-nounk"
      # D9 as amended: the allocator mode of this run (AK_CAMPAIGN_ALLOC), as for the RPC grid
      if [ "$AK_CAMPAIGN_ALLOC" = pinned ]; then sfx="$sfx-allocpinned"; AE=(env GLIBC_TUNABLES="$D9_TUNABLES" AK_CAMPAIGN_ALLOC=pinned)
      else AE=(env -u GLIBC_TUNABLES AK_CAMPAIGN_ALLOC=default); fi
      # shapes; unknown = the 92 rows at the shapes core's roots (req 7); unknown-corpus = the
      # corpus-schema core, a labelled extra
      FAMS="shapes unknown unknown-corpus"; [ "$AK_CAMPAIGN_GRID" = core ] && FAMS="shapes unknown"
      [ "$AK_CAMPAIGN_GRID" = core ] && [ "$AK_CAMPAIGN_ALLOC" != default ] && { echo "   the core grid's codec suite runs the default allocator only"; exit 2; }
      for fam in $FAMS; do
        O1="$OUT/codec-$fam$sfx-launch$l"
        rm -rf "$O1.side" "$O1.pyperf.json"
        ONLY=""; [ $fam != shapes ] && ONLY="$ONLY_UNKNOWN"
        "${AE[@]}" PYTHONPATH="$HERE/build/pyperf" "$PY" camp_pyperf.py --family $fam --launch "$l" --variant "$v" $ONLY --side "$O1.side" \
          -o "$O1.pyperf.json" $PP --affinity "$AFF" --copy-env --quiet > "$O1.pyperf.out" 2>&1 \
          || { tail -20 "$O1.pyperf.out"; echo "   pyperf failed: codec $fam ($v) launch $l"; exit 1; }
        "${AE[@]}" "$PY" camp_pyperf_export.py --json "$O1.pyperf.json" --side "$O1.side" --launch "$l" --variant "$v" \
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
    need_pyperf
    [ -x "$AK_SERVE_SH" ] || { echo "   no $AK_SERVE_SH: the snapshot has no poc/rust (run the gate)"; exit 1; }
    # Req 13 as amended at 9f6d579fa (FIX-PLAN WP10): ONE server process per launch, the Rust
    # slice's tonic rpc_server (poc/rust/SERVER.md), started through the snapshot's serve.sh,
    # pinned by it to AK_CPU_SERVER, AK_SERVER_THREADS=AK_WORKERS tokio workers (D14); since
    # WP13 (D10) every timed cell dials its TCP listener on 127.0.0.1 (AK_SERVER_TCP=0, the
    # pinned server configuration, TCP_NODELAY on accept), whatever its client configuration;
    # its Unix sockets are left unused; serving every cell of both builds and both h2 variants;
    # warmed by `serve.sh warm AK_CAMPAIGN_SERVER_WARMUP` (checked calls per direction from a
    # tonic and a core client, both sockets) before any invocation.
    if [ -n "$SMOKE" ]; then export AK_CAMPAIGN_SERVER_WARMUP="${AK_CAMPAIGN_SERVER_WARMUP:-8}"
    else export AK_CAMPAIGN_SERVER_WARMUP="${AK_CAMPAIGN_SERVER_WARMUP:-64}"; fi
    # WP9 (req 22a as amended): the grid runs on pyperf (camp_rpc_pyperf.py). Per launch: the
    # launch's server, warmed; then, per build in an order alternated by launch, a precheck of
    # every cell (req 26), and one pyperf invocation per direction group (ab, c, d), each with
    # its own --loops (AK_CAMPAIGN_RPC_LOOPS_AB / _C / _D; campaign 25 / 8 / 3, smoke 2 / 1 / 1)
    # and pyperf's --warmups (AK_CAMPAIGN_RPC_WARMUPS; campaign 3, smoke 1). Any failure
    # discards the launch's output: no sample of an aborted launch is kept (req 18).
    if [ -n "$SMOKE" ]; then
      LAB=${AK_CAMPAIGN_RPC_LOOPS_AB:-2}; LC=${AK_CAMPAIGN_RPC_LOOPS_C:-1}; LD=${AK_CAMPAIGN_RPC_LOOPS_D:-1}; RW=${AK_CAMPAIGN_RPC_WARMUPS:-1}
    else
      LAB=${AK_CAMPAIGN_RPC_LOOPS_AB:-25}; LC=${AK_CAMPAIGN_RPC_LOOPS_C:-8}; LD=${AK_CAMPAIGN_RPC_LOOPS_D:-3}; RW=${AK_CAMPAIGN_RPC_WARMUPS:-3}
    fi
    # AK_CAMPAIGN_RPC_TRANSPORTS: the client transports timed (campaign: shipped,pinned, both;
    # a minimal smoke may name one). The server always serves both.
    TR=${AK_CAMPAIGN_RPC_TRANSPORTS:-shipped,pinned}
    # CAMPAIGN 4.0 as amended (b58543f7b): the core grid's one client configuration is `armonik`
    # (packages/python's create_channel for A; the core's own client configuration for the core
    # cells); `shipped` and `pinned` stay under AK_CAMPAIGN_GRID=full.
    [ "$AK_CAMPAIGN_GRID" = core ] && TR=${AK_CAMPAIGN_RPC_TRANSPORTS:-armonik}
    AFF="${AK_CPU_CLIENT:-$("$PY" -c 'import os;print(",".join(map(str,sorted(os.sched_getaffinity(0)))))')}"
    discard() {  # discard <launch> <why>
      rm -rf "$OUT"/rpc-launch"$1".* "$OUT"/rpc-nounk-launch"$1".* "$OUT"/rpc-*-launch"$1".*
      echo "# ABORTED, NO FIGURE: $2" > "$OUT/rpc-launch$1.ABORTED"
      echo "   launch $1 DISCARDED: $2"
    }
    rpc_run() {  # rpc_run <launch> <full|nounk> <server sockets> <server log> <h2> <default|pinned>; nonzero on any failure
      local l=$1 v=$2 S=$3 SLOG=$4 h=$5 al=$6 g L hs="" AE
      [ "$h" = h2-batch ] && hs="-h2batch"
      [ "$al" = pinned ] && hs="$hs-allocpinned"
      if [ "$al" = pinned ]; then AE=(env GLIBC_TUNABLES="$D9_TUNABLES" AK_CAMPAIGN_ALLOC=pinned); else AE=(env -u GLIBC_TUNABLES AK_CAMPAIGN_ALLOC=default); fi
      local n=0 ng
      for g in ab c d; do
        ng=$("${AE[@]}" AK_H2="$h" "$PY" camp_rpc_pyperf.py --count --variant "$v" --group $g --transports "$TR" 2>/dev/null | tail -1)
        n=$((n + ng))
      done
      [ "$n" -gt 0 ] || { echo "   rpc ($v, $h, $al) launch $l: no benchmark in this grid"; return 0; }
      "$PY" camp_rpc_pyperf.py --precheck --variant "$v" --h2 "$h" --server "$S" --transports "$TR" > "$OUT/rpc-$v$hs-precheck-launch$l.out" 2>&1 \
        || { tail -3 "$OUT/rpc-$v$hs-precheck-launch$l.out"; return 1; }
      for g in ab c d; do
        ng=$("${AE[@]}" AK_H2="$h" "$PY" camp_rpc_pyperf.py --count --variant "$v" --group $g --transports "$TR" 2>/dev/null | tail -1)
        [ "$ng" -gt 0 ] || continue
        case $g in ab) L=$LAB;; c) L=$LC;; d) L=$LD;; esac
        local F="$OUT/rpc-$g$hs-launch$l"; [ "$v" = nounk ] && F="$OUT/rpc-nounk-$g$hs-launch$l"
        # Req 24 as amended (8c02e7c58): in the campaign, every calling (pool) thread makes at
        # least 20 calls at the cell's payload before the first measured value. One warm-up value
        # is L batches, one call per pool thread each, so a group's warm-up count is raised to
        # ceil(20 / L) when AK_CAMPAIGN_RPC_WARMUPS gives fewer: with the defaults ab 3 x 25 = 75,
        # c 3 x 8 = 24, d 7 x 3 = 21 calls per thread (d raised from 3, which gave 9). The smoke
        # keeps its short warm-up (stated in the header).
        local W=$RW
        [ -z "$SMOKE" ] && [ $((W * L)) -lt 20 ] && W=$(( (20 + L - 1) / L ))
        local PP="--processes 1 --values $ROUNDS --warmups $W --loops $L"
        rm -rf "$F.side" "$F.pyperf.json"
        # grpc-core sized to AK_WORKERS through the sysconf shim (D14), preloaded into the pyperf
        # processes (--copy-env carries it into every worker)
        "${AE[@]}" AK_H2="$h" AK_SHIM_NCPUS="$AK_WORKERS" LD_PRELOAD="$HERE/build/ncpus_shim.so" \
        PYTHONPATH="$HERE/build/pyperf" "$PY" camp_rpc_pyperf.py --variant "$v" --group $g --launch "$l" --server "$S" \
          --transports "$TR" --side "$F.side" -o "$F.pyperf.json" $PP --affinity "$AFF" --copy-env --quiet > "$F.pyperf.out" 2>&1 \
          || { tail -5 "$F.pyperf.out"; return 1; }
        "${AE[@]}" AK_SHIM_NCPUS="$AK_WORKERS" "$PY" camp_rpc_pyperf_export.py --json "$F.pyperf.json" --side "$F.side" --launch "$l" --variant "$v" --group $g --h2 "$h" --alloc "$al" \
          --server "$S" --server-log "$SLOG" --pyperf-args "$PP --affinity $AFF --copy-env --transports $TR" --out "$F.jsonl" $SMOKE $DIRTY || return 1
        rm -rf "$F.side"
        echo "   rpc ($v, $h, $al, $g) launch $l: $(grep -c '"phase": "value"' "$F.jsonl" || true) values, $(grep -c '^{' "$F.jsonl" || true) raw measurements"
      done
    }
    for l in $(seq 1 "$LAUNCHES"); do
      SL="$OUT/rpc-server-launch$l"; mkdir -p "$SL"
      SLINE=$(bash "$AK_SERVE_SH" start --out "$SL") || { echo "   serve.sh start failed: $(tail -2 "$SL/rpc-server.log")"; exit 1; }
      SOCKS="shipped=unix:$(echo "$SLINE" | sed -n 's/^shipped //p'),pinned=unix:$(echo "$SLINE" | sed -n 's/^pinned //p'),tcp=$(echo "$SLINE" | sed -n 's/^tcp //p'),pid=$(echo "$SLINE" | sed -n 's/^pid //p')"
      echo "   launch $l server: the Rust rpc_server, $(echo "$SLINE" | tr '\n' ' '), AK_CPU_SERVER=${AK_CPU_SERVER:-unset}, AK_SERVER_THREADS=${AK_SERVER_THREADS:-4}"
      OK=1
      bash "$AK_SERVE_SH" warm "$AK_CAMPAIGN_SERVER_WARMUP" > "$SL/warm.out" 2>&1 || OK=0
      echo "   server warm-up: serve.sh warm $AK_CAMPAIGN_SERVER_WARMUP, $( [ $OK = 1 ] && echo passed || echo FAILED: $(tail -1 "$SL/warm.out"))"
      if [ $OK = 1 ]; then
        if [ $((l % 2)) = 1 ]; then ORD="full nounk"; else ORD="nounk full"; fi
        # AK_CAMPAIGN_RPC_BUILDS: the builds timed (campaign: both; a minimal smoke may name one)
        [ "$AK_CAMPAIGN_GRID" = core ] && ORD="full"
        [ -n "${AK_CAMPAIGN_RPC_BUILDS:-}" ] && ORD="$AK_CAMPAIGN_RPC_BUILDS"
        for v in $ORD; do
          for al in $ALLOCS; do
            for h in $H2S; do rpc_run "$l" "$v" "$SOCKS" "$SL/rpc-server.log" "$h" "$al" || { OK=0; break 3; }; done
          done
        done
      fi
      bash "$AK_SERVE_SH" stop > /dev/null
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
