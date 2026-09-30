#!/usr/bin/env bash
# Physical probe step 4a (owner, 2026-09-30): where the client CPU of A, D, Cf and Cf-q goes,
# on d/16MiB at k = 1 and 8 and c/P5.4 at k = 1, main configuration (server 8 workers, core 8
# workers, grpc-core sized for 8 CPUs through gen/ncpus_shim.c, CLIENT 1-4,11-14, SERVER
# 5-8,15-18). Every client process is `campaign_rpc --profile N`: after its pre-checks and 5 warm
# batches, N batches of ONE cell on the benchmark's own path (Pool::batch), no Google Benchmark;
# perf is enabled only around that loop (--perf-ctl), and the loop reports process CPU and wall
# per chunk, per-thread CPU by class (/proc/self/task/*/schedstat) and getrusage deltas.
#
#   gen/profile_4a.sh OUT_DIR [stat|record|strace|all]
#
#   stat    REPS (3) rounds; per round and workload, in rotated order: A, D-retain, Cf-retain,
#           Cf-q-retain each ALONE in its process (only its own channel or client open), then
#           Cf-retain and Cf-q-retain again in a process with A, D, Cf, Cf-q all open (the grpc++
#           channels of A and D connected and warmed by the pre-check): perf stat of cycles and
#           instructions (user and kernel), cache-misses, page-faults, context-switches
#   record  one process per (cell alone, workload): perf record, cycles, -F 4000, LBR call stacks
#           (kernel frames by frame pointer); gen/perf_attrib.py resolves the kernel addresses
#           with the booted kernel's System.map and the KASLR offset it checks, and buckets cycles
#   plain   the control of `stat`: the same loops, cells alone, REPS rounds, no perf attached
#           (perf's counting adds a cost per context switch, and the cells differ in those)
#   strace  one process per (cell alone, workload), fewer calls: strace -f -yy, the syscalls
#           between the loop's two marker writes: counts per call, socket write sizes
#
# Each phase holds /tmp/ak-physical-bench.lock (the Rust slice measures under the same lock)
# and starts and stops its own server inside it (poc/rust/serve.sh at HEAD, not rebuilt).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
FFI=$(cd "$HERE/../.." && pwd)
OUT=${1:?usage: gen/profile_4a.sh OUT_DIR [stat|record|strace|all]}; PHASE=${2:-all}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1-4,11-14} AK_CPU_SERVER=${AK_CPU_SERVER:-5-8,15-18}
OSSET=${AK_CPU_OS:-0,9,10,19}
REPS=${REPS:-3}; WK=8; GCPUS=8
EXE=$HERE/build-campaign/campaign_rpc; SERVE=$FFI/poc/rust/serve.sh
LOCK=/tmp/ak-physical-bench.lock
LOG=$OUT/runner.log
say() { echo "$*" | tee -a "$LOG"; }
if [ "${3:-}" != --locked ]; then  # one lock hold per phase, so the other slice can interleave
  case "$PHASE" in all) PH="stat record strace plain" ;; stat|record|strace|plain) PH=$PHASE ;; *) echo "phase?" >&2; exit 2 ;; esac
  T0=$(date +%s)
  for p in $PH; do
    t=$(date +%s); flock "$LOCK" bash "$0" "$OUT" "$p" --locked || exit 1
    say "phase $p: $(( $(date +%s) - t )) s (lock held)"
  done
  say "profile_4a: $(( $(date +%s) - T0 )) s in all"
  exit 0
fi
SCR=$(mktemp -d)
trap '[ -f "$SCR/serve.state" ] && bash "$SERVE" stop > /dev/null 2>&1; rm -rf "$SCR"' EXIT
taskset -c "$OSSET" gcc -O2 -shared -fPIC -o "$SCR/ncpus.so" gen/ncpus_shim.c -ldl || exit 1
# workload: name dirs payload k profile-batches strace-batches
WLS=("d16k1 d 16MiB 1 120 12" "d16k8 d 16MiB 8 20 3" "c54k1 c P5.4 1 400 40")
CELLS=(A D-retain Cf-retain Cf-q-retain)

