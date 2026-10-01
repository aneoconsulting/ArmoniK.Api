#!/usr/bin/env bash
# h2 PR 903 against stock h2 and p4 (2026-10-01, owner), UDS and TCP loopback, timed in-process:
#   flock /tmp/ak-physical-bench.lock bash gen/h2_pr903_bench.sh OUT_DIR WT CORES_PREFIX
# gen/inproc.sh with 8 conditions alternated, 3 processes each (AK_H9_REPS), one server (8
# workers, 5-8,15-18) on the pinned Unix socket and TCP 127.0.0.1 (TCP_NODELAY on accept, read
# back on every client socket: AK_EXPECT_NODELAY=1). Every condition runs the probe with
# AK_PROBE_TASKCLOCK=1 (task-clock beside the process clock: softirq-inclusive on TCP) and
# AK_PROBE_SERVER_PID (server task-clock per call), pinned allocator, host and core 8 workers,
# AK_SPARES=6 AK_SPARE_LOCK=1. Conditions (TRANSPORT = uds|tcp):
#   stock-T  stack probe (WT target/, crates.io h2) + core CORES_PREFIX-ctl (crates.io h2)
#   p4-T     stack probe + core CORES_PREFIX-p4 (h2 + p4-h2-coalesce), AK_H2_COALESCE=16
#   prc-T    stack probe + core CORES_PREFIX-pr (h2 0.4.19 + PR 903 port): PR in the core only
#   prh-T    WT target-pr903host probe (PR in tonic in the host) + core CORES_PREFIX-pr
# AK_H9_SESSION=2 (second session): prh-T replaced by
#   pp-T     stack probe + core CORES_PREFIX-pr903p4 (PR 903 port + p4 combined), AK_H2_COALESCE=16
set -Eeuo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?usage: h2_pr903_bench.sh OUT_DIR WT CORES_PREFIX}; WT=${2:?}; CP=${3:?}
B=$WT/ffi/poc/rust/target/release/stream_probe; H=$WT/ffi/poc/rust/target-pr903host/release/stream_probe
TUN=GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
COM="AK_HOST_WORKERS=8,AK_CORE_WORKERS=8,AK_SPARES=6,AK_SPARE_LOCK=1,$TUN,AK_PROBE_TASKCLOCK=1,AK_PROBE_SERVER_PID=@SPID@"
TCPE="AK_RPC_TARGET=http://@TCP@,AK_EXPECT_NODELAY=1"
C=""
for t in uds tcp; do
  x=$COM; [ $t = tcp ] && x="$TCPE,$COM"
  C+=" stock-$t=$B:LD_LIBRARY_PATH=$CP-ctl/release,$x"
  C+=" p4-$t=$B:LD_LIBRARY_PATH=$CP-p4/release,AK_H2_COALESCE=16,$x"
  C+=" prc-$t=$B:LD_LIBRARY_PATH=$CP-pr/release,$x"
  if [ "${AK_H9_SESSION:-1}" = 2 ]; then
    C+=" pp-$t=$B:LD_LIBRARY_PATH=$CP-pr903p4/release,AK_H2_COALESCE=16,$x"
  else
    C+=" prh-$t=$H:LD_LIBRARY_PATH=$CP-pr/release,$x"
  fi
done
env AK_IP_TRANSPORT=uds AK_IP_SERVER_TCP=1 AK_IP_REPS=${AK_H9_REPS:-3} AK_IP_WORKS="${AK_H9_WORKS:-d16k1 d16k8 d4k1 c54k1 c54k8}" \
    AK_IP_CELLS=A,Cf,Cf-cb,Cf-zc AK_IP_CELLS_C=A,Cf,Cf-cb AK_IP_CONDS="${C# }" bash gen/inproc.sh "$OUT"
