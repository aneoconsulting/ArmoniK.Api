#!/usr/bin/env bash
# The rust slice's SHORT optimisation benchmark: one fixed run (about 8 to 10 minutes of
# benchmark wall time in this container) repeated unchanged before and after each
# optimisation, so the runs compare with each other. It is NOT the campaign
# (run_campaign.sh, CAMPAIGN.md) and NOT the correctness gate (gen/gate.sh): every figure it
# writes is CONTAINER INSTRUMENTATION.
#
#   gen/opt_bench.sh OUT_DIR
#
# What it does, in order (each step's wall time goes to OUT_DIR/runner.log):
#   1. build: exactly run_campaign.sh's build() -- the full build in target/, the no-unknown
#      build in target-nounk/, the codec bench executables from `cargo bench --no-run`,
#      and the same check that each binary loads the core of its variant.
#   2. crossings (req 19), both builds, compared with gen/crossings.txt and
#      gen/crossings-nounk.txt. RECORDED, NOT FATAL: an optimisation may move a count on
#      purpose, and the diff is kept in OUT_DIR for the reader. (OPT_CROSSINGS=0 skips it.)
#   3. codec, one launch (AK_LAUNCH=1), the interleaved sampler, four processes, each pinned to
#      AK_CPU_CLIENT: the 20 payload inputs (16 SHAPES payloads + the latin1/wide content
#      sets of P1.2 and P2.2) on the full build, then on the no-unknown build, with the
#      P settings below; then the 92 U-* corpus rows on each build at the REDUCED U
#      settings. Every arm, direction and unknown-field mode of each build is run. Each
#      process runs its own byte-identity pre-check before anything is timed and exits 2 on
#      any failure (no figure).
#   4. calib (forward / forward+reverse crossing), one launch.
#   5. rpc: both transports, both client builds, the planted-wrong-length control per
#      client (must abort with no file), then one launch at the reduced RPC settings.
#   6. summary: gen/opt_summary.py -> summary-codec.tsv, ratios-codec.tsv,
#      summary-rpc.tsv, summary-calib.tsv. Compare two runs with gen/opt_compare.py.
#
# The correctness gate is deliberately not run (owner's instruction for the optimisation
# experiment: it runs once at the end). The tree may be dirty; the header says so.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$HERE"
[ $# = 1 ] || { echo "usage: $0 OUT_DIR" >&2; exit 2; }
mkdir -p "$1"; OUT="$(cd "$1" && pwd)"

# ---- fixed settings (change them and the run no longer compares with the baseline) ----
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1} AK_CPU_SERVER=${AK_CPU_SERVER:-2,3}
# Harness v3 (optimisation step 0). v2 (a seeded shuffle of criterion cases) was run twice
# on one tree (logs/rust/opt/baseline2-v2, baseline2-v2-aa): single cases moved by up to
# +-25 percent between the two processes, with a lag-1 autocorrelation of 0.5-0.8 along the
# run order -- the container's speed drifts over seconds, and criterion times each case's
# samples back to back, so a ratio's two arms were seconds apart. v3 therefore uses the
# codec suite's INTERLEAVED SAMPLER (AK_ORDER=interleave, not criterion): clusters (input,
# direction) in a seeded order (the same seed in every run), and inside a cluster one
# sample of every arm per round, rotated (H2). The U-* rows get 20 samples (H4), four
# payloads carry the labelled extra decode-nodrop rows (H5), and the RPC cells are
# interleaved and rotated per round over 12 rounds (H3).
P_ONLY=P;  P_SAMPLES=20; P_WARMUP_ITERS=100; P_WARMUP_MS=50; P_MEASURE_MS=200
U_ONLY=U-; U_SAMPLES=20; U_WARMUP_ITERS=20;  U_WARMUP_MS=8;  U_MEASURE_MS=45
export AK_ORDER=interleave AK_SEED=1
NODROP=P1.2,P2.2,P4.1,P6.1
CALIB_ROUNDS=5; CALIB_ITERS=20000000
RPC_ROUNDS=12; RPC_CALLS=24; RPC_WARM=16; RPC_ORDER=interleave
LAUNCH=1
SETTINGS="harness v3; codec engine=interleaved sampler (AK_ORDER=$AK_ORDER) seed=$AK_SEED; codec P(${P_ONLY}*): samples=$P_SAMPLES warmup_iters=$P_WARMUP_ITERS warmup_ms=$P_WARMUP_MS measure_ms=$P_MEASURE_MS decode-nodrop extra rows on $NODROP; codec U(${U_ONLY}*): samples=$U_SAMPLES warmup_iters=$U_WARMUP_ITERS warmup_ms=$U_WARMUP_MS measure_ms=$U_MEASURE_MS; calib rounds=$CALIB_ROUNDS iters=$CALIB_ITERS; rpc order=$RPC_ORDER rounds=$RPC_ROUNDS calls=$RPC_CALLS warmup=$RPC_WARM; launch=$LAUNCH"

