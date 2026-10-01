#!/usr/bin/env bash
# Correctness of the h2 PR 903 builds (2026-10-01), under the bench lock:
#   flock /tmp/ak-physical-bench.lock gen/h2_pr903_checks.sh OUT_DIR WT CORE_PR_DIR
# core-only: the worktree stack's codec_suite pre-check (AK_PRECHECK_ONLY, 0 failures), its
#   upload_check and rpc_semantics (crates.io h2 in the host), and this tree's burst_check against
#   one serve.sh server, each with LD_LIBRARY_PATH=CORE_PR_DIR (the PR core); ldd recorded;
# host-too: upload_check and rpc_semantics of WT/ffi/poc/rust/target-pr903host (h2 PR in the host
#   and in its own libak_core.so); AK_CHK_HOST=0 skips it. AK_H2_COALESCE, if set, reaches every
#   check (the h2-pr903-p4 core reads it).
set -uo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?}; WT=${2:?}; CORE=${3:?}; mkdir -p "$OUT"; LOG="$OUT/checks.log"; : > "$LOG"
say() { echo "$*" | tee -a "$LOG"; }
R=$WT/ffi/poc/rust/target/release; H=$WT/ffi/poc/rust/target-pr903host/release
PC=$(ls -t "$R"/deps/codec_suite-* | grep -v '\.d$' | head -1)
rc=0
say "# h2 PR 903 checks, $(date -u +%FT%TZ); AK_H2_COALESCE=${AK_H2_COALESCE:-unset}; core-only core $CORE/libak_core.so sha256 $(sha256sum "$CORE/libak_core.so" | cut -c1-16)"
run() { # NAME ENV... -- CMD...
  local name=$1; shift; local env=(); while [ "$1" != -- ]; do env+=("$1"); shift; done; shift
  say "$name: loads $(env "${env[@]}" ldd "$1" | awk '/libak_core/{print $3}')"
  env "${env[@]}" taskset -c 1-4,11-14 "$@" > "$OUT/$name.log" 2>&1; local r=$?
  say "$name: rc $r, $(grep -m1 -E '^# precheck:|PASSED|FAILED' "$OUT/$name.log" || tail -1 "$OUT/$name.log")"
  [ $r = 0 ] || rc=1
}
SCR=$(mktemp -d)
run precheck-core-only LD_LIBRARY_PATH="$CORE" CRITERION_HOME="$SCR/crit" AK_OUT="$SCR/pc.jsonl" AK_PRECHECK_ONLY=1 AK_ZC=P5. -- "$PC"
grep -q '^# precheck: .* 0 failures' "$OUT/precheck-core-only.log" || { say "precheck: failures"; rc=1; }
for b in upload_check rpc_semantics; do run "$b-core-only" LD_LIBRARY_PATH="$CORE" -- "$R/$b"; done
[ "${AK_CHK_HOST:-1}" = 1 ] && for b in upload_check rpc_semantics; do run "$b-host-too" AK_UNUSED=1 -- "$H/$b"; done
export AK_SERVE_STATE="$SCR/state"
AK_CPU_SERVER=5-8,15-18 AK_SERVER_THREADS=8 ./serve.sh start --out "$OUT/server" > /dev/null
./serve.sh warm 2 > /dev/null 2>&1
run burst_check-core-only LD_LIBRARY_PATH="$CORE" AK_RPC_SOCKET_SHIPPED="$(sed -n 's/^shipped //p' "$AK_SERVE_STATE")" \
    AK_RPC_SOCKET_PINNED="$(sed -n 's/^pinned //p' "$AK_SERVE_STATE")" -- "$HERE/target/release/burst_check"
./serve.sh stop > /dev/null 2>&1
rm -rf "$SCR"
say "checks rc=$rc"
exit $rc
