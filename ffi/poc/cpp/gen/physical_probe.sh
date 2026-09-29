#!/usr/bin/env bash
# The C++ slice's part of the physical-machine probe (FIX-PLAN WP11 on the campaign machine;
# owner, 2026-09-29). Directions c (P5.4 unary upload) and d (4 MiB and 16 MiB streamed in
# 2 MiB M5 chunks), each at k = 1 and 8, cells A, D, Cf, Cf-q, C, C-q, the pinned transport.
# One invocation is one SEGMENT against one server; the owner runs the segments one after the
# other, each against its own server launch.
#
#   gen/physical_probe.sh --segment main|var4 --out DIR [--smoke] [--grpc-cpus N]
#
#   Shared server (the probe's mode): AK_SERVE_STATE names a server started by the owner with
#   poc/rust/serve.sh (four lines: shipped PATH, pinned PATH, pid N, dir D). The driver never
#   starts, stops, restarts or warms it; before EVERY client process it checks that the pid is
#   the one it read first and that the process's affinity is AK_CPU_SERVER, and aborts before
#   timing otherwise. It dials unix:<pinned path>.
#   Own server (standalone use, and --smoke): with AK_SERVE_STATE unset, the driver starts one
#   with serve.sh at the segment's worker count, warms it (serve.sh warm 50) and stops it.
#
# Segments (worker counts: owner, 2026-09-29):
#   main  server 8 tokio workers (checked from its environment), the core's runtime 8 workers
#         (campaign_rpc --workers 8); 4 SPREAD passes (cells A, D, Cf), then 3 MAIN passes
#         (A, D, Cf, Cf-q, C, C-q)
#   var4  server 4 workers, core 4 workers; 3 passes, every cell (A, D, Cf, Cf-q, C, C-q): each
#         is a spread pass (the segment's own spread)
#   grpc++: the harness sets no grpc-core pool; grpc-core v1.80 sizes itself from
#   sysconf(_SC_NPROCESSORS_CONF), not the affinity mask (gen/ncpus_shim.c says where), so
#   it runs the same pools in both segments. --grpc-cpus N preloads gen/ncpus_shim.c so that
#   sysconf reports N CPUs (off by default; the header says which). Every client process prints
#   its threads by name (campaign_rpc's header and end lines), which is what each stack runs.
#
# The spread (the "notable" threshold): a pass is one independent client process; within it
# Google Benchmark interleaves the repetitions of every benchmark at random, and the
# registration order is rotated by pass (launch index). Per pass and workload, the in-process
# gap of a cell is median(cell) - median(A) of per-call client CPU (and wall). The spread of
# D - A and of Cf - A is the range (max - min) of that gap over the segment's spread passes
# (main: the 4 spread passes; var4: its 3 passes); gen/physical_probe.py prints it beside every
# gap. Absolute times only, no ratio.
#
# Samples: Google Benchmark, fixed iterations per (payload, k) (--iters: no estimation, the
# same number of batches for every cell of a workload), R repetitions per pass, a warm-up of
# --warmup-s before each benchmark's first repetition, process CPU (MeasureProcessCPUTime) and
# wall (UseRealTime), and the process's getrusage deltas (context switches, faults) around each
# repetition's timed loop. Every call checked (req 18); a failed check aborts the process
# (exit 3), no sample file is written and the driver stops.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
FFI=$(cd "$HERE/../.." && pwd); REPO=$(git -C "$FFI" rev-parse --show-toplevel)
SEG=""; OUT=""; SMOKE=0; GCPUS=""
while [ $# -gt 0 ]; do
  case "$1" in
    --segment) SEG=$2; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    --smoke) SMOKE=1; shift ;;
    --grpc-cpus) GCPUS=$2; shift 2 ;;
    *) echo "usage: gen/physical_probe.sh --segment main|var4 --out DIR [--smoke] [--grpc-cpus N]" >&2; exit 2 ;;
  esac
done
case "$SEG" in main|var4) ;; *) echo "--segment main|var4" >&2; exit 2 ;; esac
[ -n "$OUT" ] || { echo "--out DIR" >&2; exit 2; }
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)

