#!/usr/bin/env bash
# UDS against TCP loopback from the Rust host (2026-10-01, owner): the stack's stream_probe
# (AK_TU_STACK), crates.io h2, AK_SPARES=6 AK_SPARE_LOCK=1, pinned allocator, host and core 8
# workers, one server (serve.sh, 8 workers, 5-8,15-18) listening on the pinned Unix socket and on
# TCP 127.0.0.1 (pinned configuration, TCP_NODELAY on accept). Run under the bench lock:
#   flock /tmp/ak-physical-bench.lock bash gen/tcp_vs_uds.sh OUT_DIR
#  1. timed: gen/inproc.sh, conditions uds and tcp alternated, 3 processes each, affinity checked;
#     tcp processes abort unless every TCP socket has TCP_NODELAY (getsockopt, AK_EXPECT_NODELAY)
#  2. server CPU per cell: perf stat -p <server> counting only one cell's timed rounds (perf
#     --control driven by the probe, AK_PERF_CELL), each cell of each transport in turn
#  3. untimed attribution: /proc per thread class (CPU, write and read syscalls, bytes written)
#  4. perf record --call-graph dwarf of the client for A and Cf on TCP d/16 k=1 (AK_PERF_CELL),
#     classified with the booted System.map (gen/perf_classify.py)
set -Eeuo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?usage: tcp_vs_uds.sh OUT_DIR}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
B=${AK_TU_STACK:?AK_TU_STACK: the stack build of stream_probe}
SYSMAP=${AK_TU_SYSMAP:-$(dirname "$(readlink -f /run/booted-system/kernel)")/System.map}
TUN=GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
CELLS=A,Df,Df-1f,Cf,Cf-cb,Cf-zc; CELLS_C=A,Df,Df-1f,Cf,Cf-cb
COMMON="AK_HOST_WORKERS=8,AK_CORE_WORKERS=8,AK_SPARES=6,AK_SPARE_LOCK=1,$TUN"

# 1. timed, in-process
env AK_IP_TRANSPORT=uds AK_IP_SERVER_TCP=1 AK_IP_REPS=${AK_TU_REPS:-3} AK_IP_WORKS="d16k1 d16k8 d4k1 c54k1 c54k8" AK_IP_CELLS=$CELLS AK_IP_CELLS_C=$CELLS_C \
    AK_IP_CONDS="uds=$B:$COMMON tcp=$B:AK_RPC_TARGET=http://@TCP@,AK_EXPECT_NODELAY=1,$COMMON" bash gen/inproc.sh "$OUT/timed"

# 2-4 on one more server of the same configuration
export AK_SERVE_STATE; AK_SERVE_STATE=$(mktemp -u)
cleanup() { ./serve.sh stop > /dev/null 2>&1 || true; }
trap cleanup EXIT
AK_SERVER_TCP=0 AK_CPU_SERVER=5-8,15-18 AK_SERVER_THREADS=8 ./serve.sh start --out "$OUT/server-attr" > /dev/null
SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE"); SPID=$(sed -n 's/^pid //p' "$AK_SERVE_STATE"); TCP=$(sed -n 's/^tcp //p' "$AK_SERVE_STATE")
./serve.sh warm 8 > "$OUT/server-attr/warm.log"
SCR=$(mktemp -d)
WRAP=()
probe() { # TRANSPORT OUT SIZE K CELLS PROC [ENV=V...]; the global array WRAP wraps the probe
  local tr=$1 o=$2 size=$3 k=$4 cells=$5 proc=$6; shift 6
  local tgt=(); [ "$tr" = tcp ] && tgt=(AK_RPC_TARGET=http://$TCP AK_EXPECT_NODELAY=1)
  env "${tgt[@]}" "$@" AK_EXPECT_CPUS=1-4,11-14 AK_HOST_WORKERS=8 AK_CORE_WORKERS=8 AK_SPARES=6 AK_SPARE_LOCK=1 "$TUN" \
      AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT=pinned AK_OUT="$o.jsonl" AK_PROBE_CELLS="$cells" AK_PROBE_SIZES="$size" AK_PROBE_K="$k" \
      AK_PROBE_ROUNDS=$([ "$k" = 1 ] && echo 3 || echo 2) AK_PROBE_CALLS=$([ "$k" = 1 ] && echo 6 || echo 2) AK_PROBE_WARM=2 \
      AK_PROBE_ORDER=block AK_PROBE_PROC="$proc" "${WRAP[@]}" taskset -c 1-4,11-14 "$B" 2> "$o.err"
}
# 2. server CPU per cell
mkdir -p "$OUT/server-cpu"
for tr in uds tcp; do
  for w in "16MiB 1 $CELLS" "16MiB 8 $CELLS" "P5.4 1 $CELLS_C"; do
    read -r size k cells <<< "$w"
    for cell in ${cells//,/ }; do
      rm -f "$SCR/ctl" "$SCR/ack"; mkfifo "$SCR/ctl" "$SCR/ack"
      o="$OUT/server-cpu/$tr-$size-k$k-$cell"
      perf stat -x, -e task-clock,context-switches,cycles,instructions -p "$SPID" --control "fifo:$SCR/ctl,$SCR/ack" -D -1 -o "$o.perf" &
      PP=$!
      sleep 0.3
      probe "$tr" "$o" "$size" "$k" "$cells" 0 AK_PERF_CTL="$SCR/ctl,$SCR/ack" AK_PERF_CELL="$cell"
      kill -INT "$PP"; wait "$PP" || true
    done
  done
done
# 3. untimed attribution (/proc per thread class)
mkdir -p "$OUT/attr"
for tr in uds tcp; do
  for w in "16MiB 1 $CELLS" "16MiB 8 $CELLS" "4MiB 1 $CELLS" "P5.4 1 $CELLS_C" "P5.4 8 $CELLS_C"; do
    read -r size k cells <<< "$w"
    probe "$tr" "$OUT/attr/$tr-$size-k$k" "$size" "$k" "$cells" 1
  done
done
# 4. perf record, TCP d/16 k=1, A and Cf (and the same on UDS for the buckets beside them)
mkdir -p "$OUT/record"
for tr in tcp uds; do
  for cell in A Cf; do
    rm -f "$SCR/ctl" "$SCR/ack"; mkfifo "$SCR/ctl" "$SCR/ack"
    o="$OUT/record/$tr-$cell"
    WRAP=(perf record -q --call-graph dwarf,16384 -F 1999 --control "fifo:$SCR/ctl,$SCR/ack" -D -1 -o "$SCR/$tr-$cell.data" --)
    probe "$tr" "$o" 16MiB 1 "$CELLS" 0 AK_PERF_CTL="$SCR/ctl,$SCR/ack" AK_PERF_CELL="$cell"
    WRAP=()
    perf script -i "$SCR/$tr-$cell.data" -F comm,tid,period,ip,sym,dso 2> /dev/null > "$SCR/$tr-$cell.script"
    python3 gen/perf_classify.py "$SCR/$tr-$cell.script" "$SYSMAP" > "$o.txt"
  done
done
rm -rf "$SCR"
echo "# server CPU ticks, attribution server: $(awk '{print $14+$15}' /proc/$SPID/stat)" >> "$OUT/timed/header.txt"
