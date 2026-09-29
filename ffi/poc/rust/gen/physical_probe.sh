#!/usr/bin/env bash
# The physical-machine probe (FIX-PLAN WP11 "to measure on the campaign machine"; coordinator
# step 2, 2026-09-29): where direction d's A -> Df and Df -> Df-chan client CPU goes, with c/P5.4
# beside it, k = 1 and 8, against an ALREADY RUNNING shared server (poc/rust/serve.sh), so the
# Rust and the C++ slices are measured one after the other against the same server process.
#
#   gen/physical_probe.sh SEGMENT OUT_DIR
#
# SEGMENT (presets, each value overridable from the environment, see below):
#   main     spread phase (AK_PP_SPREAD passes of cells A, A2, Df), then AK_PP_PASSES passes of
#            the main cells, then one attribution pass (not timed); workers 8 everywhere
#   variant  AK_PP_PASSES passes of a smaller cell set, then the attribution pass; workers 4
#   smoke    one short pass of everything (proves the driver runs; not a measurement)
#
# The server (shared-server mode, the default): AK_SERVE_STATE names the state file of a server
# started by `serve.sh start`; this driver dials its `pinned` socket (AK_PP_TRANSPORT) and never
# starts, restarts or stops it. It checks that the process is alive before every client process
# and at the end (same pid), that its CPU affinity equals AK_CPU_SERVER and that its worker count
# (AK_SERVER_THREADS in its environment, else rpc_server's default 4) equals AK_PP_SERVER_THREADS
# (the preset's), and aborts otherwise. AK_PP_OWN_SERVER=1 starts and stops a server of its own
# (for a smoke).
#
# One pass = one probe process (bin stream_probe: direction d, k = 1, block order, no /proc
# reads; per-call process CPU, getrusage per round) and one grid process (benches/rpc_suite
# narrowed: cells x c/P5.4, d/4MiB, d/16MiB x k 1, 8; criterion; getrusage per sample); the two
# processes alternate their order from pass to pass, the probe's cell order rotates by one per
# pass, the grid's benchmark order is criterion registration in a seeded permutation (seed =
# launch = pass number; spread passes 101, 102, ...). Every call is checked (requirement 18); a
# failed check aborts the driver (the partial output is kept, marked ABORTED in header.txt).
#
# Environment (defaults from the preset):
#   AK_CPU_CLIENT, AK_CPU_SERVER   taskset lists (from ffi/campaign.machine when unset)
#   AK_HOST_WORKERS, AK_CORE_WORKERS   the client's runtimes (grid.rs), exported to every client
#   AK_PP_SERVER_THREADS           the worker count the server must have
#   AK_PP_SPREAD, AK_PP_PASSES     pass counts
#   AK_PP_SPREAD_PROBE_CELLS, AK_PP_SPREAD_GRID_CELLS, AK_PP_PROBE_CELLS, AK_PP_GRID_CELLS,
#   AK_PP_ATTR_CELLS, AK_PP_GRID_PAYLOADS, AK_PP_GRID_K, AK_PP_SIZES
#   AK_PP_ROUNDS, AK_PP_CALLS, AK_PP_WARM (probe), AK_PP_ATTR_ROUNDS, AK_PP_ATTR_CALLS
#   AK_PP_SAMPLES, AK_PP_WARMUP_MS, AK_PP_MEASURE_MS (grid, criterion), AK_PP_SERVER_WARM
set -Eeuo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd); cd "$HERE"
SEG=${1:?usage: physical_probe.sh main|variant|smoke OUT_DIR}; OUT=${2:?usage: physical_probe.sh SEGMENT OUT_DIR}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
[ -e "$OUT/header.txt" ] && { echo "physical_probe.sh: $OUT already holds a session" >&2; exit 2; }
export PATH="$HERE/gen/cargo-shim:$PATH" CARGO_TARGET_DIR="$HERE/target"
if [ -z "${AK_CPU_CLIENT:-}" ] || [ -z "${AK_CPU_SERVER:-}" ]; then . "$HERE/../../campaign.machine"; fi
export AK_CPU_CLIENT AK_CPU_SERVER