# ---- CPU sets (req. 4): from the environment, else ffi/campaign.machine ------------------
[ -z "${AK_CPU_CLIENT:-}${AK_CPU_SERVER:-}" ] && . "$FFI/campaign.machine"
: "${AK_CPU_CLIENT:?AK_CPU_CLIENT}" "${AK_CPU_SERVER:?AK_CPU_SERVER}"
export AK_CPU_CLIENT AK_CPU_SERVER
TOPO=$(python3 - "$AK_CPU_CLIENT" "$AK_CPU_SERVER" <<'PY'
import glob, os, sys
def rng(s):
    o = set()
    for p in s.split(","):
        if "-" in p: a, b = p.split("-"); o.update(range(int(a), int(b) + 1))
        elif p: o.add(int(p))
    return o
cl, sv = rng(sys.argv[1]), rng(sys.argv[2])
bad = []
if cl & sv: bad.append("the sets overlap: %s" % sorted(cl & sv))
for c in cl:
    try: sib = rng(open("/sys/devices/system/cpu/cpu%d/topology/thread_siblings_list" % c).read().strip())
    except OSError: sib = {c}
    if (sib - {c}) & sv: bad.append("cpu %d (CLIENT) is an SMT sibling of %s (SERVER)" % (c, sorted((sib - {c}) & sv)))
nodes = {os.path.basename(n) for c in cl | sv for n in glob.glob("/sys/devices/system/cpu/cpu%d/node*" % c)}
if len(nodes) > 1: bad.append("the sets span NUMA nodes %s" % sorted(nodes))
print("; ".join(bad))
PY
)
[ -z "$TOPO" ] || { echo "REFUSED (req. 4): $TOPO" >&2; exit 2; }

# ---- the segment ---------------------------------------------------------------------------
MODE=retain   # C and D in retain mode, as the Rust slice's WP11 probes (Cf-retain, Df-retain)
T=pinned
if [ "$SEG" = main ]; then
  WK=8; SPREAD_PASSES=4; MAIN_PASSES=3
  SPREAD_CELLS="A,D-$MODE,Cf-$MODE"; MAIN_CELLS="A,D-$MODE,Cf-$MODE,Cf-q-$MODE,C-$MODE,C-q-$MODE"
else
  WK=4; SPREAD_PASSES=3; MAIN_PASSES=0
  SPREAD_CELLS="A,D-$MODE,Cf-$MODE,Cf-q-$MODE,C-$MODE,C-q-$MODE"; MAIN_CELLS=""
fi
PAYLOADS="P5.4,4MiB,16MiB"; KS=1,8
ROUNDS=10; WARM_S=0.05
# Fixed iterations (batches of k calls) per repetition, per (payload, k): about 100 ms of wall
# per repetition on this machine (190 ms and 3 batches for 16 MiB at k = 8), from the smoke's
# per-call wall (2026-09-29); gen/physical_probe.py prints the batches and the wall per
# repetition of every entry.
ITERS="P5.4/1:30,P5.4/8:5,4MiB/1:40,4MiB/8:6,16MiB/1:12,16MiB/8:3"
if [ $SMOKE = 1 ]; then
  SPREAD_PASSES=1; [ $MAIN_PASSES -gt 0 ] && MAIN_PASSES=1; ROUNDS=2; WARM_S=0.01
  ITERS="P5.4/1:3,P5.4/8:2,4MiB/1:3,4MiB/8:2,16MiB/1:2,16MiB/8:1"
fi
B=${BUILD:-$HERE/build-campaign}
EXE=$B/campaign_rpc
SERVE=$FFI/poc/rust/serve.sh
LOG=$OUT/runner.log; : > "$LOG"
say() { echo "$*" | tee -a "$LOG"; }
T0=$(date +%s)

