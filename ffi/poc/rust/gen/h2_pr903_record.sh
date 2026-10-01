#!/usr/bin/env bash
# Attribution of PR 903's client CPU at k=1 (2026-10-01): perf record (no call graph, -F 4999,
# every client thread) of the Cf cell's timed rounds only (AK_PERF_CTL + AK_PERF_CELL=Cf), d/16
# k=1 on UDS, one process per core (AK_REC_CORES, default `ctl pr`: crates.io h2, the PR 903 port),
# stack probe, pinned allocator; perf report by DSO and symbol (self; kernel addresses unresolved,
# kptr_restrict 1), and perf annotate of the PR's poll_write_buf. Under the bench lock:
#   flock /tmp/ak-physical-bench.lock bash gen/h2_pr903_record.sh OUT_DIR WT CORES_PREFIX
set -Eeuo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?}; WT=${2:?}; CP=${3:?}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
B=$WT/ffi/poc/rust/target/release/stream_probe
TUN=GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
SCR=$(mktemp -d); export AK_SERVE_STATE=$SCR/state
cleanup() { ./serve.sh stop > /dev/null 2>&1 || true; rm -rf "$SCR"; }
trap cleanup EXIT
AK_CPU_SERVER=5-8,15-18 AK_SERVER_THREADS=8 ./serve.sh start --out "$OUT/server" > /dev/null
./serve.sh warm 8 > /dev/null 2>&1
SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE")
for core in ${AK_REC_CORES:-ctl pr}; do
  n=$core
  rm -f "$SCR/ctl" "$SCR/ack"; mkfifo "$SCR/ctl" "$SCR/ack"
  env LD_LIBRARY_PATH="$CP-$core/release" AK_HOST_WORKERS=8 AK_CORE_WORKERS=8 AK_SPARES=6 AK_SPARE_LOCK=1 "$TUN" AK_EXPECT_CPUS=1-4,11-14 \
      AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT=pinned AK_OUT="$OUT/$n.jsonl" AK_PROBE_CELLS=A,Cf AK_PROBE_SIZES=16MiB AK_PROBE_K=1 \
      AK_PROBE_ROUNDS=8 AK_PROBE_CALLS=8 AK_PROBE_WARM=4 AK_PROBE_ORDER=block AK_PROBE_TASKCLOCK=1 \
      AK_PERF_CTL="$SCR/ctl,$SCR/ack" AK_PERF_CELL=Cf \
      perf record -q -F 4999 --control "fifo:$SCR/ctl,$SCR/ack" -D -1 -o "$OUT/$n.data" -- taskset -c 1-4,11-14 "$B" 2> "$OUT/$n.err"
  perf report -i "$OUT/$n.data" --no-children --sort dso,sym --percent-limit 0.3 --stdio 2> /dev/null | grep -v '^$' > "$OUT/$n.report.txt"
  # the PR's poll_write_buf, annotated (where its self cycles go)
  perf annotate -i "$OUT/$n.data" --stdio -s h2::codec::framed_write::poll_write_buf 2> /dev/null | grep -v '^\s*0.00 :' | head -80 > "$OUT/$n.annotate-poll_write_buf.txt" || true
  rm -f "${OUT:?}/$n.data"
done