SCRATCH=$(mktemp -d)
LOG="$OUT/runner.log"; : > "$LOG"
say() { echo "$*" | tee -a "$LOG"; }
T0=$(date +%s)
step() { say "[$(( $(date +%s) - T0 ))s] $*"; }

# ---- header: run_campaign.sh's requirement-27 header, plus this run's settings --------
REV=$(git rev-parse --short HEAD)
DIRTY=""
if ! git diff --quiet HEAD -- . ../codec ../../schema ../../corpus || [ -n "$(git status --porcelain -- . ../codec)" ]; then
  DIRTY=" + UNCOMMITTED CHANGES"
fi
TREE=$( (git rev-parse HEAD; git diff HEAD -- . ../codec ../../schema ../../corpus) | sha256sum | cut -c1-16)
sysf() { cat "$1" 2>/dev/null || echo "n/a"; }
header() {  # header SUITE [VARIANT]
  local variant=${2:-full}
  echo "# rust slice OPTIMISATION BENCHMARK (gen/opt_bench.sh), suite $1, core variant $variant"
  echo "# CONTAINER INSTRUMENTATION: not a campaign result (CAMPAIGN.md section 2), not gated by gen/gate.sh (the gate runs at the end of the experiment); the codec process's own byte-identity pre-check is on"
  echo "# commit     $REV$DIRTY   tree-id $TREE"
  echo "# date       $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# machine    $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); $(nproc) CPUs online; kernel $(uname -r)"
  echo "# smt        active=$(sysf /sys/devices/system/cpu/smt/active) control=$(sysf /sys/devices/system/cpu/smt/control)"
  echo "# governor   $(sysf /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor)"
  echo "# turbo      intel_pstate/no_turbo=$(sysf /sys/devices/system/cpu/intel_pstate/no_turbo) cpufreq/boost=$(sysf /sys/devices/system/cpu/cpufreq/boost)"
  echo "# isolation  cmdline: $(tr ' ' '\n' < /proc/cmdline | grep -E '^(isolcpus|nohz_full|rcu_nocbs)=' | tr '\n' ' ' || true)cgroup: $(sysf /sys/fs/cgroup/cpuset.cpus.effective)"
  echo "# cpu sets   CLIENT=$AK_CPU_CLIENT SERVER=$AK_CPU_SERVER OS=the rest"
  echo "# runtime    $(rustc --version); $(cargo --version)"
  echo "# incumbent  prost $(awk '/^name = "prost"$/{getline; print $3}' Cargo.lock | tr -d '"'), tonic $(awk '/^name = "tonic"$/{getline; print $3}' Cargo.lock | tr -d '"'), tonic-prost $(awk '/^name = "tonic-prost"$/{getline; print $3}' Cargo.lock | tr -d '"'); criterion $(awk '/^name = "criterion"$/{getline; print $3}' Cargo.lock | tr -d '"')"
  echo "# build      cargo --release (opt-level 3, lto off, codegen-units default), core ak-core as a cdylib linked through the dynamic linker, core features $( [ "$variant" = nounk ] && echo "rpc,init-guard WITHOUT unknown-fields (the no-unknown variant, target-nounk/)" || echo "rpc,init-guard,unknown-fields (the full variant, target/)"); harness guard on; transcoder ak_tc_utf8_trusted (a Rust String is UTF-8)"
  echo "# repeats    launches=1 (launch number $LAUNCH)"
  echo "# settings   $SETTINGS"
}
header runner > "$OUT/header.txt"
cat "$OUT/header.txt" >> "$LOG"