# ---- the tree and the build -----------------------------------------------------------------
DIRTY=$(git -C "$REPO" status --porcelain -- ffi/poc/cpp/src ffi/poc/cpp/include ffi/poc/cpp/CMakeLists.txt ffi/poc/cpp/gen ffi/poc/codec ffi/schema 2>/dev/null)
if [ -n "$DIRTY" ] && [ $SMOKE = 0 ] && [ "${AK_ALLOW_DIRTY:-0}" != 1 ]; then
  say "REFUSED: the tree is dirty (req. 27):"; echo "$DIRTY" | head -20 | tee -a "$LOG"; exit 2
fi
if command -v cmake > /dev/null; then BUILDCMD=(bash -c); else BUILDCMD=(nix-shell gen/shell.nix --run); fi
"${BUILDCMD[@]}" "cmake --build '$B' -j$(nproc) --target campaign_rpc" > "$OUT/build.log" 2>&1 \
  || { tail -20 "$OUT/build.log"; say "build failed (build.log)"; exit 1; }
CORE=$(ldd "$EXE" | grep -o '/[^ ]*libak_core\.so' | head -1)
case "$CORE" in "$HERE"/core-build/target-camp/release/libak_core.so) ;; *) say "REFUSED: $EXE loads $CORE, not the campaign core"; exit 1 ;; esac
[ "$(nm -D --defined-only "$CORE" | grep -cE ' (ak_uencode_|ak_uelem|ak_dec_reset_)')" != 0 ] \
  && [ "$(nm -D --defined-only "$CORE" | grep -c ' ak_call_open$')" = 1 ] || { say "REFUSED: $CORE is not the rpc + unknown-fields core"; exit 1; }
CORE_SHA=$(sha256sum "$CORE" | cut -d' ' -f1); EXE_SHA=$(sha256sum "$EXE" | cut -d' ' -f1)
PRELOAD=""
if [ -n "$GCPUS" ]; then
  gcc -O2 -shared -fPIC -o "$OUT/ncpus_shim.so" gen/ncpus_shim.c -ldl || { say "ncpus shim build failed"; exit 1; }
  PRELOAD=$OUT/ncpus_shim.so
fi

# ---- the server ------------------------------------------------------------------------------
SCRATCH=$(mktemp -d)
OWN=0
if [ -z "${AK_SERVE_STATE:-}" ]; then
  OWN=1; export AK_SERVE_STATE=$SCRATCH/serve.state
  bash "$SERVE" build > "$SCRATCH/sb.log" 2>&1 || { cat "$SCRATCH/sb.log"; say "serve.sh build failed"; exit 1; }
  AK_SERVER_THREADS=$WK bash "$SERVE" start --out "$OUT/server" > "$SCRATCH/srv.out" 2>&1 || { cat "$SCRATCH/srv.out"; say "the server did not start"; exit 1; }
  bash "$SERVE" warm 50 > "$OUT/server-warm.log" 2>&1 || { say "server warm-up failed"; bash "$SERVE" stop; exit 1; }
fi
cleanup() { [ $OWN = 1 ] && [ -f "$AK_SERVE_STATE" ] && bash "$SERVE" stop > /dev/null 2>&1; rm -rf "$SCRATCH"; true; }
trap cleanup EXIT
[ -f "$AK_SERVE_STATE" ] || { say "REFUSED: no server state file $AK_SERVE_STATE"; exit 2; }
SPID=$(sed -n 's/^pid //p' "$AK_SERVE_STATE"); SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE")
[ -n "$SPID" ] && [ -S "$SOCK" ] || { say "REFUSED: $AK_SERVE_STATE names no pid or no pinned socket"; exit 2; }
EXP=${AK_EXPECT_P22:-540422}   # SERVER.md; every client process checks it (pre-check and every call)
server_check() {  # the pid unchanged, alive, pinned to AK_CPU_SERVER, AK_SERVER_THREADS = WK
  python3 - "$SPID" "$AK_CPU_SERVER" "$WK" "$(sed -n 's/^pid //p' "$AK_SERVE_STATE")" <<'PY'
import sys
pid, want, wk, now = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
def rng(s):
    o = set()
    for p in s.split(","):
        if "-" in p: a, b = p.split("-"); o.update(range(int(a), int(b) + 1))
        elif p: o.add(int(p))
    return o
bad = []
if now != pid: bad.append("the state file's pid changed: %s -> %s" % (pid, now))
try:
    st = open("/proc/%s/status" % pid).read()
    aff = [l.split(":", 1)[1].strip() for l in st.splitlines() if l.startswith("Cpus_allowed_list")][0]
    if rng(aff) != rng(want): bad.append("server affinity %s, not AK_CPU_SERVER %s" % (aff, want))
    env = dict(kv.split("=", 1) for kv in open("/proc/%s/environ" % pid).read().split("\0") if "=" in kv)
    if env.get("AK_SERVER_THREADS", "4") != wk: bad.append("server AK_SERVER_THREADS=%s, the segment needs %s" % (env.get("AK_SERVER_THREADS", "4 (serve.sh default)"), wk))
except OSError as e:
    bad.append("server pid %s: %s" % (pid, e))
print("; ".join(bad))
sys.exit(1 if bad else 0)
PY
}
m=$(server_check) || { say "REFUSED before any timing: $m"; exit 2; }

