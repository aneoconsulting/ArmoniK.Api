#!/usr/bin/env bash
# The RPC grid ONLY, for a side-by-side with the Rust slice's gen/rpc_same_machine.sh run on
# the same machine (coordinator unit, 2026-09-28). CONTAINER INSTRUMENTATION. No gate, no
# codec, no counts, no planted controls (every call is still checked, requirement 18, and
# campaign_rpc's own pre-checks run in every process before any benchmark).
#
#   gen/rpc_same_machine.sh OUT_DIR
#
# Matches poc/rust/gen/rpc_same_machine.sh as far as the framework allows:
#   - the shared server (poc/rust/serve.sh: AK_SERVER_THREADS tokio workers, pinned to
#     AK_CPU_SERVER), warmed ONCE with `serve.sh warm $SWARM` before the first client;
#   - the client (campaign_rpc, Google Benchmark) pinned to AK_CPU_CLIENT; transports shipped
#     then pinned, per transport the full client then the no-unknown client; k = 1 and 8;
#   - THREE processes per (transport, client), one per direction group: G1 = a, a+read, b;
#     G2 = c (P5.3, P5.4); G3 = d (4 MiB, 16 MiB);
#   - 10 repetitions per entry; per-repetition time = the Rust run's measurement time per
#     benchmark divided by its 10 samples (criterion's measurement time is for ALL the samples
#     of a benchmark, SamplingMode::Flat: time_per_sample = target / n): G1 and G2 250 ms / 10
#     = 25 ms, G3 500 ms / 10 = 50 ms; warm-up 30 ms per benchmark (AK_WARMUP_MS=30).
# Differences the framework imposes are written into header.txt.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$HERE" || exit 2
[ $# = 1 ] || { echo "usage: $0 OUT_DIR" >&2; exit 2; }
mkdir -p "$1"; OUT="$(cd "$1" && pwd)"
FFI=$(cd "$HERE/../.." && pwd)
REPO=$(git -C "$FFI" rev-parse --show-toplevel)
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1} AK_CPU_SERVER=${AK_CPU_SERVER:-2,3} AK_SERVER_THREADS=${AK_SERVER_THREADS:-4}
SWARM=50; ROUNDS=10; WARM_S=0.03; KS=1,8; LAUNCH=1
G1_S=${G1_S:-0.025}; G2_S=${G2_S:-0.025}; G3_S=${G3_S:-0.05}
B=${BUILD:-$HERE/build-campaign}
SERVE=$FFI/poc/rust/serve.sh
SCRATCH=$(mktemp -d); export AK_SERVE_STATE=$SCRATCH/serve.state
LOG="$OUT/runner.log"; : > "$LOG"
say() { echo "$*" | tee -a "$LOG"; }
cleanup() { [ -f "$AK_SERVE_STATE" ] && bash "$SERVE" stop > /dev/null 2>&1; rm -rf "$SCRATCH"; true; }
trap cleanup EXIT
T0=$(date +%s)

# ---- build (no-op when current), the variant check -------------------------------------
eval "$(sed -n '/^GB_TAG=/,/^}/p' gen/run_campaign.sh)"   # GB_TAG, GB_COMMIT, GBPREFIX, gbench_release
gbench_release > "$SCRATCH/gb.log" 2>&1 || { cat "$SCRATCH/gb.log"; say "Google Benchmark release build failed"; exit 1; }
TARGETS="campaign_rpc campaign_rpc_nounk"
{ cmake -S . -B "$B" -DAK_RPC=ON -Dbenchmark_DIR="$GBPREFIX/lib/cmake/benchmark" \
    && cmake --build "$B" -j"$(nproc)" --target $TARGETS && bash "$SERVE" build; } > "$OUT/build.log" 2>&1 \
  || { tail -20 "$OUT/build.log"; say "build failed (build.log)"; exit 1; }
for t in $TARGETS; do
  so=$(ldd "$B/$t" | grep -o '/[^ ]*libak_core\.so' | head -1)
  n=$(nm -D --defined-only "$so" 2>/dev/null | grep -cE ' (ak_uencode_|ak_uelem|ak_dec_reset_)')
  case $t in *nounk*) want=0 ;; *) want=1 ;; esac
  if { [ $want = 0 ] && [ "$n" != 0 ]; } || { [ $want = 1 ] && [ "$n" = 0 ]; }; then say "variant mix-up: $t loads $so"; exit 1; fi
done
T1=$(date +%s)

