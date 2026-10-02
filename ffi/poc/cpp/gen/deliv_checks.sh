#!/usr/bin/env bash
# Checks of the delivery cells (owner, 2026-10-01) on one core, before any timing:
#   1. gen/deferred_checks.sh (conformance and pre-check loading the core, --semantics 1 with the core's
#      callback delivery cases required, check-stream: every d call of A, A-cb, A-q, D, Cf, Cf-q, Cf-cb at d/4
#      and d/16, k = 1 and 8, to UploadStreamCheck, the server's count and SHA-256 matched);
#   2. direction c of the new cells (A-cb, A-q, Cf-cb), k = 1 and 8, one repetition: every call's status and
#      response length checked in the cell (a failure exits non-zero).
#   gen/deliv_checks.sh LOGDIR NAME CORE_DIR CORE_NOUNK_DIR "KNOBS"
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
L=${1:?}; N=${2:?}; CD=${3:?}; CDN=${4:?}; KNOBS=${5:-}
mkdir -p "$L"; L=$(cd "$L" && pwd)
F=0
DC_STEP1_CORE=1 DC_CELLS=A,A-cb,A-q,D-retain,Cf-retain,Cf-q-retain,Cf-cb-retain DC_EXPECT="cb]:8 unary_enc_cb:2" \
  bash gen/deferred_checks.sh "$L/checks-$N.log" "$HERE/../../.." "$CD" "$CDN" "$KNOBS" || F=1
flock /tmp/ak-physical-bench.lock bash -c '
  . gen/net_target.sh
  S=$(mktemp -d); export AK_SERVE_STATE=$S/st AK_CPU_SERVER=${AK_CPU_SERVER:-5-8,15-18}
  env $(net_server_env) AK_SERVER_THREADS=8 bash ../rust/serve.sh start --out $S/srv > $S/o 2>&1 || { cat $S/o; exit 1; }
  net_endpoints $AK_SERVE_STATE; echo "# endpoint: $NET_DESC; core '"$CD"' $(sha256sum '"$CD"'/libak_core.so | cut -c1-16)"
  env LD_LIBRARY_PATH='"$CD"' '"$KNOBS"' taskset -c ${AK_CPU_CLIENT:-1-4,11-14} build-campaign/campaign_rpc --target $NET_TGT --core-target $NET_CTGT \
    --expect 540422 --transport pinned --cells A-cb,A-q,Cf-cb-retain --dirs c --inflight 1,8 --rounds 1 --min-time-s 0.001 \
    --warmup-s 0 --workers 8 --gbench-out $S/g.json > $S/r 2>&1; rc=$?
  grep -E "^(A-cb|A-q|Cf-cb)|REFUSED|CALL CHECK|status" $S/r | head -20
  n=$(python3 gen/gbench_to_jsonl.py $S/g.json 1 full rpc 2>/dev/null | grep -c "^{")
  echo "c grid: exit $rc, $n benchmarks (3 cells x P5.3 and P5.4 x k 1, 8 = 12 expected)"
  bash ../rust/serve.sh stop > /dev/null 2>&1; rm -rf $S
  [ $rc = 0 ] && [ "$n" = 12 ]' > "$L/c-grid-$N.log" 2>&1 && echo ">>> ok: c grid $N" || { echo ">>> FAIL: c grid $N"; F=1; }
cat "$L/c-grid-$N.log" | tail -2
exit $F
