#!/usr/bin/env bash
# Several builds of the shared core compared from C++ (physical probe, owner 2026-09-30), in
# one-cell `campaign_rpc --profile` processes under a client perf stat (task-clock). Every arm runs THIS tree's
# campaign_rpc with LD_LIBRARY_PATH on the arm's libak_core.so (it precedes the binary's RUNPATH),
# so arms differ only in the core and in their knobs; runner.log names every core and its sha256.
#
#   gen/core_ab.sh OUT_DIR measure|default|strace ARM=CORE_DIR[:KNOB=V,KNOB=V] ...
#
#   measure  ROUNDS (3) rounds, every process with AB_ENV (the allocator setting of the main
#            figures); per round, workload and cell, the arms in rotated order
#   default  one round, AB_ENV NOT set (the default-allocator pass)
#   strace   one process per (workload, cell, arm) under strace -f -yy with AB_ENV: the syscalls
#            between the loop's markers (gen/strace_window.py), fewer batches
#   perf     the split only: per (workload, cell, arm) one process under perf stat (cycles and
#            instructions user/kernel, cache-references, cache-misses, LLC-load-misses, faults, context
#            switches) and one under perf record (cycles, LBR stacks), perf enabled around the loop only
#   AB_CELL_KNOBS  "CELL:K=V,K=V;CELL:..." extra knobs for one cell in every arm (the ring settings)
#   AB_WLS / AB_CELLS  narrow the workloads and cells
# Cells A, D-retain, Cf-retain, Cf-q-retain (AB_CELLS for others); workloads d/16MiB and d/4MiB at k=1
# and 8, c/P5.4 k=1 and 8 (AB_D_ONLY: cells skipped on c). Each phase holds /tmp/ak-physical-bench.lock with its own 8-worker server (poc/rust/serve.sh
# at HEAD); CLIENT 1-4,11-14, SERVER 5-8,15-18, core 8 workers, grpc-core sized for 8 (ncpus_shim).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
FFI=$(cd "$HERE/../.." && pwd)
. gen/net_target.sh
OUT=${1:?usage: gen/core_ab.sh OUT_DIR PHASE ARM=CORE_DIR[:K=V,...] ...}; PHASE=${2:?phase}; shift 2
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1-4,11-14} AK_CPU_SERVER=${AK_CPU_SERVER:-5-8,15-18}
OSSET=${AK_CPU_OS:-0,9,10,19}; ROUNDS=${ROUNDS:-3}; WK=8; GCPUS=8
EXE=$HERE/build-campaign/campaign_rpc; SERVE=$FFI/poc/rust/serve.sh; LOCK=/tmp/ak-physical-bench.lock
LOG=$OUT/runner.log
say() { echo "$*" | tee -a "$LOG"; }
if [ "${AB_LOCKED:-}" != 1 ]; then
  t=$(date +%s); AB_LOCKED=1 flock "$LOCK" bash "$0" "$OUT" "$PHASE" "$@" || exit 1
  say "phase $PHASE: $(( $(date +%s) - t )) s (lock held)"; exit 0
