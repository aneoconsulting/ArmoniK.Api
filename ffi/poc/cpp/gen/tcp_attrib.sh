#!/usr/bin/env bash
# Attribution of the TCP inversion (owner, 2026-10-01). One server (poc/rust/serve.sh,
# AK_SERVER_TCP=0: the pinned configuration on the Unix socket and on 127.0.0.1, TCP_NODELAY on
# accept, 8 workers), one-cell `campaign_rpc --profile` processes, each phase one bench-lock block
# with its own server; the server's affinity checked before every process.
#
#   gen/tcp_attrib.sh OUT_DIR PHASE CUR_STACK_CORE CTL_CORE H16_CORE
#
#   perf  d/16 k=1 and 8; cells A, D-retain, Cf-retain (this tree's core), Cf-zc-retain (CUR_STACK_CORE,
#         the p1-p7 stack, 6 + lock); uds and tcp: per process perf record -e cycles --call-graph lbr on
#         the client AND perf record -p on the server, both enabled by the client around its loop
#         (gen/perf_net_attrib.py splits the kernel cycles by network path)
#   wall  the same cells and workloads, no perf: (a) one process per cell and transport with an `ss -tin`
#         sampler every 5 ms on the server's port during the process (TCP; ss.txt), the profile's batch
#         trace (per-call durations); (b) one process under strace -f -T (writev time, EAGAIN, epoll and
#         futex per thread: gen/strace_threads.py)
#   cpu   A, D-retain, Cf-retain, Cf-zc-retain; d/16 k=1 and 8, d/4 k=1, c/P5.4 k=1; uds and tcp; 2 rounds;
#         per process the client's process clock AND perf stat task-clock and cycles (softirq-inclusive),
#         the server's perf stat -p, the loop's irq/softirq time on both CPU sets
#   h2    A, D (H2_CTL), Cf, Cf-q (ring 24), Cf-zc on each of H2_CTL, H2_H16 (AK_H2_COALESCE=16), H2_PR; d/16 k=1
#         and 8, d/4 k=1, c/P5.4 k=1 and 8 (Cf-zc d only); uds and tcp; H2_ROUNDS rounds; as `cpu`, plus strace
#   sweep TCP only: A, D once; Cf, Cf-q per h2 variant (SW_STOCK, SW_BATCH at AK_H2_COALESCE=16) and core
#         --workers in SW_WORKERS; SW_WLS, SW_ROUNDS; client and server perf stat, irq_time; strace for writes
#   p4    A (ctl core arm), Cf with CTL_CORE (p1-p3, crates.io h2, AK_H2_COALESCE=1) and Cf with H16_CORE
#         (p1-p3 + p4 patched h2, AK_H2_COALESCE=16), ring 6 + lock; uds and tcp; d/16 k=1 and 8, d/4
#         k=1, c/P5.4 k=1; 3 rounds, UDS and TCP back to back (alternating order), perf stat -p on the
#         server around each loop; then one strace process per unit, workload and transport
# Every TCP process must report TCP_NODELAY = 1 on every client TCP socket (refused otherwise). The
# zero-copy cells only with TA_ZC=1. The core's --workers is 8 except in the sweep (RUN_WK).
# Every process: GLIBC_TUNABLES trim 256 MiB / mmap 32 MiB, core --workers 8, ncpus_shim 8, pinned.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
FFI=$(cd "$HERE/../.." && pwd)
. gen/net_target.sh
OUT=${1:?usage}; PHASE=${2:?phase}; STK=${3:?}; CTL=${4:?}; H16=${5:?}
mkdir -p "$OUT/$PHASE"; OUT=$(cd "$OUT" && pwd); STK=$(cd "$STK" && pwd); CTL=$(cd "$CTL" && pwd); H16=$(cd "$H16" && pwd)
if [ "${TA_LOCKED:-}" != 1 ]; then
  t=$(date +%s); TA_LOCKED=1 flock /tmp/ak-physical-bench.lock bash "$0" "$@"; rc=$?
  echo "phase $PHASE: $(( $(date +%s) - t )) s (lock held), exit $rc" | tee -a "$OUT/runner.log"; exit $rc
