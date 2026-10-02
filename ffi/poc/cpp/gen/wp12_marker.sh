#!/usr/bin/env bash
# The h2 variant marker (FIX-PLAN WP12, D11): socket writes per d/16MiB call of Cf-retain over TCP 127.0.0.1,
# k = 1 and 8, from strace -f of one `campaign_rpc --profile` process (gen/strace_threads.py, between the loop's
# markers). Stock h2 writes each 16 KiB DATA frame with its own writev (about 1027 per call at k = 1); h2-batch
# (PR 903 + p4, AK_H2_COALESCE 16) about 73. Nothing is timed; the strace text is deleted after counting.
#   gen/wp12_marker.sh CORE_DIR SCRATCH_DIR     (AK_CPU_CLIENT, AK_CPU_SERVER from the environment)
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
CD=${1:?CORE_DIR}; S=${2:?SCRATCH_DIR}; mkdir -p "$S"
AK_NET=tcp; . gen/net_target.sh
export AK_SERVE_STATE=$S/st
trap 'bash ../rust/serve.sh stop > /dev/null 2>&1' EXIT
env AK_SERVER_TCP=0 AK_SERVER_THREADS=4 AK_CPU_SERVER="$AK_CPU_SERVER" bash ../rust/serve.sh start --out "$S/srv" > "$S/o" 2>&1 \
  || { cat "$S/o"; exit 1; }
net_endpoints "$AK_SERVE_STATE" || exit 1
echo "# endpoint: $NET_DESC"
echo "# client: build-campaign/campaign_rpc with LD_LIBRARY_PATH=$CD (libak_core.so sha256 $(sha256sum "$CD/libak_core.so" | cut -c1-64))"
F=0
for k in 1 8; do
  n=$([ $k = 1 ] && echo 8 || echo 2)
  strace -f -qq -T -yy -s 16 -e signal=none -o "$S/st.txt" \
    -e trace=writev,sendmsg,write,sendto,recvfrom,recvmsg,read,futex,epoll_wait,epoll_pwait,epoll_pwait2 \
    env LD_LIBRARY_PATH="$CD" taskset -c "$AK_CPU_CLIENT" build-campaign/campaign_rpc --target "$NET_TGT" \
    --core-target "$NET_CTGT" --expect 540422 --transport pinned --cells Cf-retain --dirs d --payloads 16MiB \
    --inflight $k --workers 2 --profile $n --profile-chunks 1 > "$S/out" 2>&1 || { tail -3 "$S/out"; F=1; continue; }
  net_nodelay_ok "$S/out" && echo "  TCP_NODELAY read back 1 on every client TCP socket" || { echo "REFUSED: a client TCP socket without TCP_NODELAY"; F=1; }
  calls=$(python3 -c '
import json, sys
for l in open(sys.argv[1]):
    if l.startswith("{\"profile\""): print(json.loads(l)["profile"]["calls"])' "$S/out")
  j=$(python3 gen/strace_threads.py "$S/st.txt" "$calls" 50)
  echo "Cf-retain d/16MiB k=$k: $calls calls; strace_threads: $j"
  python3 -c '
import json, sys
d = json.loads(sys.argv[1])
print("  socket writes per call: %.1f (threads writing the socket: %d)" % (d["per_call"].get("socket_writes", 0), d["threads_writing_the_socket"]))' "$j"
  rm -f "$S/st.txt"
done
exit $F
