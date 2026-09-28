#!/usr/bin/env bash
# The queue deliveries' checks (req. 16 as amended 2026-09-28; no timing):
#   gen/q_checks.sh LOG [ASAN_BUILD_DIR]
# One serve.sh server (the shared one; its test paths StatusS<n>, SleepS, StallS, EchoS,
# StatusU<n>). Then, for the full and the no-unknown client:
#   1. campaign_rpc --semantics 1: the queue forms of client streaming and ak_call_unary_enc_q
#      (status, moved-encode sends to the checking path, cancel a pending recv, cancel a pending
#      send, misuse, a NULL queue, the queue drained to AK_QUEUE_SHUTDOWN), both send paths;
#   2. plants on queue cells: each must abort (exit 3) with no sample file;
#   3. every queue cell, every direction, k = 1 and 8, one repetition: every call checked;
#   4. the counting clients against logs/cpp/rpc-counts*.log (rows identical);
#   5. with ASAN_BUILD_DIR (campaign_rpc(_nounk) built with -fsanitize=address): 1 and 3 under
#      ASan+LSan, 0 reports required.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
LOG=${1:?usage: gen/q_checks.sh LOG [ASAN_BUILD_DIR]}; ASAN=${2:-}
FFI=$(cd "$HERE/../.." && pwd); B=${BUILD:-$HERE/build-campaign}; SERVE=$FFI/poc/rust/serve.sh
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1} AK_CPU_SERVER=${AK_CPU_SERVER:-2,3}
SCRATCH=$(mktemp -d); export AK_SERVE_STATE=$SCRATCH/serve.state
cleanup() { [ -f "$AK_SERVE_STATE" ] && bash "$SERVE" stop > /dev/null 2>&1; rm -rf "$SCRATCH"; true; }
trap cleanup EXIT
FAILS=0
ok() { echo ">>> ok: $*"; }
bad() { echo ">>> FAIL: $*"; FAILS=$((FAILS + 1)); }
{
  echo "# cpp slice: queue-delivery checks (gen/q_checks.sh), commit $(git -C "$FFI" rev-parse --short HEAD)$(git -C "$FFI" diff --quiet HEAD -- poc/cpp/src || echo ' + UNCOMMITTED CHANGES'), $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "#   machine $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); client CPU $AK_CPU_CLIENT, server CPUs $AK_CPU_SERVER"
  bash "$SERVE" start --out "$SCRATCH/srv" > "$SCRATCH/srv.out" 2>&1 || { cat "$SCRATCH/srv.out"; echo ">>> FAIL: server"; exit 1; }
  SOCK=unix:$(awk '$1=="shipped"{print $2}' "$SCRATCH/srv.out")
  EXP=$(sed -n 's/.*P2.2 \([0-9]*\) B.*/\1/p' "$SCRATCH/srv/rpc-server.log" | head -1)
  echo "#   server  $(head -1 "$SCRATCH/srv/rpc-server.log")"
  cells() { [ "$1" = full ] && echo B-q,Bf-q,C-q-drop,C-q-retain,Cf-q-drop,Cf-q-retain,E-q-drop,E-q-retain,Ef-q-drop,Ef-q-retain \
                            || echo B-q,Bf-q,C-q-nounk,Cf-q-nounk,E-q-nounk,Ef-q-nounk; }
  run() { taskset -c "$AK_CPU_CLIENT" "$@" > "$SCRATCH/o.log" 2>&1; }
  for v in full nounk; do
    exe=campaign_rpc; [ $v = nounk ] && exe=campaign_rpc_nounk
    C=$(cells $v); q1=${C%%,*}
    echo "===== $v client ($exe) ====="
    echo "-- 1. semantics"
    run "$B/$exe" --target "$SOCK" --expect "$EXP" --transport shipped --semantics 1; rc=$?
    sed 's/^/   /' "$SCRATCH/o.log"
    [ $rc = 0 ] && grep -q '"failed": 0' "$SCRATCH/o.log" && ok "semantics ($v)" || bad "semantics ($v) rc=$rc"
    echo "-- 2. plants on queue cells (each must abort, exit 3, no sample file)"
    CQ=C-q-drop; EQ=E-q-drop; [ $v = nounk ] && { CQ=C-q-nounk; EQ=Ef-q-nounk; }
    for pl in "d-sha $CQ d" "d-count $EQ d" "c-len B-q c" "c-len Cf-q-${CQ##*-q-} c"; do
      set -- $pl
      rm -f "$SCRATCH/g.json"
      run "$B/$exe" --target "$SOCK" --expect "$EXP" --transport shipped --cells "$2" --dirs "$3" --plant "$1" \
          --launch 1 --rounds 1 --min-time-s 0.001 --warmup-s 0 --inflight 1,8 --gbench-out "$SCRATCH/g.json"; rc=$?
      if [ $rc = 3 ] && [ ! -e "$SCRATCH/g.json" ]; then ok "plant $1 on $2 ($3): aborted, $(grep -m1 'CALL CHECK' "$SCRATCH/o.log")"
      else bad "plant $1 on $2 ($3): rc=$rc"; fi
    done
    echo "-- 3. every queue cell, every direction, k = 1 and 8, one repetition (every call checked)"
    rm -f "$SCRATCH/g.json"
    run "$B/$exe" --target "$SOCK" --expect "$EXP" --transport shipped --cells "$C" --launch 1 --rounds 1 \
        --min-time-s 0.001 --warmup-s 0 --inflight 1,8 --gbench-out "$SCRATCH/g.json"; rc=$?
    [ $rc = 0 ] && [ -s "$SCRATCH/g.json" ] && ok "grid smoke ($v): $(grep -o '"benchmarks": [0-9]*' "$SCRATCH/o.log")" || { tail -3 "$SCRATCH/o.log"; bad "grid smoke ($v) rc=$rc"; }
    echo "-- 4. crossing counts"
    cexe=campaign_rpc_count; want=$FFI/logs/cpp/rpc-counts.log; [ $v = nounk ] && { cexe=campaign_rpc_count_nounk; want=$FFI/logs/cpp/rpc-counts-nounk.log; }
    run "$B/$cexe" --target "$SOCK" --expect "$EXP" --transport shipped --count 4
    if diff <(grep -E '^  [BCDE]' "$want") <(grep -E '^  [BCDE]' "$SCRATCH/o.log") > "$SCRATCH/d"; then
      ok "$(grep -cE '^  [BCDE]' "$SCRATCH/o.log") RPC count rows identical to logs/cpp/$(basename "$want") ($(grep -E '^  [BCE][f]?-q' "$SCRATCH/o.log" | wc -l) queue rows)"
      grep -E '^  [BCE]f?-q' "$SCRATCH/o.log"
    else cat "$SCRATCH/d"; bad "RPC counts ($v) differ"; fi
    if [ -n "$ASAN" ]; then
      echo "-- 5. ASan + LSan ($ASAN/$exe)"
      ASAN_OPTIONS=detect_leaks=1 run "$ASAN/$exe" --target "$SOCK" --expect "$EXP" --transport shipped --semantics 1; rc=$?
      n=$(grep -c 'ERROR: \(Address\|Leak\)Sanitizer' "$SCRATCH/o.log")
      [ $rc = 0 ] && [ "$n" = 0 ] && ok "semantics under ASan ($v): $(tail -1 "$SCRATCH/o.log")" || { grep -A8 Sanitizer "$SCRATCH/o.log" | head -20; bad "semantics under ASan ($v): rc=$rc, $n reports"; }
      rm -f "$SCRATCH/g.json"
      ASAN_OPTIONS=detect_leaks=1 run "$ASAN/$exe" --target "$SOCK" --expect "$EXP" --transport shipped --cells "$C" \
          --launch 1 --rounds 1 --min-time-s 0.001 --warmup-s 0 --inflight 1,8 --gbench-out "$SCRATCH/g.json"; rc=$?
      n=$(grep -c 'ERROR: \(Address\|Leak\)Sanitizer' "$SCRATCH/o.log")
      [ $rc = 0 ] && [ "$n" = 0 ] && ok "queue-cell grid under ASan ($v): $(grep -o '"benchmarks": [0-9]*' "$SCRATCH/o.log"), 0 reports" || { grep -A8 Sanitizer "$SCRATCH/o.log" | head -20; bad "grid under ASan ($v): rc=$rc, $n reports"; }
    fi
  done
  echo "q_checks: $FAILS failure(s)"
} > "$LOG" 2>&1
grep -q '>>> FAIL' "$LOG" && exit 1 || exit 0
