#!/usr/bin/env bash
# The rust slice's SHORT optimisation benchmark (harness v5, after the merge with
# origin/rust/native-core-ffi-poc): one fixed run of about 8 to 10 minutes of benchmark wall
# time in this container, repeated unchanged so two runs compare. It follows the merged
# CAMPAIGN harness (run_campaign.sh) exactly -- the same codec suite on criterion with
# process CPU (req 21 as amended, R-H25), the same arms, encode variants (req 11), the same
# seeded random arm-block and case order (req 22 as amended, R-H23), the same RPC grid
# (cells A-F, one server per transport, Unix socket, a/a+read/b) and calib -- with fewer
# samples, one launch and no gate. It is NOT the campaign and every figure it writes is
# CONTAINER INSTRUMENTATION.
#
#   gen/opt_bench.sh OUT_DIR
#
# Order of work (each step's wall time in OUT_DIR/runner.log):
#   1. build: run_campaign.sh's build() -- target/ (full) and target-nounk/ (no-unknown),
#      the bench executables from `cargo bench --no-run`, the variant check per binary.
#   2. crossings (req 19): both counting builds against the committed files, RECORDED, not
#      fatal (OPT_CROSSINGS=0 skips).
#   3. codec, launch 1: the payload inputs (AK_ONLY=P) on the full build, then on the
#      no-unknown build; then the U-* rows (AK_ONLY=U-) on each, at the reduced U settings.
#      Each process runs its own pre-check before criterion and exits 2 on a failure.
#      Labelled extra arm core-ffi-zc on P5.* (AK_ZC), run after the arm blocks.
#   4. calib, one launch.
#   5. rpc, launch 1: per transport, the planted-wrong-length control per client binary
#      (must abort with no file), then ONE server that both clients call (full first).
#   6. summary: gen/opt_summary.py (per-case TSV, variants tables, U retain-vs-drop).
# No gate is run (the owner's instruction for these measurements).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$HERE"
[ $# = 1 ] || { echo "usage: $0 OUT_DIR" >&2; exit 2; }
mkdir -p "$1"; OUT="$(cd "$1" && pwd)"

# ---- fixed settings (change them and the run no longer compares) ----------------------
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1} AK_CPU_SERVER=${AK_CPU_SERVER:-2,3}
P_ONLY=P;  P_SAMPLES=10; P_WARMUP_ITERS=20; P_WARMUP_MS=20; P_MEASURE_MS=60
U_ONLY=U-; U_SAMPLES=10; U_WARMUP_ITERS=5;  U_WARMUP_MS=3;  U_MEASURE_MS=10
export AK_NRESAMPLES=1000 AK_ZC=P5.
export AK_SERVER_THREADS=${AK_SERVER_THREADS:-2}
CALIB_ROUNDS=5; CALIB_ITERS=20000000
RPC_ROUNDS=3; RPC_CALLS=24; RPC_WARM=16; RPC_SWARM=16
LAUNCH=1
SETTINGS="harness v5 (the merged campaign harness); codec engine=criterion, process CPU, arm blocks and cases in the seeded random order of launch $LAUNCH, criterion_resamples=$AK_NRESAMPLES, AK_POOL_BYTES=${AK_POOL_BYTES:-default (2 x AK_LLC_BYTES)}; codec P(${P_ONLY}*): samples=$P_SAMPLES warmup_iters=$P_WARMUP_ITERS warmup_ms=$P_WARMUP_MS measure_ms=$P_MEASURE_MS; core-ffi-zc extra arm on $AK_ZC*; codec U(${U_ONLY}*): samples=$U_SAMPLES warmup_iters=$U_WARMUP_ITERS warmup_ms=$U_WARMUP_MS measure_ms=$U_MEASURE_MS; calib rounds=$CALIB_ROUNDS iters=$CALIB_ITERS; rpc rounds=$RPC_ROUNDS calls=$RPC_CALLS warmup=$RPC_WARM server-warm=$RPC_SWARM server threads=$AK_SERVER_THREADS; launch=$LAUNCH"

SCRATCH=$(mktemp -d)
LOG="$OUT/runner.log"; : > "$LOG"
say() { echo "$*" | tee -a "$LOG"; }
T0=$(date +%s)
step() { say "[$(( $(date +%s) - T0 ))s] $*"; }