# ---- 1. build: run_campaign.sh's build(), verbatim ------------------------------------
bench_exe() {  # bench_exe CARGO-ARGS...
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

# ---- 2. crossings (recorded, not fatal) -----------------------------------------------
if [ "${OPT_CROSSINGS:-1}" = 1 ]; then
  step "crossings: counting builds, both variants"
  CARGO_TARGET_DIR="$HERE/target-count" cargo run --release -q -p campaign --features count --bin crossings \
    2>/dev/null > "$OUT/crossings-current.txt"
  CARGO_TARGET_DIR="$HERE/target-count-nounk" cargo run --release -q -p campaign --no-default-features \
    --features count,init-guard --bin crossings 2>/dev/null > "$OUT/crossings-nounk-current.txt"
  for v in "" -nounk; do
    if diff -u "gen/crossings$v.txt" "$OUT/crossings$v-current.txt" > "$OUT/crossings$v.diff"; then
      say "  crossings$v: $(wc -l < "$OUT/crossings$v-current.txt") rows identical to gen/crossings$v.txt"
      rm -f "$OUT/crossings$v.diff" "$OUT/crossings$v-current.txt"
    else
      say "  crossings$v: DIFFER from gen/crossings$v.txt (kept: crossings$v.diff, crossings$v-current.txt)"
    fi
  done
fi

# ---- 3. codec ---------------------------------------------------------------------------
codec_run() {  # codec_run TAG VARIANT EXE ONLY SAMPLES WARM_ITERS WARM_MS MEAS_MS [NODROP]
  local tag=$1 v=$2 exe=$3 nodrop=${9:-}
  local F="$OUT/$tag.jsonl" C="$OUT/$tag.console.log"
  header codec "$v" > "$F.head"
  echo "# this file  AK_ONLY=$4 AK_SAMPLES=$5 AK_WARMUP_ITERS=$6 AK_WARMUP_MS=$7 AK_MEASURE_MS=$8 AK_LAUNCH=$LAUNCH AK_ORDER=$AK_ORDER AK_SEED=$AK_SEED AK_NODROP=$nodrop" >> "$F.head"
  local t=$(date +%s)
  AK_LAUNCH=$LAUNCH AK_OUT="$F.body" \
    AK_ONLY=$4 AK_SAMPLES=$5 AK_WARMUP_ITERS=$6 AK_WARMUP_MS=$7 AK_MEASURE_MS=$8 AK_NODROP=$nodrop \
    taskset -c "$AK_CPU_CLIENT" "$exe" > "$C" 2>&1 \
    || { say "codec $tag FAILED (pre-check or run): $C"; exit 1; }
  { cat "$F.head"; echo "# the codec process's console output (the samples are in $(basename "$F"))"; cat "$C"; } > "$C.tmp"
  mv "$C.tmp" "$C"
  cat "$F.head" "$F.body" > "$F"; rm -f "$F.head" "$F.body"
  say "  codec $tag: $(grep -m1 '^# precheck:' "$C" | sed 's/^# //'); $(grep -vc '^#' "$F") sample rows; $(( $(date +%s) - t ))s -> $(basename "$F")"
}
step "codec: payloads, full build"
codec_run codec-P full "$BENCH" "$P_ONLY" $P_SAMPLES $P_WARMUP_ITERS $P_WARMUP_MS $P_MEASURE_MS "$NODROP"
step "codec: payloads, no-unknown build"
codec_run codec-nounk-P nounk "$BENCH_NOUNK" "$P_ONLY" $P_SAMPLES $P_WARMUP_ITERS $P_WARMUP_MS $P_MEASURE_MS "$NODROP"
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

# ---- 5. rpc -----------------------------------------------------------------------------
SP=""
cleanup() { [ -n "$SP" ] && kill "$SP" 2>/dev/null && wait "$SP" 2>/dev/null; rm -rf "$SCRATCH"; true; }
trap cleanup EXIT
for T in shipped pinned; do
  step "rpc: transport $T"
  PF="$SCRATCH/port-$T"; rm -f "$PF"
  taskset -c "$AK_CPU_SERVER" target/release/rpc_server --transport "$T" --port-file "$PF" \
    2> "$OUT/rpc-$T-server.log" &
  SP=$!
  for _ in $(seq 100); do [ -s "$PF" ] && break; sleep 0.1; done
  [ -s "$PF" ] || { say "rpc_server ($T) did not start"; exit 1; }
  PORT=$(head -1 "$PF")
  for v in full nounk; do
    CL=target/release/rpc_client; [ "$v" = nounk ] && CL=target-nounk/release/rpc_client
    rm -f "$SCRATCH/plant.jsonl"
    if taskset -c "$AK_CPU_CLIENT" "$CL" --port "$PORT" --transport "$T" \
         --rounds 1 --calls 16 --warmup 16 --out "$SCRATCH/plant.jsonl" --plant > "$OUT/rpc-$T-$v-PLANT.log" 2>&1 \
       || [ -e "$SCRATCH/plant.jsonl" ]; then
      say "CONTROL FAILED: the planted wrong length did not abort ($T, $v client)"; exit 1
    fi
    say "  rpc $T ($v client): control aborted with no output: $(tail -1 "$OUT/rpc-$T-$v-PLANT.log")"
    F="$OUT/rpc-$T.jsonl"; [ "$v" = nounk ] && F="$OUT/rpc-$T-nounk.jsonl"
    header rpc "$v" > "$F.head"
    taskset -c "$AK_CPU_CLIENT" "$CL" --port "$PORT" --transport "$T" --launch $LAUNCH \
      --rounds $RPC_ROUNDS --calls $RPC_CALLS --warmup $RPC_WARM --order $RPC_ORDER --out "$F.body" \
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