# ---- header -----------------------------------------------------------------------------
sysf() { cat "$1" 2>/dev/null || echo "n/a"; }
DIRTY=$(git -C "$REPO" status --porcelain -- ffi/poc/cpp/src ffi/poc/cpp/include ffi/poc/cpp/CMakeLists.txt ffi/poc/codec ffi/poc/rust/crates ffi/schema 2>/dev/null)
{
  echo "# cpp slice RPC grid only (gen/rpc_same_machine.sh), for a side-by-side with logs/rust/opt/rpc-same-machine/ on the same machine"
  echo "# CONTAINER INSTRUMENTATION: not a campaign result; no gate run for this tree (coordinator: no code change, no gate)"
  echo "# commit     $(git -C "$REPO" rev-parse --short HEAD)${DIRTY:+ + UNCOMMITTED CHANGES}"
  echo "# date       $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# machine    $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); $(nproc) CPUs online; kernel $(uname -r)"
  echo "# smt        active=$(sysf /sys/devices/system/cpu/smt/active) control=$(sysf /sys/devices/system/cpu/smt/control)"
  echo "# governor   $(sysf /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor)"
  echo "# cgroup     cpuset.cpus.effective=$(sysf /sys/fs/cgroup/cpuset.cpus.effective)"
  echo "# cpu sets   CLIENT=$AK_CPU_CLIENT (taskset) SERVER=$AK_CPU_SERVER (taskset, serve.sh) OS=the rest"
  echo "# server     serve.sh (poc/rust/SERVER.md): ONE rpc_server process, tokio $AK_SERVER_THREADS workers, both sockets (shipped, pinned); serve.sh warm $SWARM (checked calls per direction a, b, c, d/4 from a tonic and a core client on each socket) before the first client process"
  echo "# client     campaign_rpc (src/campaign_rpc.cpp) on Google Benchmark $GB_TAG ($GB_COMMIT, Release): Repetitions($ROUNDS), MeasureProcessCPUTime + UseRealTime, random interleaving of the repetitions of every benchmark of one process; --benchmark_min_warmup_time=$WARM_S per benchmark; --benchmark_min_time per REPETITION: G1 (a, a+read, b) ${G1_S} s, G2 (c: P5.3, P5.4) ${G2_S} s, G3 (d: 4 MiB, 16 MiB) ${G3_S} s (= the Rust run's AK_MEASURE_MS 250/250/500 over its 10 samples); one iteration = one batch of k calls; per call = repetition CPU (or wall) / (iterations x k)"
  echo "# vs rust    (1) the iteration count is fixed on the FIRST repetition by Google Benchmark growing it until the repetition's WALL time (UseRealTime is the controlling clock) reaches min_time, then reused for the other 9; criterion (Flat) derives it from the warm-up's mean of its measurement (process CPU): here a repetition is >= 25/50 ms of wall, there a sample is about 25/50 ms of client CPU; (2) repetitions of all the benchmarks of one process are randomly interleaved here; criterion runs each benchmark's samples back to back, benchmarks in a seeded random order; (3) cells: C++ has Cp/Dp (the pull decode twins, a and a+read only) and the completion-queue cells (-q: B, C, E and framed twins, req. 16 as amended), no Df/Ff and no callback cells; Rust has Df/Ff and its callback cells (-cb, rpc-same-machine-cb), no Cp/Dp and no queue cells; (4) each process runs campaign_rpc's pre-checks (a decode/re-encode per coded cell against protobuf, c/d request bytes against protobuf, one UploadStreamCheck digest per cell and payload for d, cell A's wire length) before any benchmark: untimed"
  echo "# threads    client: k caller threads created before any timed window (a Pool, one condition variable per caller) for the blocking B/C/E and A/D/F cells; the queue cells (-q) use none: one ak_queue per cell, the benchmark thread issues the batch's k calls and drains the queue itself (ONE drainer, completions matched by tag); the core's runtime ak_runtime_new(2) (campaign_rpc --workers default); grpc-core sizes its own pollers and executor (each process's thread count is in its jsonl's campaign_rpc_end line)"
  echo "# order      per transport (shipped, pinned): full client G1, G2, G3, then no-unknown client G1, G2, G3; launch $LAUNCH (registration order rotated by launch)"
  echo "# compiler   $(g++ --version | head -1); -O2 -g -DNDEBUG -std=c++17, shared libak_core.so, LTO off; $(rustc --version)"
  echo "# incumbent  protobuf $(pkg-config --modversion protobuf 2>/dev/null), grpc++ $(pkg-config --modversion grpc++ 2>/dev/null) (apt)"
  echo "# build      $B: campaign_rpc (core rpc,init-guard,unknown-fields) and campaign_rpc_nounk (core rpc,init-guard); build/check time $((T1 - T0)) s (not in the run time)"
} > "$OUT/header.txt"
cat "$OUT/header.txt" >> "$LOG"

