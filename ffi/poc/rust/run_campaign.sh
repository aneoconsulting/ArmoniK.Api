#!/usr/bin/env bash
# design/CAMPAIGN.md requirement 31: the rust slice's campaign runner.
#
#   run_campaign.sh --suite codec|rpc|calib|gate --out DIR
#
# Environment (requirement 4; the runner never picks CPUs itself):
#   AK_CPU_CLIENT   taskset CPU list for the measured process (required for codec, rpc, calib)
#   AK_CPU_SERVER   taskset CPU list for the RPC server (required for rpc)
#   AK_LAUNCHES     process launches per suite (default 3, requirement 23)
#   AK_ROUNDS       rounds per launch for rpc and calib (default 5); the codec suite's rounds
#                   are criterion samples, AK_SAMPLES (default 10, criterion's floor)
#   AK_SMOKE=1      requirement 32's smoke run: 1 launch, 1 round (codec: 10 samples, the
#                   criterion floor), reduced iterations
#   AK_ONLY         codec: comma-separated input id prefixes (narrows a launch)
#   AK_ALLOW_DIRTY=1  run on a dirty tree (recorded in every header; requirement 27 refuses
#                   a dirty tree, so the campaign never sets it)
#
# Order of work: the correctness gate (gen/gate.sh: byte identity, the full corpus in both
# unknown-field modes with its controls, crossing counts) must have PASSED for this exact
# tree before any timing suite runs (requirement 26); the crossing-count gate (19) runs again
# before codec and calib and stops the run on any difference from gen/crossings.txt.
# Every log starts with the header of requirement 27; samples are JSON lines (28), one file
# per suite, transport and launch (29).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE"
SUITE=""; OUT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --suite) SUITE="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    *) echo "usage: $0 --suite codec|rpc|calib|gate --out DIR" >&2; exit 2 ;;
  esac
done
[ -n "$SUITE" ] && [ -n "$OUT" ] || { echo "usage: $0 --suite codec|rpc|calib|gate --out DIR" >&2; exit 2; }
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
LAUNCHES=${AK_LAUNCHES:-3}; ROUNDS=${AK_ROUNDS:-5}
if [ "${AK_SMOKE:-0}" = 1 ]; then LAUNCHES=1; ROUNDS=1; fi
SCRATCH=${AK_SCRATCH:-$(mktemp -d)}

# ---- requirement 27: the header, and the dirty-tree refusal ---------------------------
REV=$(git rev-parse --short HEAD)
DIRTY=""
if ! git diff --quiet HEAD -- . ../codec ../../schema ../../corpus || [ -n "$(git status --porcelain -- . ../codec)" ]; then
  DIRTY=" + UNCOMMITTED CHANGES"
  if [ "${AK_ALLOW_DIRTY:-0}" != 1 ]; then
    echo "refused: the tree is dirty (requirement 27); commit first, or AK_ALLOW_DIRTY=1 for a non-campaign run" >&2
    exit 1
  fi
fi
TREE=$( (git rev-parse HEAD; git diff HEAD -- . ../codec ../../schema ../../corpus) | sha256sum | cut -c1-16)
sysf() { cat "$1" 2>/dev/null || echo "n/a"; }
header() {  # header SUITE
  echo "# rust slice campaign runner, suite $1"
  echo "# INSTRUMENTATION unless on the campaign machine of CAMPAIGN.md section 2 (a container figure is not a result)"
  echo "# commit     $REV$DIRTY   tree-id $TREE"
  echo "# date       $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# machine    $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); $(nproc) CPUs online; kernel $(uname -r)"
  echo "# smt        active=$(sysf /sys/devices/system/cpu/smt/active) control=$(sysf /sys/devices/system/cpu/smt/control)"
  echo "# governor   $(sysf /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor)"
  echo "# turbo      intel_pstate/no_turbo=$(sysf /sys/devices/system/cpu/intel_pstate/no_turbo) cpufreq/boost=$(sysf /sys/devices/system/cpu/cpufreq/boost)"
  echo "# isolation  cmdline: $(tr ' ' '\n' < /proc/cmdline | grep -E '^(isolcpus|nohz_full|rcu_nocbs)=' | tr '\n' ' ' || true)cgroup: $(sysf /sys/fs/cgroup/cpuset.cpus.effective)"
  echo "# cpu sets   CLIENT=${AK_CPU_CLIENT:-unset} SERVER=${AK_CPU_SERVER:-unset} OS=the rest"
  echo "# runtime    $(rustc --version); $(cargo --version)"
  echo "# incumbent  prost $(awk '/^name = "prost"$/{getline; print $3}' Cargo.lock | tr -d '"'), tonic $(awk '/^name = "tonic"$/{getline; print $3}' Cargo.lock | tr -d '"'), tonic-prost $(awk '/^name = "tonic-prost"$/{getline; print $3}' Cargo.lock | tr -d '"'); criterion $(awk '/^name = "criterion"$/{getline; print $3}' Cargo.lock | tr -d '"')"
  echo "# build      cargo --release (opt-level 3, lto off, codegen-units default), core ak-core as a cdylib linked through the dynamic linker, core features rpc,init-guard; harness guard on; transcoder ak_tc_utf8_trusted (a Rust String is UTF-8)"
  echo "# repeats    launches=$LAUNCHES rounds=$ROUNDS smoke=${AK_SMOKE:-0}"
}
cpus_required() {
  for v in "$@"; do
    [ -n "${!v:-}" ] || { echo "refused: $v is not set (requirement 4: the runner takes the CPU sets from the environment)" >&2; exit 1; }
  done
}