fi
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1-4,11-14} AK_CPU_SERVER=${AK_CPU_SERVER:-5-8,15-18}
OSSET=${AK_CPU_OS:-0,9,10,19}; WK=8; GCPUS=8
EXE=$HERE/build-campaign/campaign_rpc; SERVE=$FFI/poc/rust/serve.sh
CUR=$(dirname "$(ldd "$EXE" | grep -o '/[^ ]*libak_core\.so')")
ENVX="GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432"
LOG=$OUT/runner.log
say() { echo "$*" | tee -a "$LOG"; }
SCR=$(mktemp -d)
trap '[ -n "${SSP:-}" ] && kill $SSP 2>/dev/null; [ -f "$SCR/serve.state" ] && bash "$SERVE" stop > /dev/null 2>&1; rm -rf "$SCR"' EXIT
taskset -c "$OSSET" gcc -O2 -shared -fPIC -o "$SCR/ncpus.so" gen/ncpus_shim.c -ldl || exit 1
export AK_SERVE_STATE=$SCR/serve.state
AK_SERVER_THREADS=$WK AK_SERVER_TCP=0 bash "$SERVE" start --out "$SCR/srv" > "$SCR/srv.out" 2>&1 || { cat "$SCR/srv.out"; exit 1; }
bash "$SERVE" warm 64 > "$SCR/warm.log" 2>&1 || { cat "$SCR/warm.log"; exit 1; }
SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE"); TCPA=$(sed -n 's/^tcp //p' "$AK_SERVE_STATE"); SPID=$(sed -n 's/^pid //p' "$AK_SERVE_STATE")
PORT=${TCPA##*:}
rm -f "$SCR/ctl" "$SCR/ack" "$SCR/sctl" "$SCR/sack"; mkfifo "$SCR/ctl" "$SCR/ack" "$SCR/sctl" "$SCR/sack"
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
{ echo "# tcp_attrib $PHASE: commit $(git -C "$FFI" rev-parse --short HEAD)$(git -C "$FFI" status --porcelain -- poc/cpp/src poc/cpp/gen poc/codec | grep -q . && echo ' + UNCOMMITTED'), $(date -u +%FT%TZ)"
  echo "# exe $EXE sha256 $(sha256sum "$EXE" | cut -c1-16); cores: cur $(sha256sum "$CUR/libak_core.so" | cut -c1-16), stk $(sha256sum "$STK/libak_core.so" | cut -c1-16), ctl $(sha256sum "$CTL/libak_core.so" | cut -c1-16), h16 $(sha256sum "$H16/libak_core.so" | cut -c1-16)"
  [ -n "${TA_NOTE:-}" ] && echo "# NOTE: $TA_NOTE"
  echo "# IRQs with effective affinity on the measured CPUs: $(for d in /proc/irq/[0-9]*; do e=$(cat $d/effective_affinity_list 2>/dev/null); [ -n "$e" ] && echo "$(basename $d):$e"; done | python3 -c "import sys
want=set()
for p in (sys.argv[1]+','+sys.argv[2]).split(','):
    a,_,b=p.partition('-'); want.update(range(int(a),int(b or a)+1))
out=[]
for l in sys.stdin.read().split():
    irq,_,cpus=l.partition(':'); s=set()
    for p in cpus.split(','):
        if p: a,_,b=p.partition('-'); s.update(range(int(a),int(b or a)+1))
    if s & want: out.append(l)
print(len(out), ' '.join(out))" "$AK_CPU_CLIENT" "$AK_CPU_SERVER")"
  [ -n "${SW_STOCK:-}" ] && for v in SW_STOCK SW_BATCH; do echo "# $v ${!v}: core sha256 $(sha256sum "${!v}/libak_core.so" | cut -c1-16)"; done
  [ -n "${H2_CTL:-}" ] && for v in H2_CTL H2_H16 H2_PR ${H2_COMB:+H2_COMB}; do echo "# $v ${!v}: core sha256 $(sha256sum "${!v}/libak_core.so" | cut -c1-16)"; done
  echo "# server pid $SPID: $(tr '\n' ' ' < "$SCR/srv/rpc-server.log")"
  echo "# endpoints: uds unix:$SOCK; tcp ipv4:$TCPA (grpc++) and http://$TCPA (core, tcp_nagle 0)"
  echo "# sysctl: tcp_rmem $(cat /proc/sys/net/ipv4/tcp_rmem | tr '\t' ' '); tcp_wmem $(cat /proc/sys/net/ipv4/tcp_wmem | tr '\t' ' '); tcp_autocorking $(cat /proc/sys/net/ipv4/tcp_autocorking); tcp_limit_output_bytes $(cat /proc/sys/net/ipv4/tcp_limit_output_bytes 2>/dev/null)"
  echo "# kernel: $(uname -r); $(zcat /proc/config.gz 2>/dev/null | grep -E '^CONFIG_(IRQ_TIME_ACCOUNTING|VIRT_CPU_ACCOUNTING_GEN|HZ)=' | tr '\n' ' ')(IRQ_TIME_ACCOUNTING: softirq time is charged to no task; the client's CLOCK_PROCESS_CPUTIME_ID and getrusage miss it, perf stat task-clock and cycles do not)"
  echo "# netfilter (read-only): modules $(lsmod | awk 'NR>1 && $1 ~ /^(nf|nft|xt|ipt|ip6t|ip_set|br_netfilter|bridge|x_tables|iptable|ip6table)/ {printf "%s ", $1}'); nf_conntrack_count $(cat /proc/sys/net/netfilter/nf_conntrack_count 2>&1); bridge-nf-call-iptables $(cat /proc/sys/net/bridge/bridge-nf-call-iptables 2>&1); ruleset: nft $(command -v nft > /dev/null && (nft list ruleset 2>&1 | wc -l) || echo 'not installed'), iptables-save $(command -v iptables-save > /dev/null && (iptables-save 2>&1 | head -1) || echo 'not installed')"
  echo "# machine $(python3 gen/machine_facts.py "$AK_CPU_CLIENT" "$AK_CPU_SERVER" "$SPID")"; } >> "$LOG"
# run TR LIB KNOBS CELL DIRS PAY K N FILE MODE   (MODE: plain | stat | record | strace | ss)
run() {
  local tr=$1 lib=$2 knobs=$3 cell=$4 dirs=$5 pay=$6 k=$7 n=$8 f=$9 mode=${10} tgt ct pre=() ctl=() spf=""
  if [ "$tr" = tcp ]; then tgt="ipv4:$TCPA"; ct="http://$TCPA"; else tgt="unix:$SOCK"; ct="unix:$SOCK"; fi
  aff_ok
  case "$mode" in
    stat) pre=(perf stat -D -1 --control "fifo:$SCR/ctl,$SCR/ack" -x, -o "$f.client.perfstat"
            -e task-clock,cycles,cycles:u,cycles:k,context-switches --)
          taskset -c "$OSSET" perf stat -p "$SPID" -D -1 --control "fifo:$SCR/sctl,$SCR/sack" -x, -o "$f.server.perfstat" \
            -e task-clock,cycles,instructions,context-switches > /dev/null 2>&1 & spf=$!
          ctl=(--perf-ctl "$SCR/ctl,$SCR/ack;$SCR/sctl,$SCR/sack") ;;
    record) taskset -c "$OSSET" perf record -m 8 -p "$SPID" -D -1 --control "fifo:$SCR/sctl,$SCR/sack" -e cycles -F 4000 \
            --call-graph lbr -o "$f.server.data" > /dev/null 2>&1 & spf=$!
          pre=(perf record -m 64 -D -1 --control "fifo:$SCR/ctl,$SCR/ack" -e cycles -F 4000 --call-graph lbr -o "$f.data" --)
          ctl=(--perf-ctl "$SCR/ctl,$SCR/ack;$SCR/sctl,$SCR/sack") ;;
    strace) pre=(strace -f -qq -T -yy -s 16 -e signal=none -o "$SCR/st.txt"
            -e trace=writev,sendmsg,write,sendto,recvfrom,recvmsg,read,futex,epoll_wait,epoll_pwait,epoll_pwait2) ;;
    ss) ( while :; do echo "@ $(date +%s.%N)"; ss -tinmH "( sport = :$PORT or dport = :$PORT )" 2>/dev/null; sleep 0.005; done ) > "$f.ss.txt" &
        SSP=$! ;;
  esac
  taskset -c "$AK_CPU_CLIENT" "${pre[@]}" env LD_LIBRARY_PATH="$lib" $knobs $ENVX AK_SERVER_PID="$SPID" LD_PRELOAD="$SCR/ncpus.so" \
    AK_SHIM_NCPUS=$GCPUS "$EXE" --target "$tgt" --core-target "$ct" --expect 540422 --transport pinned --cells "$cell" \
    --dirs "$dirs" --payloads "$pay" --inflight "$k" --workers ${RUN_WK:-$WK} --profile "$n" --profile-chunks $([ "$mode" = strace ] && echo 1 || echo 10) \
    "${ctl[@]}" > "$f.out" 2>&1 || { tail -3 "$f.out"; say "FAILED $f"; [ -n "$spf" ] && kill -INT "$spf"; exit 1; }
  if [ -n "$spf" ]; then kill -INT "$spf" 2> /dev/null; wait "$spf" 2> /dev/null; fi
  # Nagle off, verified: every TCP socket of the process read TCP_NODELAY = 1 (gen/net_target.sh)
  if [ "$tr" = tcp ] && ! AK_NET=tcp net_nodelay_ok "$f.out"; then say "REFUSED $f: a client TCP socket without TCP_NODELAY, or none"; exit 1; fi
  if [ "$mode" = ss ]; then kill $SSP 2>/dev/null; wait $SSP 2>/dev/null; SSP=""; gzip -9 "$f.ss.txt"; fi
  if [ "$mode" = strace ]; then
    local calls; calls=$(python3 -c "import json,sys
for l in open(sys.argv[1]):
    if l.startswith('{\"profile\"'): print(json.loads(l)['profile']['calls'])" "$f.out")
    python3 gen/strace_threads.py "$SCR/st.txt" "$calls" 50 > "$f.threads.json"
    python3 gen/strace_window.py "$SCR/st.txt" > "$f.syscalls.txt"; gzip -9c "$SCR/st.txt" > "$f.strace.gz"
  fi
}
# The zero-copy and deferred cells are retired from the default sets (owner, 2026-10-01: zero copy is not
# pursued; they refuse on the core): TA_ZC=1 brings Cf-zc back into the perf, wall, cpu and h2 phases.
CELLS1=("A|$CUR|" "D-retain|$CUR|" "Cf-retain|$CUR|")
[ "${TA_ZC:-}" = 1 ] && CELLS1+=("Cf-zc-retain|$STK|AK_SPARES=6 AK_SPARE_LOCK=1")
WL1=("d16k1 d 16MiB 1 120 12" "d16k8 d 16MiB 8 20 3")
T0=$(date +%s)
case "$PHASE" in
  perf|wall)
    for w in "${WL1[@]}"; do
      set -- $w; name=$1; dirs=$2; pay=$3; k=$4; n=$5; ns=$6
      for u in "${CELLS1[@]}"; do
        cell=${u%%|*}; rest=${u#*|}; lib=${rest%%|*}; knobs=${rest#*|}
        for tr in uds tcp; do
          f=$OUT/$PHASE/$name-$tr-$cell
          if [ "$PHASE" = perf ]; then run "$tr" "$lib" "$knobs" "$cell" "$dirs" "$pay" "$k" "$(( n / 2 ))" "$f" record
          else
            run "$tr" "$lib" "$knobs" "$cell" "$dirs" "$pay" "$k" "$n" "$f" ss
            run "$tr" "$lib" "$knobs" "$cell" "$dirs" "$pay" "$k" "$ns" "$f.st" strace
          fi
        done
      done
      say "  $PHASE $name done, $(( $(date +%s) - T0 )) s"
    done ;;
  cpu)
    # A, D, Cf, Cf-zc on both transports with both CPU measures per process: the process clock (chunks)
    # and perf stat task-clock / cycles on the client; perf stat -p on the server; the loop's irq and
    # softirq time on the client's and the server's CPUs (/proc/stat, /proc/softirqs; profile irq_time)
    WL2=("d16k1 d 16MiB 1 120 12" "d16k8 d 16MiB 8 20 3" "d4k1 d 4MiB 1 400 40" "c54k1 c P5.4 1 400 40")
    seq_no=0
    for r in 1 2; do
      wi=0
      for w in "${WL2[@]}"; do
        set -- $w; name=$1; dirs=$2; pay=$3; k=$4; n=$5
        for i in 0 1 2 3; do
          j=$(( (i + r + wi) % 4 )); u=${CELLS1[$j]}
          cell=${u%%|*}; rest=${u#*|}; lib=${rest%%|*}; knobs=${rest#*|}
          [ "$cell" = Cf-zc-retain ] && [ "$dirs" != d ] && continue  # the zero-copy cells run direction d only
          if [ $(( (r + i + wi) % 2 )) = 0 ]; then order="uds tcp"; else order="tcp uds"; fi
          for tr in $order; do
            seq_no=$((seq_no + 1))
            run "$tr" "$lib" "$knobs" "$cell" "$dirs" "$pay" "$k" "$n" "$OUT/cpu/$(printf '%04d' $seq_no)-r$r-$name-$tr-$cell" stat
          done
        done
        wi=$((wi + 1))
      done
      say "  cpu round $r done, $(( $(date +%s) - T0 )) s"
    done ;;
  h2|h2b)
    # the h2 comparison (owner, 2026-10-01): cores H2_CTL (crates.io h2), H2_H16 (p4, AK_H2_COALESCE=16),
    # H2_PR (h2 PR #903 port), all on one stack (env; gen/h2_variants_build.sh). A and D on H2_CTL (neither
    # uses the core transport); Cf (ring 6 + lock), Cf-q (ring 24 + lock), Cf-zc (ring 6 + lock, d only) on
    # each core. H2_ROUNDS rounds (default 3), unit order rotated per round and workload, UDS and TCP back
    # to back (order alternating); client perf stat + server perf stat -p + irq_time per process; then one
    # strace process per unit, workload and transport (writes and syscalls per call).
    : "${H2_CTL:?}" "${H2_H16:?}" "${H2_PR:?}"
    R6="AK_SPARES=6 AK_SPARE_LOCK=1"; R24="AK_SPARES=24 AK_SPARE_LOCK=1"
    UNITS=("A|$H2_CTL|AK_H2_COALESCE=1|A" "D-retain|$H2_CTL|AK_H2_COALESCE=1|D")
    COS=("ctl|$H2_CTL|AK_H2_COALESCE=1" "h16|$H2_H16|AK_H2_COALESCE=16" "pr903|$H2_PR|AK_H2_COALESCE=1")
    # H2_COMB: a fourth core (PR #903 and p4 combined), run with AK_H2_COALESCE=${H2_COMB_COALESCE:-16}
    [ -n "${H2_COMB:-}" ] && COS+=("pr903p4|$H2_COMB|AK_H2_COALESCE=${H2_COMB_COALESCE:-16}")
    for co in "${COS[@]}"; do
      cn=${co%%|*}; r=${co#*|}; cd_=${r%%|*}; ck=${r#*|}
      UNITS+=("Cf-retain|$cd_|$ck $R6|Cf-$cn" "Cf-q-retain|$cd_|$ck $R24|Cf-q-$cn")
      [ "${TA_ZC:-}" = 1 ] && UNITS+=("Cf-zc-retain|$cd_|$ck $R6|Cf-zc-$cn")
    done
    WL2=("d16k1 d 16MiB 1 100 12" "d16k8 d 16MiB 8 16 3" "d4k1 d 4MiB 1 300 40" "c54k1 c P5.4 1 300 40" "c54k8 c P5.4 8 50 8")
    NU=${#UNITS[@]}; seq_no=0
    { echo "# h2 units:"; for u in "${UNITS[@]}"; do echo "#   ${u##*|}: cell ${u%%|*}, core ${u#*|}"; done; } >> "$LOG"
    for r in $(seq 1 "${H2_ROUNDS:-3}"); do
      wi=0
      for w in "${WL2[@]}"; do
        set -- $w; name=$1; dirs=$2; pay=$3; k=$4; n=$5
        for i in $(seq 0 $((NU - 1))); do
          u=${UNITS[$(( (i + r * 3 + wi) % NU ))]}
          cell=${u%%|*}; rest=${u#*|}; lib=${rest%%|*}; rest=${rest#*|}; knobs=${rest%|*}; un=${u##*|}
          [ "$cell" = Cf-zc-retain ] && [ "$dirs" != d ] && continue
          if [ $(( (r + i + wi) % 2 )) = 0 ]; then order="uds tcp"; else order="tcp uds"; fi
          for tr in $order; do
            seq_no=$((seq_no + 1))
            run "$tr" "$lib" "$knobs" "$cell" "$dirs" "$pay" "$k" "$n" "$OUT/$PHASE/$(printf '%04d' $seq_no)-r$r-$name-$tr-$un" stat
          done
        done
        wi=$((wi + 1))
      done
      say "  $PHASE round $r done, $(( $(date +%s) - T0 )) s"
    done
    for w in "${WL2[@]}"; do
      set -- $w; name=$1; dirs=$2; pay=$3; k=$4; ns=$6
      for u in "${UNITS[@]}"; do
        cell=${u%%|*}; rest=${u#*|}; lib=${rest%%|*}; rest=${rest#*|}; knobs=${rest%|*}; un=${u##*|}
        [ "$cell" = Cf-zc-retain ] && [ "$dirs" != d ] && continue
        [[ "$un" =~ ^(${H2_STRACE:-.*})$ ]] || continue  # H2_STRACE: the units to strace (regex; default all)
        for tr in uds tcp; do run "$tr" "$lib" "$knobs" "$cell" "$dirs" "$pay" "$k" "$ns" "$OUT/$PHASE/strace-$name-$tr-$un" strace; done
      done
    done ;;
  sweep)
    # The TCP worker sweep (owner, 2026-10-01): the core's --workers W in SW_WORKERS (default 1 2 4 8) by h2
    # variant {stock: SW_STOCK, h2-batch: SW_BATCH with AK_H2_COALESCE=16}; cells Cf (SW_CF_KNOBS) and Cf-q
    # (SW_CFQ_KNOBS) per variant and W; A and D once per workload and round (grpc++'s transport: neither the h2
    # variant nor W is on their path; stock core, W = 8). Transport: TCP only (AK_NET=uds is refused here).
    # Workloads SW_WLS (default d16k1 d16k8 c54k1 c54k8; d16k16 c54k16 available), SW_ROUNDS rounds (default 3),
    # unit order rotated per round and workload; client perf stat (task-clock), server perf stat -p, irq_time;
    # then one strace process per unit, workload and W in SW_STRACE_WORKERS (default 1 8) for writes per call.
    : "${SW_STOCK:?}" "${SW_BATCH:?}"
    [ "$AK_NET" = tcp ] || { say "REFUSED: the sweep is TCP only"; exit 1; }
    CFK=${SW_CF_KNOBS-AK_SPARES=6 AK_SPARE_LOCK=1}; CFQK=${SW_CFQ_KNOBS-AK_SPARES=24 AK_SPARE_LOCK=1}
    UNITS=("A|$SW_STOCK|AK_H2_COALESCE=1|A|$WK" "D-retain|$SW_STOCK|AK_H2_COALESCE=1|D|$WK")
    for v in "stock|$SW_STOCK|AK_H2_COALESCE=1" "batch|$SW_BATCH|AK_H2_COALESCE=16"; do
      vn=${v%%|*}; r=${v#*|}; vd=${r%%|*}; vk=${r#*|}
      for wk in ${SW_WORKERS:-1 2 4 8}; do
        UNITS+=("Cf-retain|$vd|$vk $CFK|Cf-$vn-w$wk|$wk" "Cf-q-retain|$vd|$vk $CFQK|Cf-q-$vn-w$wk|$wk")
      done
    done
    declare -A WLD=([d16k1]="d 16MiB 1 100 12" [d16k8]="d 16MiB 8 16 3" [d16k16]="d 16MiB 16 8 2"
                    [c54k1]="c P5.4 1 300 40" [c54k8]="c P5.4 8 50 8" [c54k16]="c P5.4 16 25 4")
    NU=${#UNITS[@]}; seq_no=0
    { echo "# sweep units (cell|core|knobs|name|core workers):"; for u in "${UNITS[@]}"; do echo "#   $u"; done
      echo "# sweep workloads: ${SW_WLS:-d16k1 d16k8 c54k1 c54k8}; rounds ${SW_ROUNDS:-3}; strace at W in ${SW_STRACE_WORKERS:-1 8}"; } >> "$LOG"
    unit() { cell=${1%%|*}; local r=${1#*|}; lib=${r%%|*}; r=${r#*|}; knobs=${r%%|*}; r=${r#*|}; un=${r%%|*}; uw=${r#*|}; }
    for r in $(seq 1 "${SW_ROUNDS:-3}"); do
      wi=0
      for name in ${SW_WLS:-d16k1 d16k8 c54k1 c54k8}; do
        set -- ${WLD[$name]}; dirs=$1; pay=$2; k=$3; n=$4
        for i in $(seq 0 $((NU - 1))); do
          unit "${UNITS[$(( (i + r * 5 + wi * 3) % NU ))]}"
          seq_no=$((seq_no + 1))
          RUN_WK=$uw run tcp "$lib" "$knobs" "$cell" "$dirs" "$pay" "$k" "$n" "$OUT/$PHASE/$(printf '%04d' $seq_no)-r$r-$name-tcp-$un" stat
        done
        wi=$((wi + 1))
      done
      say "  sweep round $r done, $(( $(date +%s) - T0 )) s"
    done
    for name in ${SW_WLS:-d16k1 d16k8 c54k1 c54k8}; do
      set -- ${WLD[$name]}; dirs=$1; pay=$2; k=$3; ns=$5
      for u in "${UNITS[@]}"; do
        unit "$u"
        [[ " ${SW_STRACE_WORKERS:-1 8} " == *" $uw "* ]] || [[ "$un" =~ ^(A|D)$ ]] || continue
        RUN_WK=$uw run tcp "$lib" "$knobs" "$cell" "$dirs" "$pay" "$k" "$ns" "$OUT/$PHASE/strace-$name-tcp-$un" strace
      done
    done ;;
  p4)
    UNITS=("A|$CTL|AK_H2_COALESCE=1" "Cf-retain|$CTL|AK_H2_COALESCE=1 AK_SPARES=6 AK_SPARE_LOCK=1" "Cf-retain|$H16|AK_H2_COALESCE=16 AK_SPARES=6 AK_SPARE_LOCK=1")
    NAMES=("A-ctl" "Cf-ctl" "Cf-h16")
    WL2=("d16k1 d 16MiB 1 120 12" "d16k8 d 16MiB 8 20 3" "d4k1 d 4MiB 1 400 40" "c54k1 c P5.4 1 400 40")
    seq_no=0
    for r in 1 2 3; do
      wi=0
      for w in "${WL2[@]}"; do
        set -- $w; name=$1; dirs=$2; pay=$3; k=$4; n=$5
        for i in 0 1 2; do
          j=$(( (i + r + wi) % 3 )); u=${UNITS[$j]}; un=${NAMES[$j]}
          cell=${u%%|*}; rest=${u#*|}; lib=${rest%%|*}; knobs=${rest#*|}
          if [ $(( (r + i + wi) % 2 )) = 0 ]; then order="uds tcp"; else order="tcp uds"; fi
          for tr in $order; do
            seq_no=$((seq_no + 1))
            run "$tr" "$lib" "$knobs" "$cell" "$dirs" "$pay" "$k" "$n" "$OUT/p4/$(printf '%04d' $seq_no)-r$r-$name-$tr-$un" stat
          done
        done
        wi=$((wi + 1))
      done
      say "  p4 round $r done, $(( $(date +%s) - T0 )) s"
    done
    for w in "${WL2[@]}"; do
      set -- $w; name=$1; dirs=$2; pay=$3; k=$4; ns=$6
      for j in 0 1 2; do
        u=${UNITS[$j]}; un=${NAMES[$j]}; cell=${u%%|*}; rest=${u#*|}; lib=${rest%%|*}; knobs=${rest#*|}
        for tr in uds tcp; do run "$tr" "$lib" "$knobs" "$cell" "$dirs" "$pay" "$k" "$ns" "$OUT/p4/strace-$name-$tr-$un" strace; done
      done
    done ;;
  *) echo "phase?" >&2; exit 2 ;;
esac
say "$PHASE: $(( $(date +%s) - T0 )) s of benchmark"