# ---- the header --------------------------------------------------------------------------------
NB_SPREAD=$(( $(echo "$SPREAD_CELLS" | tr ',' '\n' | wc -l) * 6 )); NB_MAIN=$(( $(echo "${MAIN_CELLS:-x}" | tr ',' '\n' | wc -l) * 6 ))
{
  echo "# {\"physical_probe\": {\"slice\": \"cpp\", \"segment\": \"$SEG\", \"smoke\": $([ $SMOKE = 1 ] && echo true || echo false), \"instrumentation\": $([ $SMOKE = 1 ] || [ -n "$DIRTY" ] && echo true || echo false), \"commit\": \"$(git -C "$REPO" rev-parse HEAD)\", \"dirty\": $([ -n "$DIRTY" ] && echo true || echo false), \"date\": \"$(date -u +%FT%TZ)\","
  echo "#   \"server\": {\"mode\": \"$([ $OWN = 1 ] && echo "own: serve.sh start with AK_SERVER_THREADS=$WK, serve.sh warm 50" || echo "shared: started by the owner, never started, stopped or warmed here")\", \"state\": \"$AK_SERVE_STATE\", \"pid\": $SPID, \"socket\": \"unix:$SOCK (pinned)\", \"expected_workers\": $WK, \"p22_bytes\": $EXP},"
  echo "#   \"client\": {\"exe\": \"${EXE#$HERE/}\", \"exe_sha256\": \"$EXE_SHA\", \"core\": \"${CORE#$HERE/}\", \"core_sha256\": \"$CORE_SHA\", \"core_features\": \"rpc,init-guard,unknown-fields\", \"core_runtime_workers\": $WK, \"caller_threads\": 8, \"grpc_cpus_shim\": \"$([ -n "$GCPUS" ] && echo "sysconf(_SC_NPROCESSORS_*) = $GCPUS (gen/ncpus_shim.c preloaded)" || echo "off: grpc-core sizes itself from sysconf(_SC_NPROCESSORS_CONF)")\", \"taskset\": \"$AK_CPU_CLIENT\"},"
  echo "#   \"grid\": {\"transport\": \"$T\", \"mode\": \"$MODE\", \"dirs\": \"c (P5.4), d (4MiB, 16MiB)\", \"inflight\": \"$KS\", \"spread_passes\": $SPREAD_PASSES, \"spread_cells\": \"$SPREAD_CELLS\", \"main_passes\": $MAIN_PASSES, \"main_cells\": \"$MAIN_CELLS\", \"order\": \"spread passes S1..S$SPREAD_PASSES, then main passes M1..M$MAIN_PASSES; one client process per pass; launch index = pass number (1..), rotating Google Benchmark's registration order; --benchmark_enable_random_interleaving within a pass\", \"repetitions_per_pass\": $ROUNDS, \"warmup_s_per_benchmark\": $WARM_S, \"fixed_iterations\": \"$ITERS\", \"benchmarks_per_pass\": {\"spread\": $NB_SPREAD, \"main\": $([ $MAIN_PASSES -gt 0 ] && echo $NB_MAIN || echo 0)}},"
  echo "#   \"spread\": \"a pass = one client process; per pass and workload the in-process gap of a cell = median(cell) - median(A) of per-call client CPU (and wall); the spread of D - A and Cf - A = max - min of that gap over the segment's spread passes; absolute times only\"}}"
  echo "# machine $(python3 gen/machine_facts.py "$AK_CPU_CLIENT" "$AK_CPU_SERVER" "$SPID")"
  echo "# toolchain {\"cxx\": \"$(g++ --version | head -1)\", \"grpcpp\": \"$(ldd "$EXE" | grep -o 'grpc-[0-9.]*' | head -1)\", \"protobuf\": \"$(ldd "$EXE" | grep -o 'protobuf-[0-9.]*' | head -1)\", \"rustc\": \"$(rustc --version)\", \"google_benchmark\": \"v1.8.3 (344117638c8f, Release)\"}"
} > "$OUT/header.txt"
cat "$OUT/header.txt" >> "$LOG"
T1=$(date +%s)

