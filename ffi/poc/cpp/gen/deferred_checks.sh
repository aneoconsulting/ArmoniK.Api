#!/usr/bin/env bash
# Checks for the deferred-encode cells (Rust patch p5-deferred and patches on top of it) from C++,
# no timing. Under /tmp/ak-physical-bench.lock with an own 8-worker server.
#
#   gen/deferred_checks.sh LOG PATCH_TREE CORE_DIR CORE_NOUNK_DIR "KNOBS"
#
#   1. the patch tree's binaries (built against its cores): conformance full and no-unknown (byte
#      identity), the codec pre-check of both builds;
#   2. this tree's campaign_rpc(_nounk) with LD_LIBRARY_PATH on CORE_DIR (CORE_NOUNK_DIR):
#      --semantics 1 (the queue forms on both send paths, and the deferred send: both waits, both
#      payloads to the checking path, a failing encode, misuse);
#   3. the same on HEAD's core: the deferred cases are skipped, a Cf-enc cell is refused;
#   4. the grid with --check-stream 1 (every d call to UploadStreamCheck: the server's byte count and
#      SHA-256 of every message as received), cells A, D, Cf, Cf-enc, Cf-encp, Cf-q, d/4MiB and d/16MiB,
#      k = 1 and 8, one repetition, patched core.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
FFI=$(cd "$HERE/../.." && pwd)
LOGF=${1:?LOG}; PT=${2:?PATCH_TREE}; CD=${3:?CORE_DIR}; CDN=${4:?CORE_NOUNK_DIR}; KNOBS=${5:-}
if [ "${DC_LOCKED:-}" != 1 ]; then DC_LOCKED=1 exec flock /tmp/ak-physical-bench.lock bash "$0" "$@"; fi
mkdir -p "$(dirname "$LOGF")"
PB=$PT/ffi/poc/cpp/build-campaign
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1-4,11-14} AK_CPU_SERVER=${AK_CPU_SERVER:-5-8,15-18}
SERVE=$FFI/poc/rust/serve.sh; B=$HERE/build-campaign
SCR=$(mktemp -d); export AK_SERVE_STATE=$SCR/serve.state
trap '[ -f "$AK_SERVE_STATE" ] && bash "$SERVE" stop > /dev/null 2>&1; rm -rf "$SCR"' EXIT
F=0
ok() { echo ">>> ok: $*"; }
bad() { echo ">>> FAIL: $*"; F=$((F + 1)); }
{
  echo "# deferred_checks: commit $(git -C "$FFI" rev-parse --short HEAD)$(git -C "$FFI" status --porcelain -- poc/cpp/src poc/cpp/gen | grep -q . && echo ' + UNCOMMITTED'), $(date -u +%FT%TZ)"
  echo "# patch tree $PT at $(git -C "$PT" rev-parse --short HEAD), diff sha256 $(git -C "$PT" diff | sha256sum | cut -c1-16): $(git -C "$PT" diff --stat | tail -1)"
  echo "# core $CD/libak_core.so sha256 $(sha256sum "$CD/libak_core.so" | cut -c1-16); no-unknown $CDN/libak_core.so sha256 $(sha256sum "$CDN/libak_core.so" | cut -c1-16); knobs: ${KNOBS:-none}"
  AK_SERVER_THREADS=8 bash "$SERVE" start --out "$SCR/srv" > "$SCR/srv.out" 2>&1 || { cat "$SCR/srv.out"; exit 1; }
  SOCK=unix:$(sed -n 's/^pinned //p' "$AK_SERVE_STATE")
  echo "===== 1. the patch tree's binaries"
  for c in conformance_a17_shared conformance_nounk_a17; do
    (cd ../../schema/generated && env $KNOBS "$PB/$c" payloads > "$SCR/c" 2>&1); rc=$?
    r=$(grep -E 'checks,' "$SCR/c" | tail -1); echo "  $c: exit $rc: $r"
    [ $rc = 0 ] && echo "$r" | grep -q ' 0 failures' && ok "$c" || bad "$c"
  done
  python3 gen/u_rows.py ../../corpus/generated "$SCR/rows.tsv" 2>/dev/null
  for c in campaign_codec campaign_codec_nounk; do
    (cd ../../schema/generated && env $KNOBS taskset -c "$AK_CPU_CLIENT" "$PB/$c" --rounds 0 --pool-bytes 1048576 \
      --corpus "$PWD/../../corpus/generated" --rows "$SCR/rows.tsv" > "$SCR/p" 2>&1); rc=$?
    g=$(grep -o '"campaign_codec_gate": {[^}]*}' "$SCR/p"); echo "  $c: exit $rc $g"
    [ $rc = 0 ] && echo "$g" | grep -q '"failed": 0' && ok "$c pre-check" || bad "$c pre-check"
  done
  echo "===== 2. --semantics 1, this tree's clients on the patched core"
  for v in full nounk; do
    exe=$B/campaign_rpc; d=$CD; [ $v = nounk ] && { exe=$B/campaign_rpc_nounk; d=$CDN; }
    env LD_LIBRARY_PATH=$d $KNOBS taskset -c "$AK_CPU_CLIENT" "$exe" --target "$SOCK" --expect 540422 --transport pinned --semantics 1 > "$SCR/s" 2>&1; rc=$?
    grep -E '^(PASS|FAIL|SKIP)' "$SCR/s" | sed 's/^/    /'
    [ $rc = 0 ] && grep -q '"failed": 0' "$SCR/s" && [ "$(grep -c 'deferred' "$SCR/s")" -ge 7 ] && ok "semantics $v ($(grep -o '"checks": [0-9]*' "$SCR/s"))" || bad "semantics $v rc=$rc"
  done
  echo "===== 3. HEAD's core: the deferred cases skipped, a Cf-enc cell refused"
  taskset -c "$AK_CPU_CLIENT" "$B/campaign_rpc" --target "$SOCK" --expect 540422 --transport pinned --semantics 1 > "$SCR/s" 2>&1; rc=$?
  [ $rc = 0 ] && grep -q '^SKIP deferred' "$SCR/s" && ok "HEAD core: semantics pass, deferred skipped ($(grep -o '"checks": [0-9]*' "$SCR/s"))" || bad "HEAD core semantics rc=$rc"
  taskset -c "$AK_CPU_CLIENT" "$B/campaign_rpc" --target "$SOCK" --expect 540422 --transport pinned --cells Cf-enc-retain --dirs d \
    --inflight 1 --rounds 1 --gbench-out "$SCR/g.json" > "$SCR/r" 2>&1; rc=$?
  [ $rc = 2 ] && grep -q REFUSED "$SCR/r" && ok "HEAD core refuses Cf-enc: $(grep REFUSED "$SCR/r")" || bad "HEAD core did not refuse Cf-enc (rc=$rc)"
  echo "===== 4. every d call checked against the server's count and SHA-256 (--check-stream 1), patched core"
  rm -f "$SCR/g.json"
  env LD_LIBRARY_PATH=$CD $KNOBS taskset -c "$AK_CPU_CLIENT" "$B/campaign_rpc" --target "$SOCK" --expect 540422 --transport pinned \
    --cells A,D-retain,Cf-retain,Cf-enc-retain,Cf-encp-retain,Cf-q-retain --dirs d --inflight 1,8 --rounds 1 --min-time-s 0.001 \
    --warmup-s 0 --workers 8 --check-stream 1 --gbench-out "$SCR/g.json" > "$SCR/o" 2>&1; rc=$?
  n=$(python3 gen/gbench_to_jsonl.py "$SCR/g.json" 1 full rpc 2>/dev/null | grep -c '^{')
  python3 gen/gbench_to_jsonl.py "$SCR/g.json" 1 full rpc 2>/dev/null | grep '^{' | python3 -c "
import json,sys
for l in sys.stdin:
    d=json.loads(l); print('    %-16s %-6s k=%d  %d calls checked' % (d['cell'], d['payload'], d['inflight'], d['iters']))"
  [ $rc = 0 ] && [ "$n" = 24 ] && ok "check-stream grid: $n benchmarks (6 cells x 2 payloads x k 1, 8), every call's count and SHA-256 matched" \
    || { grep -m3 -E 'CALL CHECK|REFUSED' "$SCR/o"; bad "check-stream grid rc=$rc n=$n"; }
  echo "deferred_checks: $F failure(s)"
} > "$LOGF" 2>&1
grep -E '>>>|deferred_checks:' "$LOGF"
[ $F = 0 ]