# ---- requirement 19: crossing counts against the committed ones -----------------------
crossings() {
  local got="$OUT/crossings-current.txt"
  CARGO_TARGET_DIR="$HERE/target-count" cargo run --release -q -p campaign --features count --bin crossings \
    2>/dev/null > "$got"
  if diff -u gen/crossings.txt "$got" > "$OUT/crossings.diff"; then
    echo "crossing counts: $(wc -l < "$got") rows identical to gen/crossings.txt"
    rm -f "$OUT/crossings.diff" "$got"
  else
    echo "STOP: crossing counts differ from gen/crossings.txt (requirement 19); see $OUT/crossings.diff" >&2
    exit 1
  fi
}

# ---- requirement 26: the gate, passed for THIS tree -----------------------------------
gate_passed() { [ -f "$OUT/gate.ok" ] && [ "$(cat "$OUT/gate.ok")" = "$TREE" ]; }
run_gate() {
  { header gate; echo; bash gen/gate.sh; } > "$OUT/gate.log" 2>&1 || { echo "GATE FAILED: see $OUT/gate.log; no figure is produced" >&2; exit 1; }
  grep -q "GATE PASSED" "$OUT/gate.log" || { echo "GATE did not pass: $OUT/gate.log" >&2; exit 1; }
  { header gate; echo; crossings; } >> "$OUT/gate.log" 2>&1
  echo "$TREE" > "$OUT/gate.ok"
  echo "gate: PASSED ($OUT/gate.log)"
}
need_gate() {
  if ! gate_passed; then
    echo "no gate pass recorded for tree $TREE: running the gate first (requirement 26)"
    run_gate
  fi
}

build() {
  cargo build --release -q -p campaign --bins 2>/dev/null
  BENCH=$(cargo bench -q -p campaign --bench codec_suite --no-run --message-format=json 2>/dev/null \
    | python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1)
  [ -x "$BENCH" ] || { echo "no codec bench executable" >&2; exit 1; }
}

