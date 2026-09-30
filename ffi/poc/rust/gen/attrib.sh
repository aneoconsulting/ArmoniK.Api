#!/usr/bin/env bash
# Attribution of the client CPU of cells A, Df, Cf, Ff (2026-09-30, owner's goal 1): per cell and
# workload, one bin stream_probe process per measurement, cells rotated between repetitions:
#   stat-<w>-<cell>-<rep>   perf stat (cycles, instructions, cache references and misses, page
#                           faults, context switches, migrations, task clock) over the TIMED
#                           rounds only (perf --control; the probe enables and disables it)
#   rec-<w>-<cell>          perf record --call-graph dwarf over the timed rounds, classified by
#                           gen/perf_classify.py (leaf crate, the caller crate of libc and kernel
#                           leaves, thread class); perf.data kept in the scratch directory only
#   proc-<w>-<cell>         /proc per thread class (CPU, syscalls, bytes written), untimed
#   strace-<w>-<cell>       strace -f -c (syscall counts, warm-up and setup included), untimed
# AK_AT_INPROC=1: every process holds ALL the cells (block order, rotated), and perf counts only
# the named cell's timed rounds (AK_PERF_CELL), so each cell is profiled inside the same process
# composition; /proc and strace runs are skipped in that mode.
# Workloads: d16k1 (16 MiB streamed, k = 1), d16k8 (k = 8), c54k1 (c/P5.4, k = 1); the same calls
# per cell within a workload. Own server (serve.sh, 8 workers, SERVER set), started and stopped
# here. Run the whole script under the benchmark lock:
#   flock /tmp/ak-physical-bench.lock bash gen/attrib.sh OUT_DIR
set -Eeuo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
OUT=${1:?usage: attrib.sh OUT_DIR}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
export PATH="$HERE/gen/cargo-shim:$PATH" CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$HERE/target}
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1-4,11-14} AK_CPU_SERVER=${AK_CPU_SERVER:-5-8,15-18}
export AK_HOST_WORKERS=${AK_HOST_WORKERS:-8} AK_CORE_WORKERS=${AK_CORE_WORKERS:-8}
CELLS=${AK_AT_CELLS:-A,Df,Cf,Ff}
WORKS=${AK_AT_WORKS:-d16k1 d16k8 c54k1}
REPS=${AK_AT_REPS:-3}
PROBE=${AK_AT_PROBE:-$CARGO_TARGET_DIR/release/stream_probe}
SCR=${AK_AT_SCRATCH:-$(mktemp -d)}; mkdir -p "$SCR"
[ -x "$PROBE" ] || { echo "no $PROBE" >&2; exit 1; }
export AK_SERVE_STATE=$SCR/serve.state
cleanup() { ./serve.sh stop > /dev/null 2>&1 || true; }
trap cleanup EXIT
AK_SERVER_THREADS=8 ./serve.sh start --out "$OUT/server" > /dev/null
./serve.sh warm 16 > "$OUT/server/warm.log" 2>&1
SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE"); SPID=$(sed -n 's/^pid //p' "$AK_SERVE_STATE")
EV=${AK_AT_EVENTS:-cycles,instructions,cache-references,cache-misses,page-faults,context-switches,cpu-migrations,task-clock}
# workload -> size k rounds calls-per-round warm
wl() { case $1 in d16k1) echo "16MiB 1 4 12 8";; d16k8) echo "16MiB 8 4 3 4";; c54k1) echo "P5.4 1 4 40 16";; d4k1) echo "4MiB 1 4 40 16";; *) echo "?"; exit 2;; esac; }
{
  echo "# attribution: commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + UNCOMMITTED'); $(date -u +%FT%TZ); $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); kernel $(uname -r); no_turbo $(cat /sys/devices/system/cpu/intel_pstate/no_turbo); scaling min/max cpu1 $(cat /sys/devices/system/cpu/cpu1/cpufreq/scaling_min_freq)/$(cat /sys/devices/system/cpu/cpu1/cpufreq/scaling_max_freq) kHz"
  echo "# client $AK_CPU_CLIENT (host $AK_HOST_WORKERS, core $AK_CORE_WORKERS workers); server $AK_CPU_SERVER, 8 workers, pid $SPID, pinned socket; probe $PROBE sha256 $(sha256sum "$PROBE" | cut -c1-16), core $(ldd "$PROBE" | awk '/libak_core/{print $3}')"
  echo "# cells $CELLS; workloads $WORKS (16MiB k1: 4 x 12 calls, warm 8; 16MiB k8: 4 x 3 batches of 8, warm 4; P5.4 k1: 4 x 40, warm 16); perf stat reps $REPS (cell order rotated); perf $(perf --version), events $EV; perf_event_paranoid $(cat /proc/sys/kernel/perf_event_paranoid), kptr_restrict $(cat /proc/sys/kernel/kptr_restrict) (kernel symbols unresolved when 1)"
} > "$OUT/header.txt"
rot() { python3 -S -c 'import sys; c=sys.argv[1].split(","); n=int(sys.argv[2])%len(c); print(" ".join(c[n:]+c[:n]))' "$1" "$2"; }
S0=$(date +%s)
probe() { # OUTBASE CELL WORK PROC [wrapper...]
  local ob=$1 cell=$2 w=$3 pr=$4; shift 4
  read -r size k rounds calls warm <<< "$(wl "$w")"
  local cells=$cell pc=""
  if [ "${AK_AT_INPROC:-0}" = 1 ]; then cells=$(rot "$CELLS" "${ROT:-0}" | tr ' ' ','); pc=$cell; fi
  env ${pc:+AK_PERF_CELL=$pc} AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT=pinned AK_OUT="$ob.jsonl" AK_PROBE_CELLS="$cells" AK_PROBE_SIZES="$size" \
      AK_PROBE_K="$k" AK_PROBE_ROUNDS="$rounds" AK_PROBE_CALLS="$calls" AK_PROBE_WARM="$warm" AK_PROBE_ORDER=block AK_PROBE_PROC="$pr" \
      "$@" taskset -c "$AK_CPU_CLIENT" "$PROBE" 2> "$ob.err"
}
perfctl() { rm -f "$SCR/ctl" "$SCR/ack"; mkfifo "$SCR/ctl" "$SCR/ack"; }
for w in $WORKS; do
  for rep in $(seq 1 "$REPS"); do
    for cell in $(rot "$CELLS" "$rep"); do
      ROT=$rep
      perfctl
      probe "$OUT/stat-$w-$cell-$rep" "$cell" "$w" 0 AK_PERF_CTL="$SCR/ctl,$SCR/ack" \
        perf stat -x, -e "$EV" --control "fifo:$SCR/ctl,$SCR/ack" -D -1 -o "$OUT/stat-$w-$cell-$rep.perf" --
    done
  done
  echo "stat $w done at +$(( $(date +%s) - S0 )) s" >> "$OUT/progress.txt"
  for cell in ${CELLS//,/ }; do
    perfctl
    probe "$OUT/rec-$w-$cell" "$cell" "$w" 0 AK_PERF_CTL="$SCR/ctl,$SCR/ack" \
      perf record -q --call-graph dwarf,16384 -F 1999 --control "fifo:$SCR/ctl,$SCR/ack" -D -1 -o "$SCR/rec-$w-$cell.data" --
    perf script -i "$SCR/rec-$w-$cell.data" -F comm,tid,period,ip,sym,dso 2> /dev/null > "$SCR/rec-$w-$cell.script"
    python3 gen/perf_classify.py "$SCR/rec-$w-$cell.script" ${AK_AT_SYSMAP:+"$AK_AT_SYSMAP"} > "$OUT/rec-$w-$cell.txt"
    [ "${AK_AT_INPROC:-0}" = 1 ] && continue
    probe "$OUT/proc-$w-$cell" "$cell" "$w" 1
    probe "$OUT/strace-$w-$cell" "$cell" "$w" 0 strace -f -c -o "$OUT/strace-$w-$cell.txt"
  done
  echo "record/proc/strace $w done at +$(( $(date +%s) - S0 )) s" >> "$OUT/progress.txt"
done
echo "# run time $(( $(date +%s) - S0 )) s" >> "$OUT/header.txt"