server_up() {
  export AK_SERVE_STATE=$SCR/serve.state
  AK_SERVER_THREADS=$WK bash "$SERVE" start --out "$SCR/srv" > "$SCR/srv.out" 2>&1 || { cat "$SCR/srv.out"; return 1; }
  bash "$SERVE" warm 64 > /dev/null 2>&1 || return 1
  SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE")
  say "  server pid $(sed -n 's/^pid //p' "$AK_SERVE_STATE"), $(head -2 "$SCR/srv/rpc-server.log" | tail -1 | sed 's/.*tokio/tokio/')"
}
server_down() { bash "$SERVE" stop > /dev/null 2>&1; }
client() {  # client CELLS PROFILE_CELL WL_FIELDS... -- PREFIX...; prints the profile JSON line
  local cells=$1 pcell=$2 dirs=$3 pay=$4 k=$5 n=$6; shift 6
  taskset -c "$AK_CPU_CLIENT" "$@" env LD_PRELOAD="$SCR/ncpus.so" AK_SHIM_NCPUS=$GCPUS "$EXE" --target "unix:$SOCK" \
    --expect 540422 --transport pinned --cells "$cells" --profile-cell "$pcell" --dirs "$dirs" --payloads "$pay" \
    --inflight "$k" --workers $WK --profile "$n" --profile-chunks 10 --perf-ctl "$SCR/ctl,$SCR/ack"
}
header() {
  { echo "# profile_4a $1: commit $(git -C "$FFI" rev-parse --short HEAD)$(git -C "$FFI" status --porcelain -- poc/cpp/src poc/cpp/gen poc/codec | grep -q . && echo ' + UNCOMMITTED'), $(date -u +%FT%TZ)"
    echo "# exe $EXE sha256 $(sha256sum "$EXE" | cut -c1-16); core $(ldd "$EXE" | grep -o '/[^ ]*libak_core\.so') sha256 $(sha256sum "$(ldd "$EXE" | grep -o '/[^ ]*libak_core\.so')" | cut -c1-16)"
    echo "# CLIENT $AK_CPU_CLIENT SERVER $AK_CPU_SERVER; server 8 workers; core --workers $WK; grpc-core sysconf = $GCPUS (ncpus_shim); pinned transport; retain mode"
    echo "# $(perf --version); perf_event_paranoid $(cat /proc/sys/kernel/perf_event_paranoid); kptr_restrict $(cat /proc/sys/kernel/kptr_restrict)"
    echo "# machine $(python3 gen/machine_facts.py "$AK_CPU_CLIENT" "$AK_CPU_SERVER")"; } >> "$LOG"
}
rm -f "$SCR/ctl" "$SCR/ack"; mkfifo "$SCR/ctl" "$SCR/ack"