case "$SUITE" in
  gate)
    run_gate ;;

  codec)
    cpus_required AK_CPU_CLIENT
    need_gate
    crossings
    build
    if [ "${AK_SMOKE:-0}" = 1 ]; then
      export AK_SAMPLES=10 AK_WARMUP_ITERS=${AK_WARMUP_ITERS:-3} AK_WARMUP_MS=${AK_WARMUP_MS:-5} AK_MEASURE_MS=${AK_MEASURE_MS:-10}
    fi
    for L in $(seq 1 "$LAUNCHES"); do
      F="$OUT/codec-launch$L.jsonl"
      header codec > "$F.head"
      CRITERION_HOME="$SCRATCH/criterion-launch$L" AK_LAUNCH=$L AK_OUT="$F.body" \
        taskset -c "$AK_CPU_CLIENT" "$BENCH" > "$OUT/codec-launch$L.criterion.log" 2>&1 \
        || { echo "codec launch $L FAILED: $OUT/codec-launch$L.criterion.log" >&2; exit 1; }
      { cat "$F.head"; echo "# criterion's console output (its own summary; the samples are in $(basename "$F"))"; cat "$OUT/codec-launch$L.criterion.log"; } > "$OUT/codec-launch$L.criterion.tmp"
      mv "$OUT/codec-launch$L.criterion.tmp" "$OUT/codec-launch$L.criterion.log"
      cat "$F.head" "$F.body" > "$F"; rm -f "$F.head" "$F.body"
      echo "codec launch $L: $(grep -vc '^#' "$F") sample rows -> $F"
    done ;;

  rpc)
    cpus_required AK_CPU_CLIENT AK_CPU_SERVER
    need_gate
    build
    CALLS=${AK_RPC_CALLS:-96}; WARM=${AK_RPC_WARMUP:-64}
    if [ "${AK_SMOKE:-0}" = 1 ]; then CALLS=16; WARM=16; fi
    for T in shipped pinned; do
      PF="$SCRATCH/port-$T"; rm -f "$PF"
      taskset -c "$AK_CPU_SERVER" target/release/rpc_server --transport "$T" --port-file "$PF" \
        2> "$OUT/rpc-$T-server.log" &
      SP=$!
      for _ in $(seq 100); do [ -s "$PF" ] && break; sleep 0.1; done
      [ -s "$PF" ] || { echo "rpc_server ($T) did not start" >&2; kill $SP; exit 1; }
      PORT=$(head -1 "$PF")
      # Requirement 18's control: a wrong expected length must abort with no figure.
      if taskset -c "$AK_CPU_CLIENT" target/release/rpc_client --port "$PORT" --transport "$T" \
           --rounds 1 --calls 16 --warmup 16 --out "$SCRATCH/plant.jsonl" --plant > "$OUT/rpc-$T-PLANT.log" 2>&1 \
         || [ -e "$SCRATCH/plant.jsonl" ]; then
        echo "CONTROL FAILED: the planted wrong length did not abort ($T)" >&2; kill $SP; exit 1
      fi
      echo "rpc $T: control (planted wrong length) aborted with no output: $(tail -1 "$OUT/rpc-$T-PLANT.log")"
      for L in $(seq 1 "$LAUNCHES"); do
        F="$OUT/rpc-$T-launch$L.jsonl"
        header rpc > "$F.head"
        taskset -c "$AK_CPU_CLIENT" target/release/rpc_client --port "$PORT" --transport "$T" --launch "$L" \
          --rounds "$ROUNDS" --calls "$CALLS" --warmup "$WARM" --out "$F.body" \
          || { echo "rpc $T launch $L ABORTED (requirement 18): no figure" >&2; kill $SP; rm -f "$F.head" "$F.body"; exit 1; }
        cat "$F.head" "$F.body" > "$F"; rm -f "$F.head" "$F.body"
        echo "rpc $T launch $L: $(grep -vc '^#' "$F") sample rows -> $F"
      done
      kill $SP; wait $SP 2>/dev/null || true
    done ;;

  calib)
    cpus_required AK_CPU_CLIENT
    need_gate
    crossings
    build
    ITERS=${AK_CALIB_ITERS:-20000000}
    [ "${AK_SMOKE:-0}" = 1 ] && ITERS=1000000
    for L in $(seq 1 "$LAUNCHES"); do
      F="$OUT/calib-launch$L.jsonl"
      header calib > "$F.head"
      taskset -c "$AK_CPU_CLIENT" target/release/calib --launch "$L" --rounds "$ROUNDS" --iters "$ITERS" --out "$F.body"
      cat "$F.head" "$F.body" > "$F"; rm -f "$F.head" "$F.body"
      if command -v perf >/dev/null 2>&1; then
        for A in forward forward-reverse; do
          { echo "# perf stat, arm $A, $ITERS iterations (cycles and instructions per iteration = count / $ITERS)"
            perf stat -x, -e cycles,instructions taskset -c "$AK_CPU_CLIENT" target/release/calib --only "$A" --iters "$ITERS" 2>&1; } \
            >> "$OUT/calib-perf-launch$L.txt"
        done
      else
        echo "# perf is not installed on this machine: no cycles/instructions (requirement 20)" > "$OUT/calib-perf-launch$L.txt"
      fi
      echo "calib launch $L: $(grep -vc '^#' "$F") rows -> $F"
    done ;;

  *) echo "unknown suite $SUITE" >&2; exit 2 ;;
esac