fi
ARMS=(); declare -A ACORE AKNOB
for a in "$@"; do
  n=${a%%=*}; rest=${a#*=}; d=${rest%%:*}; k=""; [ "$rest" != "$d" ] && k=${rest#*:}
  ARMS+=("$n"); ACORE[$n]=$(cd "$d" && pwd); AKNOB[$n]=${k//,/ }
done
WLS=("d16k1 d 16MiB 1 120 12" "d16k8 d 16MiB 8 20 3" "d4k1 d 4MiB 1 400 40" "d4k8 d 4MiB 8 60 8" "c54k1 c P5.4 1 400 40" "c54k8 c P5.4 8 50 6")
CELLS=(A D-retain Cf-retain Cf-q-retain)
if [ -n "${AB_WLS:-}" ]; then kk=(); for w in "${WLS[@]}"; do case " $AB_WLS " in *" ${w%% *} "*) kk+=("$w") ;; esac; done; WLS=("${kk[@]}"); fi
if [ -n "${AB_CELLS:-}" ]; then read -r -a CELLS <<< "$AB_CELLS"; fi
# AB_D_ONLY: cells that run direction d only (the deferred-encode cells), skipped on c workloads
d_only() { case " ${AB_D_ONLY:-} " in *" $1 "*) return 0 ;; esac; return 1; }
cellknobs() {  # the extra knobs of cell $1
  local e; IFS=';' read -r -a e <<< "${AB_CELL_KNOBS:-}"
  for x in "${e[@]}"; do [ "${x%%:*}" = "$1" ] && echo "${x#*:}" | tr ',' ' '; done
}
SCR=$(mktemp -d)
trap '[ -f "$SCR/serve.state" ] && bash "$SERVE" stop > /dev/null 2>&1; rm -rf "$SCR"' EXIT
taskset -c "$OSSET" gcc -O2 -shared -fPIC -o "$SCR/ncpus.so" gen/ncpus_shim.c -ldl || exit 1
export AK_SERVE_STATE=$SCR/serve.state
env $(net_server_env) AK_SERVER_THREADS=$WK bash "$SERVE" start --out "$SCR/srv" > "$SCR/srv.out" 2>&1 || { cat "$SCR/srv.out"; exit 1; }
bash "$SERVE" warm 64 > /dev/null 2>&1 || exit 1
net_endpoints "$AK_SERVE_STATE" || exit 1
ENVX=""; [ "$PHASE" != default ] && ENVX=${AB_ENV:-}
{ echo "# core_ab $PHASE: commit $(git -C "$FFI" rev-parse --short HEAD)$(git -C "$FFI" status --porcelain -- poc/cpp/src poc/cpp/gen poc/codec | grep -q . && echo ' + UNCOMMITTED'), $(date -u +%FT%TZ)"
  echo "# exe $EXE sha256 $(sha256sum "$EXE" | cut -c1-16); server pid $(sed -n 's/^pid //p' "$AK_SERVE_STATE"), $(head -2 "$SCR/srv/rpc-server.log" | tail -1 | sed 's/.*tokio/tokio/')"
  for n in "${ARMS[@]}"; do
    r=$(LD_LIBRARY_PATH=${ACORE[$n]} ldd "$EXE" | grep -o '/[^ ]*libak_core\.so')
    echo "# arm $n: core $r sha256 $(sha256sum "$r" | cut -c1-16); knobs: ${AKNOB[$n]:-none}"
    [ "$r" = "${ACORE[$n]}/libak_core.so" ] || { echo "REFUSED: arm $n resolves $r"; exit 1; }
  done
  echo "# endpoint: $NET_DESC; client CPU: perf stat task-clock around the loop in the measure and default phases (FILE.client.perfstat)"
  echo "# every process: ${ENVX:-no extra environment (default allocator)}; cell knobs: ${AB_CELL_KNOBS:-none}"
  echo "# workloads: ${WLS[*]}; cells: ${CELLS[*]} (direction d only: ${AB_D_ONLY:-none}); rounds: $ROUNDS"
  echo "# CLIENT $AK_CPU_CLIENT SERVER $AK_CPU_SERVER; server 8 workers; core --workers ${AB_CLIENT_WORKERS:-$WK}; grpc-core sysconf = $GCPUS (ncpus_shim); pinned; retain"
  echo "# machine $(python3 gen/machine_facts.py "$AK_CPU_CLIENT" "$AK_CPU_SERVER")"; } >> "$LOG"