phase_stat() {
  header stat; server_up || exit 1
  local r w i f
  for r in $(seq 1 "$REPS"); do
    for w in "${WLS[@]}"; do
      set -- $w; local name=$1 dirs=$2 pay=$3 k=$4 n=$5
      for i in 0 1 2 3; do
        local cell=${CELLS[$(( (i + r - 1) % 4 ))]}
        f=$OUT/stat/$name-$cell-alone-r$r; mkdir -p "$OUT/stat"
        client "$cell" "$cell" "$dirs" "$pay" "$k" "$n" perf stat -D -1 --control "fifo:$SCR/ctl,$SCR/ack" -x, -o "$f.perfstat" \
          -e cycles:u,cycles:k,instructions:u,instructions:k,cache-misses,page-faults,context-switches -- > "$f.out" 2>&1 \
          || { tail -3 "$f.out"; say "FAILED $f"; server_down; exit 1; }
      done
      for cell in Cf-retain Cf-q-retain; do
        f=$OUT/stat/$name-$cell-withgrpc-r$r
        client "A,D-retain,Cf-retain,Cf-q-retain" "$cell" "$dirs" "$pay" "$k" "$n" perf stat -D -1 --control "fifo:$SCR/ctl,$SCR/ack" -x, -o "$f.perfstat" \
          -e cycles:u,cycles:k,instructions:u,instructions:k,cache-misses,page-faults,context-switches -- > "$f.out" 2>&1 \
          || { tail -3 "$f.out"; say "FAILED $f"; server_down; exit 1; }
      done
      say "  stat round $r $name done"
    done
  done
  server_down
}
phase_plain() {  # the control: the same loops as `stat` (cells alone), with no perf attached
  header plain; server_up || exit 1
  local r w i f
  for r in $(seq 1 "$REPS"); do
    for w in "${WLS[@]}"; do
      set -- $w; local name=$1 dirs=$2 pay=$3 k=$4 n=$5
      for i in 0 1 2 3; do
        local cell=${CELLS[$(( (i + r - 1) % 4 ))]}
        f=$OUT/plain/$name-$cell-alone-r$r; mkdir -p "$OUT/plain"
        taskset -c "$AK_CPU_CLIENT" env LD_PRELOAD="$SCR/ncpus.so" AK_SHIM_NCPUS=$GCPUS "$EXE" --target "unix:$SOCK" \
          --expect 540422 --transport pinned --cells "$cell" --dirs "$dirs" --payloads "$pay" --inflight "$k" --workers $WK \
          --profile "$n" --profile-chunks 10 > "$f.out" 2>&1 || { tail -3 "$f.out"; say "FAILED $f"; server_down; exit 1; }
      done
      say "  plain round $r $name done"
    done
  done
  server_down
}
phase_record() {
  header record; server_up || exit 1
  local w cell f; mkdir -p "$OUT/record"
  for w in "${WLS[@]}"; do
    set -- $w; local name=$1 dirs=$2 pay=$3 k=$4 n=$5
    for cell in "${CELLS[@]}"; do
      f=$OUT/record/$name-$cell
      client "$cell" "$cell" "$dirs" "$pay" "$k" "$n" perf record -D -1 --control "fifo:$SCR/ctl,$SCR/ack" -e cycles -F 4000 \
        --call-graph lbr -o "$f.data" -- > "$f.out" 2>&1 || { tail -3 "$f.out"; say "FAILED $f"; server_down; exit 1; }
      say "  record $name $cell: $(grep -o 'wrote [^)]*samples)' "$f.out")"
    done
  done
  server_down
}
phase_strace() {
  header strace; server_up || exit 1
  local w cell f; mkdir -p "$OUT/strace"
  for w in "${WLS[@]}"; do
    set -- $w; local name=$1 dirs=$2 pay=$3 k=$4 ns=$6
    for cell in "${CELLS[@]}"; do
      f=$OUT/strace/$name-$cell
      # --perf-ctl is not used here: the markers bracket the loop
      taskset -c "$AK_CPU_CLIENT" strace -f -qq -yy -s 24 -e signal=none \
        -e trace=write,writev,sendmsg,sendto,read,readv,recvmsg,recvfrom,futex,epoll_wait,epoll_pwait,epoll_pwait2,mmap,munmap,madvise,mremap,brk,sched_yield,poll,ppoll,io_uring_enter \
        -o "$SCR/st.txt" env LD_PRELOAD="$SCR/ncpus.so" AK_SHIM_NCPUS=$GCPUS "$EXE" --target "unix:$SOCK" --expect 540422 --transport pinned --cells "$cell" --dirs "$dirs" \
        --payloads "$pay" --inflight "$k" --workers $WK --profile "$ns" --profile-chunks 1 > "$f.out" 2>&1 \
        || { tail -3 "$f.out"; say "FAILED $f"; server_down; exit 1; }
      python3 gen/strace_window.py "$SCR/st.txt" > "$f.syscalls.txt"; gzip -9c "$SCR/st.txt" > "$f.strace.gz"
      say "  strace $name $cell: $(head -1 "$f.syscalls.txt")"
    done
  done
  server_down
}
case "$PHASE" in
  stat) phase_stat ;;
  record) phase_record ;;
  strace) phase_strace ;;
  plain) phase_plain ;;
esac
