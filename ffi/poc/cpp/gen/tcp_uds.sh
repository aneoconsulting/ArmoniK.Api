#!/usr/bin/env bash
# UDS against TCP loopback (owner, 2026-10-01): are the orderings and the gaps to A the same on
# TCP, and where do the absolute costs move? One server process (poc/rust/serve.sh with
# AK_SERVER_TCP=0: the pinned configuration on its Unix socket AND on 127.0.0.1, TCP_NODELAY on
# every accepted socket), one-cell `campaign_rpc --profile` processes, UDS and TCP back to back for
# every (round, workload, unit), their order alternating; the whole block under
# /tmp/ak-physical-bench.lock. Client sockets: grpc++ dials ipv4:127.0.0.1:PORT, the core
# http://127.0.0.1:PORT with ak_client_opts.tcp_nagle = 0 (pinned_core_opts); every TCP process
# reports its sockets' TCP_NODELAY read back with getsockopt (`tcp_sockets` in its profile JSON).
#
#   gen/tcp_uds.sh OUT_DIR STACK_CORE_DIR [ROUNDS]
#
#   units: cur/A, cur/D-retain, cur/Cf-retain, cur/Cf-q-retain (this tree's core); stk/Cf-zc-retain
#          (STACK_CORE_DIR, AK_SPARES=6 AK_SPARE_LOCK=1; direction d only)
#   workloads: d/16MiB k=1 and 8, d/4MiB k=1, c/P5.4 k=1 and 8; ROUNDS (3) processes per cell and transport
#   every process: GLIBC_TUNABLES trim 256 MiB / mmap 32 MiB, core --workers 8, ncpus_shim 8,
#          AK_SERVER_PID (the server's threads by schedstat around the loop) and perf stat -p on the
#          server enabled by the client around the same loop (task-clock, cycles, context switches);
#          the server's affinity checked before every process
#   then one strace process per (workload, unit, transport): writes and syscalls per call
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
FFI=$(cd "$HERE/../.." && pwd)
OUT=${1:?usage: gen/tcp_uds.sh OUT_DIR STACK_CORE_DIR [ROUNDS]}; STK=${2:?STACK_CORE_DIR}; ROUNDS=${3:-3}
mkdir -p "$OUT/proc" "$OUT/strace"; OUT=$(cd "$OUT" && pwd); STK=$(cd "$STK" && pwd)
if [ "${TU_LOCKED:-}" != 1 ]; then
  t=$(date +%s); TU_LOCKED=1 flock /tmp/ak-physical-bench.lock bash "$0" "$@"; rc=$?
  echo "tcp_uds: $(( $(date +%s) - t )) s (lock held), exit $rc" | tee -a "$OUT/runner.log"; exit $rc