case "$SEG" in
  main)
    : "${AK_HOST_WORKERS:=8}" "${AK_CORE_WORKERS:=8}" "${AK_PP_SERVER_THREADS:=8}"
    : "${AK_PP_SPREAD:=4}" "${AK_PP_PASSES:=3}"
    : "${AK_PP_SPREAD_PROBE_CELLS:=A,A2,Df}" "${AK_PP_SPREAD_GRID_CELLS:=A,Df}"
    : "${AK_PP_PROBE_CELLS:=A,A2,Df,Df-chan,Cf,Cf-cb,C,C-cb}" "${AK_PP_GRID_CELLS:=A,Df,Cf,Cf-cb,C,C-cb}"
    : "${AK_PP_ATTR_CELLS:=A,Df,Df-chan,Cf,Cf-cb,C,C-cb,Cf-split,Cf-cb-split}"
    : "${AK_PP_GRID_PAYLOADS:=P5.4,4MiB,16MiB}" ;;
  variant)
    : "${AK_HOST_WORKERS:=4}" "${AK_CORE_WORKERS:=4}" "${AK_PP_SERVER_THREADS:=4}"
    : "${AK_PP_SPREAD:=0}" "${AK_PP_PASSES:=3}"
    : "${AK_PP_SPREAD_PROBE_CELLS:=A,A2,Df}" "${AK_PP_SPREAD_GRID_CELLS:=A,Df}"
    : "${AK_PP_PROBE_CELLS:=A,A2,Df,Df-chan,Cf,Cf-cb}" "${AK_PP_GRID_CELLS:=A,Df,Cf,Cf-cb}"
    : "${AK_PP_ATTR_CELLS:=A,Df,Df-chan,Cf,Cf-cb}"
    : "${AK_PP_GRID_PAYLOADS:=P5.4,4MiB,16MiB}" ;;
  smoke)
    : "${AK_HOST_WORKERS:=8}" "${AK_CORE_WORKERS:=8}" "${AK_PP_SERVER_THREADS:=${AK_SERVER_THREADS:-8}}"
    : "${AK_PP_SPREAD:=1}" "${AK_PP_PASSES:=1}"
    : "${AK_PP_SPREAD_PROBE_CELLS:=A,A2,Df}" "${AK_PP_SPREAD_GRID_CELLS:=A,Df}"
    : "${AK_PP_PROBE_CELLS:=A,A2,Df,Df-chan,Cf,Cf-cb,C,C-cb}" "${AK_PP_GRID_CELLS:=A,Df,Cf,Cf-cb,C,C-cb}"
    : "${AK_PP_ATTR_CELLS:=A,Df,Df-chan,Cf,Cf-cb,C,C-cb,Cf-split,Cf-cb-split}"
    : "${AK_PP_GRID_PAYLOADS:=P5.4,4MiB,16MiB}"
    : "${AK_PP_ROUNDS:=2}" "${AK_PP_CALLS:=2}" "${AK_PP_WARM:=1}" "${AK_PP_ATTR_ROUNDS:=1}" "${AK_PP_ATTR_CALLS:=2}"
    : "${AK_PP_WARMUP_MS:=5}" "${AK_PP_MEASURE_MS:=20}" "${AK_PP_SERVER_WARM:=4}" ;;
  *) echo "physical_probe.sh: SEGMENT is main, variant or smoke" >&2; exit 2 ;;
esac
: "${AK_PP_GRID_K:=1,8}" "${AK_PP_SIZES:=16MiB,4MiB}" "${AK_PP_TRANSPORT:=pinned}"
: "${AK_PP_ROUNDS:=10}" "${AK_PP_CALLS:=8}" "${AK_PP_WARM:=4}" "${AK_PP_ATTR_ROUNDS:=4}" "${AK_PP_ATTR_CALLS:=4}"
: "${AK_PP_SAMPLES:=10}" "${AK_PP_WARMUP_MS:=200}" "${AK_PP_MEASURE_MS:=800}" "${AK_PP_SERVER_WARM:=16}"
export AK_HOST_WORKERS AK_CORE_WORKERS

