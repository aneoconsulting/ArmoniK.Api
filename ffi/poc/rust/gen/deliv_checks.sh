#!/usr/bin/env bash
# Correctness of the delivery cells before timing (2026-10-01), TCP, own server, under the lock:
#   flock /tmp/ak-physical-bench.lock gen/deliv_checks.sh OUT_DIR CORES
# For each (probe, core): target-deliv + CORES/stock, target-deliv + CORES/h2-batch,
# target-deliv-h2batch + CORES/h2-batch (A in host-too): bin stream_probe with AK_PROBE_CHECK=1
# (direction d on UploadStreamCheck: the server's data byte count AND the SHA-256 of the messages
# as received, every call) over every delivery cell, d/16 MiB and d/4 MiB at k = 1 and 8, and
# c/P5.4 at k = 1 and 8 (response length, the Empty answer decoded by A's codec); a failed check
# panics the probe (requirement 18). Then the negative control AK_PROBE_PLANT=1 (one byte more
# expected) per delivery cell and direction: every one must FAIL.
set -uo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?}; CORES=${2:?}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd); LOG="$OUT/checks.log"; : > "$LOG"
say() { echo "$*" | tee -a "$LOG"; }
TUN=GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
SCR=$(mktemp -d); export AK_SERVE_STATE=$SCR/state
cleanup() { ./serve.sh stop > /dev/null 2>&1 || true; rm -rf "$SCR"; }
trap cleanup EXIT
AK_SERVER_TCP=0 AK_CPU_SERVER=5-8,15-18 AK_SERVER_THREADS=8 ./serve.sh start --out "$OUT/server" > /dev/null
./serve.sh warm 4 > /dev/null 2>&1
SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE"); TCP=$(sed -n 's/^tcp //p' "$AK_SERVE_STATE")
CELLS=A,A-blk,A-cb,A-q,Cf,Cf-cb,Cf-q
rc=0
probe() { # NAME PROBE CORE SIZES K [ENV...]
  local name=$1 b=$2 core=$3 sizes=$4 k=$5; shift 5
  env "$@" LD_LIBRARY_PATH="$core" AK_HOST_WORKERS=8 AK_CORE_WORKERS=8 "$TUN" AK_EXPECT_CPUS=1-4,11-14 \
      AK_RPC_SOCKET="$SOCK" AK_RPC_TARGET="http://$TCP" AK_EXPECT_NODELAY=1 AK_RPC_TRANSPORT=pinned AK_OUT="$OUT/$name.jsonl" \
      AK_PROBE_SIZES="$sizes" AK_PROBE_K="$k" AK_PROBE_ROUNDS=1 AK_PROBE_CALLS=2 AK_PROBE_WARM=1 AK_PROBE_ORDER=block \
      taskset -c 1-4,11-14 "$b" > "$OUT/$name.out" 2> "$OUT/$name.err"
}
for pc in "stock:target-deliv:stock" "h2-batch:target-deliv:h2-batch" "host-too:target-deliv-h2batch:h2-batch"; do
  IFS=: read -r tag tgt core <<< "$pc"
  b="$HERE/$tgt/release/stream_probe"
  say "# $tag: probe $b ($(sha256sum "$b" | cut -c1-16)), core $CORES/$core/release ($(sha256sum "$CORES/$core/release/libak_core.so" | cut -c1-16))"
  for k in 1 8; do
    for sizes in "16MiB,4MiB" "P5.4"; do
      n="$tag-k$k-${sizes%%,*}"
      probe "$n" "$b" "$CORES/$core/release" "$sizes" "$k" AK_PROBE_CELLS="$CELLS" AK_PROBE_CHECK=1
      r=$?; rows=$(grep -c '"cpu_ns"' "$OUT/$n.jsonl" 2>/dev/null || echo 0)
      say "check $n ($CELLS, $sizes, k=$k, $( [ "$sizes" = P5.4 ] && echo 'response length' || echo 'server byte count + SHA-256')): rc $r, $rows rounds"
      [ $r = 0 ] || rc=1
    done
  done
done
b="$HERE/target-deliv/release/stream_probe"
for cell in A-blk A-cb A-q Cf-q; do
  for sizes in 4MiB P5.4; do
    n="plant-$cell-$sizes"
    probe "$n" "$b" "$CORES/stock/release" "$sizes" 1 AK_PROBE_CELLS="$cell" AK_PROBE_CHECK=1 AK_PROBE_PLANT=1
    r=$?
    # the failure must be the planted one (the expected count), not another error
    why=$(grep -m1 -o 'data bytes, expected [0-9]*\|response 0 B, expected 1' "$OUT/$n.err" | head -1)
    say "plant $cell $sizes (one byte more expected): rc $r ($( [ $r != 0 ] && [ -n "$why" ] && echo 'FAILED as required' || echo 'CONTROL BROKEN')): $(grep -m1 -o 'panicked.*\|ABORT.*' "$OUT/$n.err" | head -c 220; grep -m1 -o 'warm-up .*' "$OUT/$n.err" | head -c 220)"
    { [ $r != 0 ] && [ -n "$why" ]; } || rc=1
  done
done
say "checks rc=$rc"
exit $rc