REV=$(git rev-parse --short HEAD)
DIRTY=""
if ! git diff --quiet HEAD -- . ../codec ../../schema ../../corpus || [ -n "$(git status --porcelain -- . ../codec)" ]; then
  DIRTY=" + UNCOMMITTED CHANGES"
fi
TREE=$( (git rev-parse HEAD; git diff HEAD -- . ../codec ../../schema ../../corpus) | sha256sum | cut -c1-16)
sysf() { cat "$1" 2>/dev/null || echo "n/a"; }
header() {  # header SUITE [VARIANT]: run_campaign.sh's requirement-27 header, plus the settings
  local variant=${2:-full}
  echo "# rust slice OPTIMISATION BENCHMARK (gen/opt_bench.sh), suite $1, core variant $variant"
  echo "# CONTAINER INSTRUMENTATION: not a campaign result (CAMPAIGN.md section 2), not gated; the codec process's own pre-check is on"
  echo "# commit     $REV$DIRTY   tree-id $TREE"
  echo "# date       $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# machine    $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); $(nproc) CPUs online; kernel $(uname -r)"
  echo "# smt        active=$(sysf /sys/devices/system/cpu/smt/active) control=$(sysf /sys/devices/system/cpu/smt/control)"
  echo "# governor   $(sysf /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor)"
  echo "# turbo      intel_pstate/no_turbo=$(sysf /sys/devices/system/cpu/intel_pstate/no_turbo) cpufreq/boost=$(sysf /sys/devices/system/cpu/cpufreq/boost)"
  echo "# isolation  cmdline: $(tr ' ' '\n' < /proc/cmdline | grep -E '^(isolcpus|nohz_full|rcu_nocbs)=' | tr '\n' ' ' || true)cgroup: $(sysf /sys/fs/cgroup/cpuset.cpus.effective)"
  echo "# cpu sets   CLIENT=$AK_CPU_CLIENT SERVER=$AK_CPU_SERVER OS=the rest"
  echo "# threads    codec: 1 measuring thread; rpc client: tokio 2 workers per A/D/F cell, ak_runtime_new(2) per B/C/E client, k = 1/8/16 callers; rpc server: tokio $AK_SERVER_THREADS workers; calib: 1 thread"
  echo "# runtime    $(rustc --version); $(cargo --version)"
  echo "# incumbent  prost $(awk '/^name = "prost"$/{getline; print $3}' Cargo.lock | tr -d '"'), tonic $(awk '/^name = "tonic"$/{getline; print $3}' Cargo.lock | tr -d '"'), tonic-prost $(awk '/^name = "tonic-prost"$/{getline; print $3}' Cargo.lock | tr -d '"'); criterion $(awk '/^name = "criterion"$/{getline; print $3}' Cargo.lock | tr -d '"')"
  echo "# build      cargo --release (opt-level 3, lto off, codegen-units default), core ak-core as a cdylib linked through the dynamic linker, core features $( [ "$variant" = nounk ] && echo "rpc,init-guard WITHOUT unknown-fields (the no-unknown variant, target-nounk/)" || echo "rpc,init-guard,unknown-fields (the full variant, target/)"); harness guard on; transcoder ak_tc_utf8_trusted (a Rust String is UTF-8)"
  echo "# repeats    launches=1 (launch number $LAUNCH)"
  echo "# settings   $SETTINGS"
}
header runner > "$OUT/header.txt"
cat "$OUT/header.txt" >> "$LOG"

