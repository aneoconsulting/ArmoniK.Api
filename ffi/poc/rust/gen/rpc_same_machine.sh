#!/usr/bin/env bash
# The RPC grid ONLY, for a side-by-side with another slice's grid run on the same machine
# (coordinator unit, 2026-09-28). CONTAINER INSTRUMENTATION. No gate, no codec, no calib,
# no planted controls (every call is still checked, requirement 18, inside the bench).
#
#   gen/rpc_same_machine.sh OUT_DIR
#
# The shared server (serve.sh: AK_SERVER_THREADS tokio workers, pinned to AK_CPU_SERVER,
# warmed with serve.sh warm $SWARM), the client (benches/rpc_suite.rs, criterion) pinned
# to AK_CPU_CLIENT; transports shipped then pinned, per transport the full client then the
# no-unknown client; in-flight k = 1 and 8. Each (transport, client) runs as THREE criterion
# processes, one per direction group, because one measurement time cannot give every group
# several batches per sample inside the budget: G1 = a, a+read, b; G2 = c (P5.3, P5.4);
# G3 = d (4 MiB, 16 MiB). Every direction's cells share one process.
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd)
OUT=${1:?usage: rpc_same_machine.sh OUT_DIR}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
cd "$HERE"
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1} AK_CPU_SERVER=${AK_CPU_SERVER:-2,3} AK_SERVER_THREADS=${AK_SERVER_THREADS:-4}
SWARM=${SWARM:-50}; SAMPLES=10; WARM_MS=30; NRES=1000; KS=1,8
G1_MS=${G1_MS:-250}; G2_MS=${G2_MS:-300}; G3_MS=${G3_MS:-600}
SCRATCH=$(mktemp -d); export AK_SERVE_STATE="$SCRATCH/serve.state"
T0=$(date +%s)
cleanup() { ./serve.sh stop > /dev/null 2>&1 || true; rm -rf "$SCRATCH"; }
trap cleanup EXIT
NOUNK_FEATURES=(--no-default-features --features init-guard)
bench_exe() {
  cargo bench -q -p campaign "$@" --bench rpc_suite --no-run --message-format=json 2>/dev/null \
    | python3 -S -c 'import sys,json
for l in sys.stdin:
    try: o=json.loads(l)
    except Exception: continue
    if o.get("reason")=="compiler-artifact" and o.get("executable") and o["target"]["name"]=="rpc_suite": print(o["executable"])' | tail -1
}
RPCB=$(bench_exe); RPCB_NOUNK=$(CARGO_TARGET_DIR="$HERE/target-nounk" bench_exe "${NOUNK_FEATURES[@]}")
[ -x "$RPCB" ] && [ -x "$RPCB_NOUNK" ] || { echo "no bench executable" >&2; exit 1; }
for b in "$RPCB:0" "$RPCB_NOUNK:1"; do  # the variant, checked on the core each binary loads
  exe=${b%:*}; want=${b##*:}; so=$(ldd "$exe" | grep -o '/[^ ]*libak_core.so')
  n=$(nm -D --defined-only "$so" | grep -c ' T ak_uencode_' || true)
  if { [ "$want" = 1 ] && [ "$n" != 0 ]; } || { [ "$want" = 0 ] && [ "$n" = 0 ]; }; then echo "variant mix-up: $exe" >&2; exit 1; fi
done
./serve.sh build > /dev/null
T1=$(date +%s)
sysf() { cat "$1" 2>/dev/null || echo "n/a"; }
{
  echo "# rust slice RPC grid only (gen/rpc_same_machine.sh), for a side-by-side on the same machine"
  echo "# CONTAINER INSTRUMENTATION: not a campaign result; no gate run for this tree (coordinator: no code change, no gate)"
  echo "# commit     $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + UNCOMMITTED CHANGES')"
  echo "# date       $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# machine    $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); $(nproc) CPUs online; kernel $(uname -r)"
  echo "# smt        active=$(sysf /sys/devices/system/cpu/smt/active) control=$(sysf /sys/devices/system/cpu/smt/control)"
  echo "# governor   $(sysf /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor)"
  echo "# cgroup     cpuset.cpus.effective=$(sysf /sys/fs/cgroup/cpuset.cpus.effective)"
  echo "# cpu sets   CLIENT=$AK_CPU_CLIENT (taskset) SERVER=$AK_CPU_SERVER (taskset, serve.sh) OS=the rest"
  echo "# server     serve.sh (poc/rust/SERVER.md): ONE rpc_server process, tokio $AK_SERVER_THREADS workers, both sockets (shipped, pinned); serve.sh warm $SWARM (checked calls per direction a, b, c, d/4 from a tonic and a core client on each socket) before the first client process"
  echo "# client     benches/rpc_suite.rs, criterion 0.5 with the process-CPU measurement, SamplingMode::Flat; AK_SAMPLES=$SAMPLES AK_WARMUP_MS=$WARM_MS AK_NRESAMPLES=$NRES (criterion's console bootstrap only; the samples are raw) AK_RPC_SERVER_WARMUP=0 AK_RPC_INFLIGHT=$KS; AK_MEASURE_MS per direction group: G1 (a, a+read, b) $G1_MS, G2 (c: P5.3, P5.4) $G2_MS, G3 (d: 4 MiB, 16 MiB) $G3_MS; one criterion iteration = one batch of k calls; per call = sample CPU (or wall) / (batches x k)"
  echo "# threads    client: tokio 2 workers per A/D/F cell runtime, ak_runtime_new(2) per B/C/E core client, k caller threads (B/C/E) or tasks (A/D/F)"
  echo "# order      per transport (shipped, pinned): full client G1, G2, G3, then no-unknown client G1, G2, G3; inside a process criterion's benchmarks in the seeded random order of AK_LAUNCH=1"
  [ -n "${AK_RPC_CELLS:-}" ] && echo "# NARROWED   AK_RPC_CELLS=$AK_RPC_CELLS (a smoke, not the grid)"
  echo "# runtime    $(rustc --version); $(cargo --version)"
  echo "# incumbent  prost $(awk '/^name = "prost"$/{getline; print $3}' Cargo.lock | tr -d '"'), tonic $(awk '/^name = "tonic"$/{getline; print $3}' Cargo.lock | tr -d '"'); criterion $(awk '/^name = "criterion"$/{getline; print $3}' Cargo.lock | tr -d '"')"
  echo "# build      cargo --release (opt-level 3, lto off), ak-core cdylib; full: rpc,init-guard,unknown-fields (target/); no-unknown: rpc,init-guard (target-nounk/); build time $((T1 - T0)) s (not in the run time)"
} > "$OUT/header.txt"
cat "$OUT/header.txt"
o=$(./serve.sh start --out "$SCRATCH/serve")
declare -A SOCK; SOCK[shipped]=$(echo "$o" | sed -n 's/^shipped //p'); SOCK[pinned]=$(echo "$o" | sed -n 's/^pinned //p')
echo "server: $(echo "$o" | sed -n 's/^pid /pid /p')"
./serve.sh warm "$SWARM" > "$OUT/server-warm.log" 2>&1 || { echo "server warm-up failed" >&2; cp "$SCRATCH/serve/rpc-server.log" "$OUT/"; exit 1; }
for T in shipped pinned; do
  for v in full nounk; do
    EXE=$RPCB; F="$OUT/rpc-$T.jsonl"
    [ "$v" = nounk ] && { EXE=$RPCB_NOUNK; F="$OUT/rpc-$T-nounk.jsonl"; }
    : > "$F"
    for g in 1 2 3; do
      case $g in 1) D="a,a+read,b"; M=$G1_MS ;; 2) D=c; M=$G2_MS ;; 3) D=d; M=$G3_MS ;; esac
      C="$OUT/rpc-$T-$v-G$g.criterion.log"; B="$SCRATCH/body.jsonl"; s=$(date +%s)
      env AK_RPC_SOCKET="${SOCK[$T]}" AK_RPC_TRANSPORT="$T" AK_OUT="$B" CRITERION_HOME="$SCRATCH/crit-$T-$v-$g" \
          AK_SAMPLES=$SAMPLES AK_WARMUP_MS=$WARM_MS AK_MEASURE_MS=$M AK_NRESAMPLES=$NRES AK_RPC_SERVER_WARMUP=0 \
          AK_RPC_INFLIGHT=$KS AK_RPC_DIRS="$D" AK_LAUNCH=1 \
          taskset -c "$AK_CPU_CLIENT" "$EXE" > "$C" 2>&1 \
        || { echo "rpc $T $v G$g ABORTED: $(grep -m1 ABORT "$C")" >&2; cp "$SCRATCH/serve/rpc-server.log" "$OUT/"; exit 1; }
      { echo "# process: $T, $v client, group G$g (AK_RPC_DIRS=$D, AK_MEASURE_MS=$M), $(( $(date +%s) - s )) s"; cat "$B"; } >> "$F"
      echo "rpc $T $v G$g ($D): $(grep -vc '^#' "$B") sample rows, $(( $(date +%s) - s )) s"
    done
  done
done
./serve.sh stop > /dev/null
cp "$SCRATCH/serve/rpc-server.log" "$OUT/" 2>/dev/null || true
echo "run time (server start to stop, build excluded): $(( $(date +%s) - T1 )) s"
python3 "$HERE/gen/rpc_same_machine.py" "$OUT"