# ---- build (release), before anything is timed ----
cargo build --release -q -p campaign --bin stream_probe 2>/dev/null
RPCB=$(cargo bench -q -p campaign --bench rpc_suite --no-run --message-format=json 2>/dev/null | python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="rpc_suite" and m.get("executable"): print(m["executable"])' | tail -1)
PROBE="$CARGO_TARGET_DIR/release/stream_probe"
[ -x "$PROBE" ] && [ -x "$RPCB" ] || { echo "physical_probe.sh: build failed" >&2; exit 1; }
cc -O2 -shared -fPIC -o "$CARGO_TARGET_DIR/allocprobe.so" gen/probe/allocprobe.c -ldl

SCRATCH=$(mktemp -d)
cleanup() { [ "${AK_PP_OWN_SERVER:-0}" = 1 ] && ./serve.sh stop > /dev/null 2>&1; rm -rf "$SCRATCH"; }
trap cleanup EXIT
if [ "${AK_PP_OWN_SERVER:-0}" = 1 ]; then
  export AK_SERVE_STATE="$SCRATCH/serve.state"
  ./serve.sh build > /dev/null 2>&1
  AK_SERVER_THREADS=$AK_PP_SERVER_THREADS ./serve.sh start --out "$OUT/own-server" > /dev/null
fi
STATE=${AK_SERVE_STATE:?shared-server mode: AK_SERVE_STATE must name the state file of a running serve.sh server}
[ -f "$STATE" ] || { echo "physical_probe.sh: no state file $STATE (start the server with serve.sh start)" >&2; exit 1; }
SOCK=$(sed -n "s/^$AK_PP_TRANSPORT //p" "$STATE"); SPID=$(sed -n 's/^pid //p' "$STATE")
[ -S "$SOCK" ] && kill -0 "$SPID" 2>/dev/null || { echo "physical_probe.sh: the server of $STATE is not running" >&2; exit 1; }

# ---- the machine, read at run time ----
expand() { python3 -S -c 'import sys
out=[]
for p in sys.argv[1].split(","):
    a,_,b=p.partition("-"); out+=range(int(a),int(b or a)+1)
print(" ".join(map(str,out)))' "$1"; }
sysf() { cat "$1" 2>/dev/null || echo n/a; }
percpu() { # label file-under-cpufreq-or-topology cpus...
  local l=$1 f=$2; shift 2; local s=""
  for c in "$@"; do s="$s $c:$(sysf /sys/devices/system/cpu/cpu$c/$f)"; done; echo "# $l$s"; }
srv_threads_env=$(tr '\0' '\n' < /proc/$SPID/environ 2>/dev/null | sed -n 's/^AK_SERVER_THREADS=//p')
srv_workers=${srv_threads_env:-4}
export AK_SERVER_THREADS=$srv_workers   # read by rpc_suite for its header line only
srv_aff=$(taskset -pc "$SPID" | sed 's/.*: //')
srv_cpu() { awk '{print $14+$15}' /proc/$SPID/stat; }   # server utime+stime, clock ticks
same_set() { [ "$(expand "$1")" = "$(expand "$2")" ]; }
fail=""
same_set "$srv_aff" "$AK_CPU_SERVER" || fail="server affinity $srv_aff differs from AK_CPU_SERVER $AK_CPU_SERVER"
[ "$srv_workers" = "$AK_PP_SERVER_THREADS" ] || fail="${fail:+$fail; }server workers $srv_workers differ from the preset's $AK_PP_SERVER_THREADS"
CL=$(expand "$AK_CPU_CLIENT"); SV=$(expand "$AK_CPU_SERVER")
iso_cmd=$(tr ' ' '\n' < /proc/cmdline | grep -E '^(isolcpus|nohz_full|rcu_nocbs|irqaffinity)=' | tr '\n' ' ' || true)
cg=$(sed -n 's/^0:://p' /proc/self/cgroup)
cg_cpus=$(sysf "/sys/fs/cgroup$cg/cpuset.cpus.effective")
iso_mech="taskset -c affinity only"
[ -n "$iso_cmd" ] && iso_mech="kernel command line: $iso_cmd"
iso_sys=$(cat /sys/devices/system/cpu/isolated 2>/dev/null || true)
[ -n "$iso_sys" ] && iso_mech="$iso_mech; isolated=$iso_sys"
[ "$cg_cpus" != n/a ] && ! same_set "$cg_cpus" "0-$(( $(nproc --all) - 1 ))" && iso_mech="$iso_mech; cgroup cpuset $cg_cpus"
{
  echo "# physical probe, segment $SEG: commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + UNCOMMITTED changes in poc/rust or poc/codec'); $(date -u +%FT%TZ); host $(hostname)"
  echo "# purpose: FIX-PLAN WP11 (campaign machine), coordinator step 2: direction d (16 MiB, 4 MiB; 2 MiB chunks) and c/P5.4, k = 1 and 8"
  echo "# cpu        $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); $(nproc --all) logical CPUs online; NUMA nodes $(ls -d /sys/devices/system/node/node* 2>/dev/null | wc -l)"
  echo "# kernel     $(uname -r) ($(uname -v))"
  echo "# smt        active=$(sysf /sys/devices/system/cpu/smt/active) control=$(sysf /sys/devices/system/cpu/smt/control)"
  echo "# turbo      intel_pstate/no_turbo=$(sysf /sys/devices/system/cpu/intel_pstate/no_turbo) intel_pstate/status=$(sysf /sys/devices/system/cpu/intel_pstate/status) min_perf_pct=$(sysf /sys/devices/system/cpu/intel_pstate/min_perf_pct) max_perf_pct=$(sysf /sys/devices/system/cpu/intel_pstate/max_perf_pct) cpufreq/boost=$(sysf /sys/devices/system/cpu/cpufreq/boost)"
  percpu "governor  " cpufreq/scaling_governor $CL $SV
  percpu "scaling_min_freq (kHz)" cpufreq/scaling_min_freq $CL $SV
  percpu "scaling_max_freq (kHz)" cpufreq/scaling_max_freq $CL $SV
  percpu "scaling_cur_freq at start (kHz)" cpufreq/scaling_cur_freq $CL $SV
  percpu "siblings  " topology/thread_siblings_list $CL $SV
  echo "# isolation  $iso_mech; /sys/devices/system/cpu/isolated='$(sysf /sys/devices/system/cpu/isolated)' nohz_full='$(sysf /sys/devices/system/cpu/nohz_full)'; this process's cgroup $cg cpuset.cpus.effective=$cg_cpus"
  echo "# cpu sets   CLIENT AK_CPU_CLIENT=$AK_CPU_CLIENT (taskset -c on every client process); SERVER AK_CPU_SERVER=$AK_CPU_SERVER; ffi/campaign.machine not edited"
  echo "# server     $([ "${AK_PP_OWN_SERVER:-0}" = 1 ] && echo own || echo shared): state $STATE, pid $SPID, $(readlink /proc/$SPID/exe), affinity $srv_aff, AK_SERVER_THREADS=${srv_threads_env:-unset (rpc_server default 4)} -> $srv_workers tokio workers; threads by name: $(cat /proc/$SPID/task/*/comm | sort | uniq -c | awk '{printf "%s x%s, ", $2, $1}')socket $SOCK ($AK_PP_TRANSPORT)$([ "${AK_PP_OWN_SERVER:-0}" = 1 ] && echo '; started by this driver (AK_PP_OWN_SERVER=1)')"
  echo "# server core $(ldd "$(readlink /proc/$SPID/exe)" | awk '/libak_core/{print $3}')"
  echo "# client     AK_HOST_WORKERS=$AK_HOST_WORKERS (tokio workers of each A/D/Df/-cb cell runtime), AK_CORE_WORKERS=$AK_CORE_WORKERS (ak_runtime_new per core client), AK_CHAN_DEPTH=${AK_CHAN_DEPTH:-1} (Df-chan); k caller threads (blocking cells) or tasks; criterion default-features off (no rayon pool); no pool in the client is sized from the affinity mask (every tokio runtime has an explicit worker count)"
  echo "# client core $(ldd "$PROBE" | awk '/libak_core/{print $3}') (probe), $(ldd "$RPCB" | awk '/libak_core/{print $3}') (grid); rustc $(rustc -V | cut -d' ' -f2), cargo $(cargo -V | cut -d' ' -f2); probe sha256 $(sha256sum "$PROBE" | cut -c1-16), grid $(basename "$RPCB") sha256 $(sha256sum "$RPCB" | cut -c1-16)"
  echo "# load       /proc/loadavg at start: $(cat /proc/loadavg); perf: $(perf --version 2>/dev/null || echo absent) (not used), perf_event_paranoid=$(sysf /proc/sys/kernel/perf_event_paranoid)"
  echo "# plan       spread passes $AK_PP_SPREAD (probe cells $AK_PP_SPREAD_PROBE_CELLS; grid cells $AK_PP_SPREAD_GRID_CELLS), main passes $AK_PP_PASSES (probe cells $AK_PP_PROBE_CELLS; grid cells $AK_PP_GRID_CELLS), attribution pass cells $AK_PP_ATTR_CELLS; probe sizes $AK_PP_SIZES, k = 1, rounds $AK_PP_ROUNDS x calls $AK_PP_CALLS, warm $AK_PP_WARM calls per (cell, size), block order, no /proc; grid dirs c,d payloads $AK_PP_GRID_PAYLOADS k $AK_PP_GRID_K, criterion samples $AK_PP_SAMPLES, warm-up $AK_PP_WARMUP_MS ms, measurement $AK_PP_MEASURE_MS ms per benchmark, nresamples 1000, AK_RPC_SERVER_WARMUP=8; attribution rounds $AK_PP_ATTR_ROUNDS x calls $AK_PP_ATTR_CALLS with /proc reads and the allocation shim (not timed); server warm-up serve.sh warm $AK_PP_SERVER_WARM at the start"
  echo "# notable    the run-to-run spread of an in-process gap: for each workload, the range (max - min) over the SPREAD passes of the per-pass median of (Df - A), each pass its own client processes against the same server; the A2 - A range (probe, two independent A instances in one process) is the same-code floor beside it. A difference between cells or variants smaller than that range is not attributed. The main passes' own per-gap ranges are reported beside it"
  echo "# clocks     per-call CPU = process CPU (CLOCK_PROCESS_CPUTIME_ID, every thread of the client): probe the delta around each call, grid per criterion sample / calls; wall beside it; context switches and minor faults per call = getrusage(RUSAGE_SELF) deltas / calls (probe per round, grid per sample)"
} > "$OUT/header.txt"
if [ -n "$fail" ]; then echo "# ABORTED before any timing: $fail" >> "$OUT/header.txt"; echo "physical_probe.sh: $fail" >&2; exit 1; fi
cat "$OUT/header.txt"

alive() { [ "$(sed -n 's/^pid //p' "$STATE" 2>/dev/null)" = "$SPID" ] && kill -0 "$SPID" 2>/dev/null || { echo "# ABORTED: the server (pid $SPID) is gone or was replaced" >> "$OUT/header.txt"; exit 1; }; }
trap 'echo "# ABORTED at +$(( $(date +%s) - S0 )) s (a failed check or command; see progress.txt and the *.err / *.log files)" >> "$OUT/header.txt"' ERR
rot() { python3 -S -c 'import sys; c=sys.argv[1].split(","); n=int(sys.argv[2])%len(c); print(",".join(c[n:]+c[:n]))' "$1" "$2"; }
S0=$(date +%s)
log() { echo "$1 at +$(( $(date +%s) - S0 )) s, server CPU ticks $(srv_cpu)" >> "$OUT/progress.txt"; }

probe() { # NAME CELLS PASS [attr]
  alive; log "start $1"
  if [ "${4:-}" = attr ]; then
    env LD_PRELOAD="$CARGO_TARGET_DIR/allocprobe.so" AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT="$AK_PP_TRANSPORT" AK_OUT="$OUT/$1.jsonl" \
        AK_PROBE_CELLS="$2" AK_PROBE_SIZES="$AK_PP_SIZES" AK_PROBE_ORDER=rotate AK_PROBE_PROC=1 \
        AK_PROBE_ROUNDS="$AK_PP_ATTR_ROUNDS" AK_PROBE_CALLS="$AK_PP_ATTR_CALLS" AK_PROBE_WARM="$AK_PP_WARM" \
        taskset -c "$AK_CPU_CLIENT" "$PROBE" 2> "$OUT/$1.err"
  else
    env AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT="$AK_PP_TRANSPORT" AK_OUT="$OUT/$1.jsonl" \
        AK_PROBE_CELLS="$(rot "$2" "$3")" AK_PROBE_SIZES="$AK_PP_SIZES" AK_PROBE_ORDER=block AK_PROBE_PROC=0 \
        AK_PROBE_ROUNDS="$AK_PP_ROUNDS" AK_PROBE_CALLS="$AK_PP_CALLS" AK_PROBE_WARM="$AK_PP_WARM" \
        taskset -c "$AK_CPU_CLIENT" "$PROBE" 2> "$OUT/$1.err"
  fi
  log "end $1"
}
grid() { # NAME CELLS LAUNCH
  alive; log "start $1"
  env AK_RPC_SOCKET="$SOCK" AK_RPC_TRANSPORT="$AK_PP_TRANSPORT" AK_OUT="$OUT/$1.jsonl" CRITERION_HOME="$SCRATCH/crit-$1" AK_LAUNCH="$3" \
      AK_SAMPLES="$AK_PP_SAMPLES" AK_WARMUP_MS="$AK_PP_WARMUP_MS" AK_MEASURE_MS="$AK_PP_MEASURE_MS" AK_NRESAMPLES=1000 AK_RPC_SERVER_WARMUP=8 \
      AK_RPC_CELLS="$2" AK_RPC_DIRS=c,d AK_RPC_PAYLOADS="$AK_PP_GRID_PAYLOADS" AK_RPC_INFLIGHT="$AK_PP_GRID_K" \
      taskset -c "$AK_CPU_CLIENT" "$RPCB" > "$OUT/$1.log" 2>&1
  log "end $1"
}
pass() { # KIND(spread|main) N PROBE_CELLS GRID_CELLS LAUNCH
  if [ $(( $2 % 2 )) = 1 ]; then probe "$1-probe-$2" "$3" "$2"; grid "$1-grid-$2" "$4" "$5"
  else grid "$1-grid-$2" "$4" "$5"; probe "$1-probe-$2" "$3" "$2"; fi
}

log "server warm-up"; AK_SERVE_STATE="$STATE" ./serve.sh warm "$AK_PP_SERVER_WARM" > "$OUT/server-warm.log" 2>&1
for i in $(seq 1 "$AK_PP_SPREAD"); do pass spread "$i" "$AK_PP_SPREAD_PROBE_CELLS" "$AK_PP_SPREAD_GRID_CELLS" $((100 + i)); done
for i in $(seq 1 "$AK_PP_PASSES"); do pass main "$i" "$AK_PP_PROBE_CELLS" "$AK_PP_GRID_CELLS" "$i"; done
T1=$(date +%s)
probe attr "$AK_PP_ATTR_CELLS" 0 attr
alive
echo "# benchmark wall time $(( T1 - S0 )) s (spread and main passes, server warm-up included); attribution pass $(( $(date +%s) - T1 )) s; scaling_cur_freq at end (kHz):$(for c in $CL $SV; do printf ' %s:%s' "$c" "$(sysf /sys/devices/system/cpu/cpu$c/cpufreq/scaling_cur_freq)"; done); loadavg at end: $(cat /proc/loadavg)" >> "$OUT/header.txt"
python3 gen/physical_tables.py "$OUT" > "$OUT/tables.md"
echo "tables: $OUT/tables.md"