# ---- 1. build: run_campaign.sh's build() ------------------------------------------------
bench_exe() {
  cargo bench -q -p campaign "$@" --bench codec_suite --no-run --message-format=json 2>/dev/null \
    | python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1
}
NOUNK_FEATURES=(--no-default-features --features init-guard)
step "build: full (target/) and no-unknown (target-nounk/)"
cargo build --release -q -p campaign --bins 2>/dev/null
BENCH=$(bench_exe)
CARGO_TARGET_DIR="$HERE/target-nounk" cargo build --release -q -p campaign "${NOUNK_FEATURES[@]}" --bins 2>/dev/null
BENCH_NOUNK=$(CARGO_TARGET_DIR="$HERE/target-nounk" bench_exe "${NOUNK_FEATURES[@]}")
[ -x "$BENCH" ] && [ -x "$BENCH_NOUNK" ] || { echo "no codec bench executable" >&2; exit 1; }
for b in "$BENCH:0" "$BENCH_NOUNK:1" "target/release/rpc_client:0" "target-nounk/release/rpc_client:1"; do
  exe=${b%:*}; want=${b##*:}
  so=$(ldd "$exe" | grep -o '/[^ ]*libak_core.so')
  n=$(nm -D --defined-only "$so" | grep -c ' T ak_uencode_' || true)
  if { [ "$want" = 1 ] && [ "$n" != 0 ]; } || { [ "$want" = 0 ] && [ "$n" = 0 ]; }; then
    echo "variant mix-up: $exe loads $so ($n ak_uencode_* exports)" >&2; exit 1
  fi
  say "  variant ok: $exe -> $so ($n ak_uencode_* exports)"
done

# ---- 2. crossings (recorded, not fatal) -------------------------------------------------
if [ "${OPT_CROSSINGS:-1}" = 1 ]; then
  step "crossings: counting builds, both variants"
  CARGO_TARGET_DIR="$HERE/target-count" cargo run --release -q -p campaign --features count --bin crossings \
    2>/dev/null > "$OUT/crossings-current.txt" || say "  crossings (full) FAILED to run"
  CARGO_TARGET_DIR="$HERE/target-count-nounk" cargo run --release -q -p campaign --no-default-features \
    --features count,init-guard --bin crossings 2>/dev/null > "$OUT/crossings-nounk-current.txt" || say "  crossings (nounk) FAILED to run"
  for v in "" -nounk; do
    if diff -u "gen/crossings$v.txt" "$OUT/crossings$v-current.txt" > "$OUT/crossings$v.diff"; then
      say "  crossings$v: $(grep -vc '^#' "$OUT/crossings$v-current.txt") rows identical to gen/crossings$v.txt"
      rm -f "$OUT/crossings$v.diff" "$OUT/crossings$v-current.txt"
    else
      say "  crossings$v: DIFFER from gen/crossings$v.txt (kept: crossings$v.diff, crossings$v-current.txt)"
    fi
  done
fi

# ---- 3. codec ---------------------------------------------------------------------------
codec_run() {  # codec_run TAG VARIANT EXE ONLY SAMPLES WARM_ITERS WARM_MS MEAS_MS
  local tag=$1 v=$2 exe=$3
  local F="$OUT/$tag.jsonl" C="$OUT/$tag.criterion.log"
  header codec "$v" > "$F.head"
  echo "# this file  AK_ONLY=$4 AK_SAMPLES=$5 AK_WARMUP_ITERS=$6 AK_WARMUP_MS=$7 AK_MEASURE_MS=$8 AK_LAUNCH=$LAUNCH AK_NRESAMPLES=$AK_NRESAMPLES AK_ZC=$AK_ZC AK_POOL_BYTES=${AK_POOL_BYTES:-default}" >> "$F.head"
  local t=$(date +%s)
  CRITERION_HOME="$SCRATCH/criterion-$tag" AK_LAUNCH=$LAUNCH AK_OUT="$F.body" \
    AK_ONLY=$4 AK_SAMPLES=$5 AK_WARMUP_ITERS=$6 AK_WARMUP_MS=$7 AK_MEASURE_MS=$8 \
    taskset -c "$AK_CPU_CLIENT" "$exe" > "$C" 2>&1 \
    || { say "codec $tag FAILED (pre-check or run): $C"; exit 1; }
  { cat "$F.head"; echo "# criterion's console output (its own summary; the samples are in $(basename "$F"))"; cat "$C"; } > "$C.tmp"
  mv "$C.tmp" "$C"
  cat "$F.head" "$F.body" > "$F"; rm -f "$F.head" "$F.body"
  rm -rf "$SCRATCH/criterion-$tag"
  say "  codec $tag: $(grep -m1 '^# precheck:' "$C" | sed 's/^# //'); $(grep -vc '^#' "$F") sample rows; $(( $(date +%s) - t ))s -> $(basename "$F")"
}
step "codec: payloads, full build"
codec_run codec-P full "$BENCH" "$P_ONLY" $P_SAMPLES $P_WARMUP_ITERS $P_WARMUP_MS $P_MEASURE_MS
step "codec: payloads, no-unknown build"
codec_run codec-nounk-P nounk "$BENCH_NOUNK" "$P_ONLY" $P_SAMPLES $P_WARMUP_ITERS $P_WARMUP_MS $P_MEASURE_MS
step "codec: U-* rows, full build (reduced settings)"
codec_run codec-U full "$BENCH" "$U_ONLY" $U_SAMPLES $U_WARMUP_ITERS $U_WARMUP_MS $U_MEASURE_MS
step "codec: U-* rows, no-unknown build (reduced settings)"
codec_run codec-nounk-U nounk "$BENCH_NOUNK" "$U_ONLY" $U_SAMPLES $U_WARMUP_ITERS $U_WARMUP_MS $U_MEASURE_MS

# ---- 4. calib ---------------------------------------------------------------------------
step "calib"
F="$OUT/calib.jsonl"
header calib > "$F.head"
taskset -c "$AK_CPU_CLIENT" target/release/calib --launch $LAUNCH --rounds $CALIB_ROUNDS --iters $CALIB_ITERS --out "$F.body"
cat "$F.head" "$F.body" > "$F"; rm -f "$F.head" "$F.body"
say "  calib: $(grep -vc '^#' "$F") rows"

# ---- 5. rpc: run_campaign.sh's rpc suite, one launch ------------------------------------
SP=""
cleanup() { [ -n "$SP" ] && kill "$SP" 2>/dev/null && wait "$SP" 2>/dev/null; rm -rf "$SCRATCH"; true; }
trap cleanup EXIT
start_server() {  # start_server T L -> SP, SOCK
  SOCK="$SCRATCH/grid-$1-$2.sock"; local RF="$SCRATCH/ready-$1-$2"; rm -f "$RF" "$SOCK"
  taskset -c "$AK_CPU_SERVER" target/release/rpc_server --transport "$1" --socket "$SOCK" --ready-file "$RF" \
    2> "$OUT/rpc-$1-launch$2.server.log" &
  SP=$!
  for _ in $(seq 100); do [ -s "$RF" ] && break; sleep 0.1; done
  [ -s "$RF" ] || { say "rpc_server ($1, $2) did not start"; exit 1; }
}
CLARGS=(--server-warm "$RPC_SWARM")
for T in shipped pinned; do
  step "rpc: transport $T"
  start_server "$T" plant
  for v in full nounk; do
    CL=target/release/rpc_client; [ "$v" = nounk ] && CL=target-nounk/release/rpc_client
    rm -f "$SCRATCH/plant.jsonl"
    if taskset -c "$AK_CPU_CLIENT" "$CL" --socket "$SOCK" --transport "$T" "${CLARGS[@]}" \
         --rounds 1 --calls 16 --warmup 16 --out "$SCRATCH/plant.jsonl" --plant > "$OUT/rpc-$T-$v-PLANT.log" 2>&1 \
       || [ -e "$SCRATCH/plant.jsonl" ]; then
      say "CONTROL FAILED: the planted wrong length did not abort ($T, $v client)"; exit 1
    fi
    say "  rpc $T ($v client): control aborted with no output: $(tail -1 "$OUT/rpc-$T-$v-PLANT.log")"
  done
  kill $SP; wait $SP 2>/dev/null || true; SP=""
  start_server "$T" "$LAUNCH"
  for v in full nounk; do
    CL=target/release/rpc_client; F="$OUT/rpc-$T.jsonl"
    [ "$v" = nounk ] && { CL=target-nounk/release/rpc_client; F="$OUT/rpc-$T-nounk.jsonl"; }
    header rpc "$v" > "$F.head"
    taskset -c "$AK_CPU_CLIENT" "$CL" --socket "$SOCK" --transport "$T" --launch $LAUNCH "${CLARGS[@]}" \
      --rounds $RPC_ROUNDS --calls $RPC_CALLS --warmup $RPC_WARM --out "$F.body" \
      || { say "rpc $T ($v) ABORTED (requirement 18): no figure"; rm -f "$F.head" "$F.body"; exit 1; }
    cat "$F.head" "$F.body" > "$F"; rm -f "$F.head" "$F.body"
    say "  rpc $T ($v client): $(grep -vc '^#' "$F") rows -> $(basename "$F")"
  done
  kill $SP; wait $SP 2>/dev/null || true; SP=""
done

# ---- 6. summary -------------------------------------------------------------------------
step "summary"
python3 gen/opt_summary.py "$OUT" | tee -a "$LOG"
step "done"