grep -q REFUSED "$LOG" && exit 1
rm -f "$SCR/ctl" "$SCR/ack" "$SCR/sctl" "$SCR/sack"; mkfifo "$SCR/ctl" "$SCR/ack" "$SCR/sctl" "$SCR/sack"
run() {  # run ARM CELL DIRS PAY K N OUTFILE [strace|stat:FILE|record:FILE]
  local arm=$1 cell=$2 dirs=$3 pay=$4 k=$5 n=$6 f=$7 st=${8:-} pre=() ctl=()
  case "$st" in
    '') pre=(perf stat -D -1 --control "fifo:$SCR/ctl,$SCR/ack" -x, -o "${f%.out}.client.perfstat" -e task-clock,cycles,context-switches --)
      ctl=(--perf-ctl "$SCR/ctl,$SCR/ack") ;;
    strace) pre=(strace -f -qq -yy -s 24 -e signal=none -o "$SCR/st.txt" \
      -e trace=write,writev,sendmsg,sendto,read,readv,recvmsg,recvfrom,futex,epoll_wait,epoll_pwait,epoll_pwait2,mmap,munmap,madvise,mremap,brk,sched_yield,poll,ppoll,io_uring_enter) ;;
    stat:*) pre=(perf stat -D -1 --control "fifo:$SCR/ctl,$SCR/ack" -x, -o "${st#stat:}" \
      -e cycles:u,cycles:k,instructions:u,instructions:k,cache-references,cache-misses,LLC-load-misses,page-faults,context-switches --); ctl=(--perf-ctl "$SCR/ctl,$SCR/ack") ;;
    record:*) pre=(perf record -m 64 -D -1 --control "fifo:$SCR/ctl,$SCR/ack" -e cycles -F 4000 --call-graph lbr -o "${st#record:}" --)
      ctl=(--perf-ctl "$SCR/ctl,$SCR/ack") ;;
  esac
  # AB_SERVER_PERF=1 (perf phase): the same perf attached to the server process too (perf stat
  # -p / perf record -p, not ptrace), enabled by the client around the same loop
  local spf=""
  if [ "${AB_SERVER_PERF:-}" = 1 ] && [ ${#ctl[@]} -gt 0 ]; then
    local sp; sp=$(sed -n 's/^pid //p' "$AK_SERVE_STATE")
    case "$st" in
      stat:*) taskset -c "$OSSET" perf stat -p "$sp" -D -1 --control "fifo:$SCR/sctl,$SCR/sack" -x, -o "${st#stat:}.server" \
                -e cycles:u,cycles:k,instructions:u,instructions:k,cache-misses,context-switches > /dev/null 2>&1 & spf=$! ;;
      record:*) taskset -c "$OSSET" perf record -m 8 -p "$sp" -D -1 --control "fifo:$SCR/sctl,$SCR/sack" -e cycles -F 4000 --call-graph lbr \
                -o "${st#record:}.server" > /dev/null 2>&1 & spf=$! ;;
    esac
    ctl=(--perf-ctl "$SCR/ctl,$SCR/ack;$SCR/sctl,$SCR/sack")
  fi
  taskset -c "$AK_CPU_CLIENT" "${pre[@]}" env LD_LIBRARY_PATH="${ACORE[$arm]}" ${AKNOB[$arm]} $(cellknobs "$cell") $ENVX \
    AK_SERVER_PID="$(sed -n 's/^pid //p' "$AK_SERVE_STATE")" LD_PRELOAD="$SCR/ncpus.so" AK_SHIM_NCPUS=$GCPUS "$EXE" --target "$NET_TGT" --core-target "$NET_CTGT" --expect 540422 --transport pinned \
    --cells "$cell" --dirs "$dirs" --payloads "$pay" --inflight "$k" --workers ${AB_CLIENT_WORKERS:-$WK} --profile "$n" \
    --profile-chunks $([ "$st" = strace ] && echo 1 || echo 10) "${ctl[@]}" > "$f" 2>&1 || { tail -3 "$f"; say "FAILED $f"; [ -n "$spf" ] && kill -INT $spf; exit 1; }
  if [ -n "$spf" ]; then kill -INT "$spf" 2> /dev/null; wait "$spf" 2> /dev/null; fi
  net_nodelay_ok "$f" || { say "REFUSED $f: a client TCP socket without TCP_NODELAY, or none"; exit 1; }
}
NA=${#ARMS[@]}; NC=${#CELLS[@]}
case "$PHASE" in
  measure|default)
    R=$ROUNDS; [ "$PHASE" = default ] && R=1
    mkdir -p "$OUT/$PHASE"
    for r in $(seq 1 "$R"); do
      for w in "${WLS[@]}"; do
        set -- $w; name=$1; dirs=$2; pay=$3; k=$4; n=$5
        for i in $(seq 0 $((NC - 1))); do
          cell=${CELLS[$(( (i + r - 1) % NC ))]}
          [ "$dirs" != d ] && d_only "$cell" && continue
          for j in $(seq 0 $((NA - 1))); do
            arm=${ARMS[$(( (j + i + r) % NA ))]}
            run "$arm" "$cell" "$dirs" "$pay" "$k" "$n" "$OUT/$PHASE/$name-$cell-$arm-r$r.out"
          done
        done
        say "  $PHASE round $r $name done"
      done
    done ;;
  strace)
    mkdir -p "$OUT/strace"
    for w in "${WLS[@]}"; do
      set -- $w; name=$1; dirs=$2; pay=$3; k=$4; ns=$6
      for cell in "${CELLS[@]}"; do
        [ "$dirs" != d ] && d_only "$cell" && continue
        for arm in "${ARMS[@]}"; do
          f=$OUT/strace/$name-$cell-$arm
          run "$arm" "$cell" "$dirs" "$pay" "$k" "$ns" "$f.out" strace
          python3 gen/strace_window.py "$SCR/st.txt" > "$f.syscalls.txt"; gzip -9c "$SCR/st.txt" > "$f.strace.gz"
        done
      done
      say "  strace $name done"
    done ;;
  perf)  # the split only (perf attached costs CPU): perf stat, then perf record, one process each
    mkdir -p "$OUT/perf"
    for w in "${WLS[@]}"; do
      set -- $w; name=$1; dirs=$2; pay=$3; k=$4; n=$5
      for cell in "${CELLS[@]}"; do
        [ "$dirs" != d ] && d_only "$cell" && continue
        for arm in "${ARMS[@]}"; do
          f=$OUT/perf/$name-$cell-$arm
          run "$arm" "$cell" "$dirs" "$pay" "$k" "$n" "$f.stat.out" "stat:$f.perfstat"
          run "$arm" "$cell" "$dirs" "$pay" "$k" "$n" "$f.out" "record:$f.data"
        done
      done
      say "  perf $name done"
    done ;;
  *) echo "phase?" >&2; exit 2 ;;
esac