fi
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1-4,11-14} AK_CPU_SERVER=${AK_CPU_SERVER:-5-8,15-18}
OSSET=${AK_CPU_OS:-0,9,10,19}; WK=8; GCPUS=8
EXE=$HERE/build-campaign/campaign_rpc; SERVE=$FFI/poc/rust/serve.sh
CUR=$(dirname "$(ldd "$EXE" | grep -o '/[^ ]*libak_core\.so')")
ENVX="GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432"
LOG=$OUT/runner.log
say() { echo "$*" | tee -a "$LOG"; }
UNITS=("cur|A|" "cur|D-retain|" "cur|Cf-retain|" "cur|Cf-q-retain|" "stk|Cf-zc-retain|AK_SPARES=6 AK_SPARE_LOCK=1")
WLS=("d16k1 d 16MiB 1 120 12" "d16k8 d 16MiB 8 20 3" "d4k1 d 4MiB 1 400 40" "c54k1 c P5.4 1 400 40" "c54k8 c P5.4 8 50 6")
SCR=$(mktemp -d)
trap '[ -f "$SCR/serve.state" ] && bash "$SERVE" stop > /dev/null 2>&1; rm -rf "$SCR"' EXIT
taskset -c "$OSSET" gcc -O2 -shared -fPIC -o "$SCR/ncpus.so" gen/ncpus_shim.c -ldl || exit 1
taskset -c "$OSSET" bash "$SERVE" build > "$SCR/sb.log" 2>&1 || { cat "$SCR/sb.log"; exit 1; }
export AK_SERVE_STATE=$SCR/serve.state
AK_SERVER_THREADS=$WK AK_SERVER_TCP=0 bash "$SERVE" start --out "$SCR/srv" > "$SCR/srv.out" 2>&1 || { cat "$SCR/srv.out"; exit 1; }
bash "$SERVE" warm 64 > "$SCR/warm.log" 2>&1 || { cat "$SCR/warm.log"; exit 1; }
SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE"); TCPA=$(sed -n 's/^tcp //p' "$AK_SERVE_STATE"); SPID=$(sed -n 's/^pid //p' "$AK_SERVE_STATE")
[ -n "$TCPA" ] || { say "REFUSED: the server reports no TCP listener"; exit 2; }
rm -f "$SCR/sctl" "$SCR/sack"; mkfifo "$SCR/sctl" "$SCR/sack"
facts() { python3 gen/machine_facts.py "$AK_CPU_CLIENT" "$AK_CPU_SERVER" "$SPID"; }
aff_ok() {
  local sa; sa=$(sed -n 's/^Cpus_allowed_list:[[:space:]]*//p' /proc/$SPID/status 2>/dev/null)
  python3 -c "import sys
def r(s):
    o=set()
    for p in s.split(','):
        if '-' in p: a,b=p.split('-'); o.update(range(int(a),int(b)+1))
        elif p: o.add(int(p))
    return o
sys.exit(0 if r(sys.argv[1])==r(sys.argv[2]) else 1)" "$sa" "$AK_CPU_SERVER" || { say "ABORTED: the server's affinity is '$sa', not $AK_CPU_SERVER"; exit 3; }
}
{ echo "# tcp_uds: commit $(git -C "$FFI" rev-parse --short HEAD)$(git -C "$FFI" status --porcelain -- poc/cpp/src poc/cpp/gen poc/codec | grep -q . && echo ' + UNCOMMITTED'), $(date -u +%FT%TZ), rounds $ROUNDS"
  echo "# exe $EXE sha256 $(sha256sum "$EXE" | cut -c1-16); arm cur: $CUR/libak_core.so sha256 $(sha256sum "$CUR/libak_core.so" | cut -c1-16); arm stk: $STK/libak_core.so sha256 $(sha256sum "$STK/libak_core.so" | cut -c1-16)"
  echo "# server pid $SPID: $(cat "$SCR/srv/rpc-server.log" | tr '\n' ' ')"
  echo "# endpoints: uds unix:$SOCK (pinned); tcp grpc++ ipv4:$TCPA, core http://$TCPA (pinned opts, tcp_nagle 0)"
  echo "# units: ${UNITS[*]}; workloads: ${WLS[*]}; every process: $ENVX, core --workers $WK, grpc-core sysconf $GCPUS"
  echo "# machine_start $(facts)"; } >> "$LOG"