# ---- server ------------------------------------------------------------------------------
bash "$SERVE" start --out "$SCRATCH/srv" > "$SCRATCH/srv.out" 2>&1 || { cat "$SCRATCH/srv.out"; say "the server did not start"; exit 1; }
SOCK_shipped=$(awk '$1=="shipped"{print $2}' "$SCRATCH/srv.out"); SOCK_pinned=$(awk '$1=="pinned"{print $2}' "$SCRATCH/srv.out")
EXP=$(sed -n 's/.*P2.2 \([0-9]*\) B.*/\1/p' "$SCRATCH/srv/rpc-server.log" | head -1)
[ -n "$EXP" ] && [ -n "$SOCK_shipped" ] && [ -n "$SOCK_pinned" ] || { say "the server did not report its sockets"; exit 1; }
say "server: $(awk '$1=="pid"' "$SCRATCH/srv.out")"
bash "$SERVE" warm "$SWARM" > "$OUT/server-warm.log" 2>&1 || { say "server warm-up failed"; cp "$SCRATCH/srv/rpc-server.log" "$OUT/"; exit 1; }
T2=$(date +%s)
for T in shipped pinned; do
  S=$SOCK_shipped; [ $T = pinned ] && S=$SOCK_pinned
  for bld in full no-unknown; do
    exe=campaign_rpc; F="$OUT/rpc-$T.jsonl"; v=full
    [ $bld = no-unknown ] && { exe=campaign_rpc_nounk; F="$OUT/rpc-$T-nounk.jsonl"; v=nounk; }
    : > "$F"
    for g in 1 2 3; do
      case $g in 1) D=arb; M=$G1_S ;; 2) D=c; M=$G2_S ;; 3) D=d; M=$G3_S ;; esac
      C="$OUT/rpc-$T-$v-G$g.console"; G="$SCRATCH/g.json"; s=$(date +%s)
      timeout 1800 taskset -c "$AK_CPU_CLIENT" "$B/$exe" --target "unix:$S" --expect "$EXP" --transport "$T" \
        --dirs "$D" --launch "$LAUNCH" --rounds "$ROUNDS" --min-time-s "$M" --warmup-s "$WARM_S" \
        --inflight "$KS" --gbench-out "$G" > "$C" 2>&1; rc=$?
      if [ $rc != 0 ] || [ ! -s "$G" ]; then
        say "rpc $T $v G$g ABORTED (exit $rc; requirement 18): no figure; $(grep -m1 -E 'CALL CHECK|pre-check' "$C")"
        cp "$SCRATCH/srv/rpc-server.log" "$OUT/"; exit 1
      fi
      { echo "# process: $T, $v client, group G$g (--dirs $D, --min-time-s $M), $(( $(date +%s) - s )) s"
        echo "# this file  $exe --target <$T socket> --expect $EXP --transport $T --dirs $D --launch $LAUNCH --rounds $ROUNDS --min-time-s $M --warmup-s $WARM_S --inflight $KS"
        grep '^#' "$C"
        python3 gen/gbench_to_jsonl.py "$G" "$LAUNCH" "$bld" rpc; } >> "$F" 2> "$SCRATCH/conv.err" \
        || { cat "$SCRATCH/conv.err"; say "rpc $T $v G$g: the Google Benchmark output was refused"; exit 1; }
      gzip -9c "$G" > "$OUT/rpc-$T-$v-G$g.gbench.json.gz"
      say "rpc $T $v G$g ($D): $(python3 gen/gbench_to_jsonl.py "$G" "$LAUNCH" "$bld" rpc | grep -c '^{') sample rows, $(( $(date +%s) - s )) s"
    done
  done
done
bash "$SERVE" stop > /dev/null 2>&1
cp "$SCRATCH/srv/rpc-server.log" "$OUT/rpc-server.log"
say "run time (server start to stop, build excluded): $(( $(date +%s) - T1 )) s (client processes $(( $(date +%s) - T2 )) s)"
python3 gen/rpc_same_machine.py "$OUT" | tee -a "$LOG"