# ---- the passes --------------------------------------------------------------------------------
run_pass() {  # run_pass KIND N LAUNCH CELLS
  local kind=$1 n=$2 launch=$3 cells=$4 m
  m=$(server_check) || { say "ABORTED before pass $kind$n: $m"; exit 2; }
  local tag=$kind$n C=$OUT/pass-$1$2.console G=$SCRATCH/g.json F=$OUT/pass-$1$2.jsonl s=$(date +%s)
  rm -f "$G"
  env ${PRELOAD:+LD_PRELOAD=$PRELOAD AK_SHIM_NCPUS=$GCPUS} timeout 1200 taskset -c "$AK_CPU_CLIENT" "$EXE" \
    --target "unix:$SOCK" --expect "$EXP" --transport $T --cells "$cells" --dirs cd --payloads "$PAYLOADS" \
    --inflight $KS --launch "$launch" --rounds $ROUNDS --warmup-s $WARM_S --min-time-s 0.05 --iters "$ITERS" \
    --workers $WK --gbench-out "$G" > "$C" 2>&1; local rc=$?
  if [ $rc != 0 ] || [ ! -s "$G" ]; then
    say "pass $tag ABORTED (exit $rc; req. 18): no figure; $(grep -m1 -E 'CALL CHECK|pre-check|unknown' "$C")"; exit 1
  fi
  m=$(server_check) || { say "pass $tag: the server changed during the pass ($m): its samples are discarded"; rm -f "$F"; exit 2; }
  { echo "# pass $tag (launch $launch, cells $cells), $(( $(date +%s) - s )) s, client pid-independent header below"
    echo "# machine_end $(python3 gen/machine_facts.py "$AK_CPU_CLIENT" "$AK_CPU_SERVER" "$SPID")"
    grep '^#' "$C"
    python3 gen/gbench_to_jsonl.py "$G" "$launch" full rpc | sed -E "s/^\{/{\"segment\":\"$SEG\",\"pass\":\"$tag\",/"; } > "$F" \
    || { say "pass $tag: the Google Benchmark output was refused"; exit 1; }
  gzip -9c "$G" > "$OUT/pass-$tag.gbench.json.gz"
  say "pass $tag: $(grep -c '^{' "$F") samples, $(( $(date +%s) - s )) s"
}
L=0
for i in $(seq 1 $SPREAD_PASSES); do L=$((L + 1)); run_pass S "$i" $L "$SPREAD_CELLS"; done
for i in $(seq 1 $MAIN_PASSES); do L=$((L + 1)); run_pass M "$i" $L "$MAIN_CELLS"; done
say "benchmark wall time (passes only): $(( $(date +%s) - T1 )) s; with build and checks: $(( $(date +%s) - T0 )) s"
[ $OWN = 1 ] && cp "$OUT/server/rpc-server.log" "$OUT/rpc-server.log" 2>/dev/null
python3 gen/physical_probe.py "$OUT" > "$OUT/tables.md" && say "tables: $OUT/tables.md"