run() {  # run TR ARM CELL DIRS PAY K N FILE [strace]
  local tr=$1 arm=$2 cell=$3 dirs=$4 pay=$5 k=$6 n=$7 f=$8 st=${9:-} lib=$CUR knobs="" tgt ct pre=() spf=""
  [ "$arm" = stk ] && lib=$STK
  [ "$cell" = Cf-zc-retain ] && knobs="AK_SPARES=6 AK_SPARE_LOCK=1"
  if [ "$tr" = tcp ]; then tgt="ipv4:$TCPA"; ct="http://$TCPA"; else tgt="unix:$SOCK"; ct="unix:$SOCK"; fi
  aff_ok
  local ctl=()
  if [ -n "$st" ]; then
    pre=(strace -f -qq -yy -s 16 -e signal=none -o "$SCR/st.txt" \
      -e trace=write,writev,sendmsg,sendto,read,readv,recvmsg,recvfrom,futex,epoll_wait,epoll_pwait,epoll_pwait2,mmap,munmap,madvise,brk,sched_yield,poll,ppoll)
  else
    taskset -c "$OSSET" perf stat -p "$SPID" -D -1 --control "fifo:$SCR/sctl,$SCR/sack" -x, -o "$f.server.perfstat" \
      -e task-clock,cycles,instructions,context-switches > /dev/null 2>&1 & spf=$!
    ctl=(--perf-ctl "$SCR/sctl,$SCR/sack")
  fi
  taskset -c "$AK_CPU_CLIENT" "${pre[@]}" env LD_LIBRARY_PATH="$lib" $knobs $ENVX AK_SERVER_PID="$SPID" LD_PRELOAD="$SCR/ncpus.so" \
    AK_SHIM_NCPUS=$GCPUS "$EXE" --target "$tgt" --core-target "$ct" --expect 540422 --transport pinned --cells "$cell" \
    --dirs "$dirs" --payloads "$pay" --inflight "$k" --workers $WK --profile "$n" --profile-chunks $([ -n "$st" ] && echo 1 || echo 10) \
    "${ctl[@]}" > "$f" 2>&1 || { tail -3 "$f"; say "FAILED $f"; [ -n "$spf" ] && kill -INT "$spf"; exit 1; }
  if [ -n "$spf" ]; then kill -INT "$spf" 2> /dev/null; wait "$spf" 2> /dev/null; fi
}
T0=$(date +%s); N=${#UNITS[@]}; seq_no=0
for r in $(seq 1 "$ROUNDS"); do
  wi=0
  for w in "${WLS[@]}"; do
    set -- $w; name=$1; dirs=$2; pay=$3; k=$4; n=$5
    for i in $(seq 0 $((N - 1))); do
      u=${UNITS[$(( (i + r + wi) % N ))]}
      arm=${u%%|*}; rest=${u#*|}; cell=${rest%%|*}
      [ "$cell" = Cf-zc-retain ] && [ "$dirs" != d ] && continue
      if [ $(( (r + i + wi) % 2 )) = 0 ]; then order="uds tcp"; else order="tcp uds"; fi
      for tr in $order; do
        seq_no=$((seq_no + 1))
        run "$tr" "$arm" "$cell" "$dirs" "$pay" "$k" "$n" "$OUT/proc/$(printf '%04d' $seq_no)-r$r-$name-$tr-$arm-$cell.out"
      done
    done
    wi=$((wi + 1))
  done
  echo "# machine_after_round $r $(date -u +%FT%TZ) $(facts)" >> "$LOG"
  say "  round $r done, $(( $(date +%s) - T0 )) s"
done
say "timed rounds: $(( $(date +%s) - T0 )) s, $seq_no processes"
T1=$(date +%s)
for w in "${WLS[@]}"; do
  set -- $w; name=$1; dirs=$2; pay=$3; k=$4; ns=$6
  for u in "${UNITS[@]}"; do
    arm=${u%%|*}; rest=${u#*|}; cell=${rest%%|*}
    [ "$cell" = Cf-zc-retain ] && [ "$dirs" != d ] && continue
    for tr in uds tcp; do
      f=$OUT/strace/$name-$tr-$arm-$cell
      run "$tr" "$arm" "$cell" "$dirs" "$pay" "$k" "$ns" "$f.out" strace
      python3 gen/strace_window.py "$SCR/st.txt" > "$f.syscalls.txt"; gzip -9c "$SCR/st.txt" > "$f.strace.gz"
    done
  done
done
say "strace: $(( $(date +%s) - T1 )) s"
echo "# machine_end $(facts)" >> "$LOG"
cp "$SCR/srv/rpc-server.log" "$OUT/rpc-server.log" 2>/dev/null
python3 gen/tcp_uds_tables.py "$OUT" > "$OUT/tables.md" && say "tables: $OUT/tables.md"
