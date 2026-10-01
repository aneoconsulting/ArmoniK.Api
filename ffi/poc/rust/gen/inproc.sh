#!/usr/bin/env bash
# In-process comparisons (2026-09-30, owner's goals 1 and 2): every cell of AK_IP_CELLS in ONE
# bin stream_probe process per (condition, workload, repetition), block order, the cell order
# rotated by one per repetition, conditions alternated inside each repetition. Own server
# (serve.sh, 8 workers, SERVER set, pinned socket; AK_IP_SERVER_THREADS), started and stopped
# here; run the whole script under the benchmark lock:
#   flock /tmp/ak-physical-bench.lock bash gen/inproc.sh OUT_DIR
# Conditions: AK_IP_CONDS, space-separated NAME=PROBE_BINARY[:ENV=V,ENV=V...] (PROBE_BINARY `main` =
# this tree's target/release/stream_probe; another path = e.g. a patched worktree's build).
# Workloads: AK_IP_WORKS from d16k1 d16k8 d4k1 d4k8 c54k1 c54k8. Tables: gen/inproc_tables.py.
# Transport (owner, 2026-10-01: TCP for every benchmark from now on): AK_IP_TRANSPORT=tcp (the
# default): the server also listens on TCP 127.0.0.1 (TCP_NODELAY on accept) and every client
# process gets AK_RPC_TARGET=http://127.0.0.1:PORT and AK_EXPECT_NODELAY=1 (the probe reads
# TCP_NODELAY back on every live client socket and aborts unless all have it); AK_IP_TRANSPORT=uds:
# the pinned Unix socket only (the history before 2026-10-01). Both: every process runs with
# AK_PROBE_TASKCLOCK=1 (inherited perf task-clock beside the process clock: softirq-inclusive on
# TCP) and AK_PROBE_SERVER_PID=<server> (server task-clock per call); a condition's env may override.
set -Eeuo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
. "$HERE/gen/machine_header.sh"
OUT=${1:?usage: inproc.sh OUT_DIR}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
[ -e "$OUT/header.txt" ] && { echo "inproc.sh: $OUT already holds a session" >&2; exit 2; }
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1-4,11-14} AK_CPU_SERVER=${AK_CPU_SERVER:-5-8,15-18}
export AK_HOST_WORKERS=${AK_HOST_WORKERS:-8} AK_CORE_WORKERS=${AK_CORE_WORKERS:-8}
CELLS=${AK_IP_CELLS:-A,A2,Df,Cf,Ff}
# the cells of the direction-c workloads (default: the same); stream-only cells are left out there
CELLS_C=${AK_IP_CELLS_C:-$CELLS}
WORKS=${AK_IP_WORKS:-d16k1 d16k8}
REPS=${AK_IP_REPS:-3}
CONDS=${AK_IP_CONDS:-main=main}
TRANSPORT=${AK_IP_TRANSPORT:-tcp}
case $TRANSPORT in tcp|uds) ;; *) echo "inproc.sh: AK_IP_TRANSPORT must be tcp or uds" >&2; exit 2;; esac
read -r -a CS <<< "$CONDS"
SCR=$(mktemp -d); export AK_SERVE_STATE=$SCR/serve.state
cleanup() { ./serve.sh stop > /dev/null 2>&1 || true; rm -rf "$SCR"; }
trap cleanup EXIT
# AK_IP_SERVER_TCP=1: the server also listens on TCP 127.0.0.1 (serve.sh AK_SERVER_TCP=0); a
# condition's env may then use @TCP@ for its address (e.g. AK_RPC_TARGET=http://@TCP@); @SPID@ is
# the server's pid in any condition (e.g. AK_PROBE_SERVER_PID=@SPID@)
AK_SERVER_TCP=$([ "${AK_IP_SERVER_TCP:-0}" = 1 ] || [ "$TRANSPORT" = tcp ] && echo 0) AK_SERVER_THREADS=${AK_IP_SERVER_THREADS:-8} ./serve.sh start --out "$OUT/server" > /dev/null
./serve.sh warm 16 > "$OUT/server/warm.log" 2>&1
SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE"); SPID=$(sed -n 's/^pid //p' "$AK_SERVE_STATE")
TCPADDR=$(sed -n 's/^tcp //p' "$AK_SERVE_STATE")
[ "$TRANSPORT" = tcp ] && [ -z "$TCPADDR" ] && { echo "inproc.sh: the server has no TCP listener" >&2; exit 1; }
# the per-process environment every condition starts from (its own env is applied after it)
BASE=(AK_PROBE_TASKCLOCK=1 "AK_PROBE_SERVER_PID=$SPID")
[ "$TRANSPORT" = tcp ] && BASE+=("AK_RPC_TARGET=http://$TCPADDR" AK_EXPECT_NODELAY=1)
srv_ticks() { awk '{print $14+$15}' /proc/$SPID/stat; }
bin_of() { local b=${1%%:*}; [ "$b" = main ] && b="$HERE/target/release/stream_probe"; echo "$b"; }
# workload -> size k rounds calls-per-round warm
wl() { case $1 in d16k1) echo "16MiB 1 8 8 4";; d16k8) echo "16MiB 8 6 2 2";; d4k1) echo "4MiB 1 8 16 8";; d4k8) echo "4MiB 8 6 4 2";;
                  c54k1) echo "P5.4 1 8 16 8";; c54k8) echo "P5.4 8 6 4 2";;
                  d16k16) echo "16MiB 16 5 1 1";; d16k32) echo "16MiB 32 4 1 1";; c54k16) echo "P5.4 16 6 2 1";; c54k32) echo "P5.4 32 5 2 1";;
                  *) echo "?"; exit 2;; esac; }
{
  echo "# in-process comparison: commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + UNCOMMITTED'); $(date -u +%FT%TZ); $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); kernel $(uname -r); no_turbo $(cat /sys/devices/system/cpu/intel_pstate/no_turbo); cpu1 min/max $(cat /sys/devices/system/cpu/cpu1/cpufreq/scaling_min_freq)/$(cat /sys/devices/system/cpu/cpu1/cpufreq/scaling_max_freq) kHz; smt $(cat /sys/devices/system/cpu/smt/control); isolation: taskset only"
  machine_header
  echo "# affinity checks: every server thread allowed exactly AK_CPU_SERVER, checked before and after every client process; every client thread allowed exactly AK_CPU_CLIENT, checked by the probe before and after its timed rounds (AK_EXPECT_CPUS); a mismatch aborts"
  echo "# transport $TRANSPORT$([ "$TRANSPORT" = tcp ] && echo " (client target http://$TCPADDR, TCP_NODELAY read back on every client socket)"); base env ${BASE[*]}"
  [ -n "$TCPADDR" ] && echo "# server also on TCP $TCPADDR (pinned configuration, TCP_NODELAY on accept)"
  echo "# netfilter (read-only): modules $(awk '$1 ~ /^(nf|nft|xt|ip|br|iptable|ip6table|ebtable)/ {printf "%s ", $1}' /proc/modules)"
  echo "# client $AK_CPU_CLIENT, host $AK_HOST_WORKERS / core $AK_CORE_WORKERS workers (unless a condition sets them); server $AK_CPU_SERVER, ${AK_IP_SERVER_THREADS:-8} workers, pid $SPID, pinned configuration, transport $TRANSPORT"
  echo "# cells $CELLS (direction c: $CELLS_C) (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads $WORKS (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2; d16k16 5 x 1 batch of 16 warm 1; d16k32 4 x 1 x 32 warm 1; c54k16 6 x 2 x 16 warm 1; c54k32 5 x 2 x 32 warm 1); repetitions $REPS"
  for c in "${CS[@]}"; do
    b=$(bin_of "${c#*=}"); r=${c#*=}; ev=(); [ "$r" != "${r#*:}" ] && IFS=, read -r -a ev <<< "${r#*:}"
    core=$(env "${ev[@]}" ldd "$b" | awk '/libak_core/{print $3}')
    echo "# condition ${c%%=*}: binary $b (sha256 $(sha256sum "$b" | cut -c1-16)), core $core (sha256 $(sha256sum "$core" | cut -c1-16)), env ${c#*:}"
  done
} > "$OUT/header.txt"
# the server's affinity, every thread, must equal AK_CPU_SERVER (checked around every client
# process); the client's is checked inside the probe (AK_EXPECT_CPUS)
srv_ok() {
  local want got t
  want=$(python3 -S -c 'import sys
o=[]
for p in sys.argv[1].split(","):
    a,_,b=p.partition("-"); o+=range(int(a),int(b or a)+1)
print(o)' "$AK_CPU_SERVER")
  for t in /proc/$SPID/task/*; do
    got=$(python3 -S -c 'import sys
o=[]
for p in sys.argv[1].split(","):
    a,_,b=p.partition("-"); o+=range(int(a),int(b or a)+1)
print(o)' "$(sed -n 's/^Cpus_allowed_list:\s*//p' "$t/status")")
    if [ "$got" != "$want" ]; then
      echo "# ABORTED: server thread ${t##*/} allowed $(sed -n 's/^Cpus_allowed_list:\s*//p' "$t/status"), expected $AK_CPU_SERVER ($1)" >> "$OUT/header.txt"
      echo "server affinity mismatch ($1)" >&2; exit 1
    fi
  done
}
rot() { python3 -S -c 'import sys; c=sys.argv[1].split(","); n=int(sys.argv[2])%len(c); print(",".join(c[n:]+c[:n]))' "$1" "$2"; }
S0=$(date +%s)
for rep in $(seq 1 "$REPS"); do
  for w in $WORKS; do
    read -r size k rounds calls warm <<< "$(wl "$w")"
    wc=$CELLS; case $w in c*) wc=$CELLS_C;; esac
    n=${#CS[@]}
    for j in $(seq 0 $((n - 1))); do
      c=${CS[$(( (j + rep - 1) % n ))]}; name=${c%%=*}; rest=${c#*=}; b=$(bin_of "$rest")
      ev=(); [ "$rest" != "${rest#*:}" ] && IFS=, read -r -a ev <<< "${rest#*:}"
      for i in "${!ev[@]}"; do ev[$i]=${ev[$i]//@TCP@/$TCPADDR}; ev[$i]=${ev[$i]//@SPID@/$SPID}; done
      t0=$(srv_ticks)
      srv_ok "before $name $w $rep"
      env "${BASE[@]}" "${ev[@]}" AK_EXPECT_CPUS="$AK_CPU_CLIENT" AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT=pinned AK_OUT="$OUT/$name-$w-$rep.jsonl" AK_PROBE_CELLS="$(rot "$wc" "$rep")" \
          AK_PROBE_SIZES="$size" AK_PROBE_K="$k" AK_PROBE_ROUNDS="$rounds" AK_PROBE_CALLS="$calls" AK_PROBE_WARM="$warm" \
          AK_PROBE_ORDER=block AK_PROBE_PROC=0 taskset -c "$AK_CPU_CLIENT" "$b" 2> "$OUT/$name-$w-$rep.err"
      srv_ok "after $name $w $rep"
      echo "$name $w $rep done at +$(( $(date +%s) - S0 )) s, server CPU ticks $(( $(srv_ticks) - t0 ))" >> "$OUT/progress.txt"
    done
  done
done
echo "# benchmark wall time $(( $(date +%s) - S0 )) s" >> "$OUT/header.txt"
machine_header | sed 's/^# /# at the end: /' >> "$OUT/header.txt"
python3 gen/inproc_tables.py "$OUT" > "$OUT/tables.md"
