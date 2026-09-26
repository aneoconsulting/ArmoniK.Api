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
#   AK_LLC_BYTES    codec: the last-level cache in bytes (default 14417920, the i9-7900X's 13.75 MiB)
#   AK_POOL_BYTES   codec: requirement 11's pool input (default 2 x AK_LLC_BYTES; smoke 1 MiB)
#   AK_SERVER_THREADS  rpc: the server's tokio workers (default 4, the SERVER set size)
#   AK_ALLOW_DIRTY=1  run on a dirty tree (recorded in every header; requirement 27 refuses
#                   a dirty tree, so the campaign never sets it)
#
# Order of work: the correctness gate (gen/gate.sh: byte identity, the full corpus in both
# unknown-field modes with its controls AND the no-unknown build (step 12), crossing counts) must have PASSED for this exact
# tree before any timing suite runs (requirement 26); the crossing-count gate (19) runs again
# before codec and calib and stops the run on any difference from gen/crossings.txt or
# gen/crossings-nounk.txt.
# Every log starts with the header of requirement 27; samples are JSON lines (28), one file
# per suite, transport and launch (29).
#
# Requirements 10 and 12 (amended, WP5 step 10): the unknown-field modes retain and drop run
# in the FULL build (target/), the third mode no-unknown in a SEPARATE build with
# ak-core/ak-abi's `unknown-fields` feature off (target-nounk/, its own target directory: a
# shared one would overwrite libak_core.so). Each codec launch runs both binaries, the
# order alternating by launch (odd: full first); each rpc launch starts ONE server (per
# transport) that both clients call, also alternating. Files: codec-launchN.jsonl (retain,
# drop) and codec-nounk-launchN.jsonl (no-unknown); rpc-T-launchN.jsonl (A, B, C/D/E/F in
# retain and drop) and rpc-T-nounk-launchN.jsonl (A, B, C/D/E/F-nounk). Ratios are formed
# from per-launch medians (requirement 30 as amended); each binary carries A and B.
#
# CPU sets (requirement 4): ffi/campaign.sh exports AK_CPU_CLIENT / AK_CPU_SERVER from
# ffi/campaign.machine; run alone, this runner reads that file when they are unset.
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
if { [ -z "${AK_CPU_CLIENT:-}" ] || [ -z "${AK_CPU_SERVER:-}" ]; } && [ -f "$HERE/../../campaign.machine" ]; then
  # shellcheck source=/dev/null
  . "$HERE/../../campaign.machine"
fi
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
header() {  # header SUITE [VARIANT]
  local variant=${2:-full}
  echo "# rust slice campaign runner, suite $1, core variant $variant"
  echo "# INSTRUMENTATION unless on the campaign machine of CAMPAIGN.md section 2 (a container figure is not a result)"
  echo "# commit     $REV$DIRTY   tree-id $TREE"
  echo "# date       $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# machine    $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); $(nproc) CPUs online; kernel $(uname -r)"
  echo "# smt        active=$(sysf /sys/devices/system/cpu/smt/active) control=$(sysf /sys/devices/system/cpu/smt/control)"
  echo "# governor   $(sysf /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor)"
  echo "# turbo      intel_pstate/no_turbo=$(sysf /sys/devices/system/cpu/intel_pstate/no_turbo) cpufreq/boost=$(sysf /sys/devices/system/cpu/cpufreq/boost)"
  echo "# isolation  cmdline: $(tr ' ' '\n' < /proc/cmdline | grep -E '^(isolcpus|nohz_full|rcu_nocbs)=' | tr '\n' ' ' || true)cgroup: $(sysf /sys/fs/cgroup/cpuset.cpus.effective)"
  echo "# cpu sets   CLIENT=${AK_CPU_CLIENT:-unset} SERVER=${AK_CPU_SERVER:-unset} OS=the rest; set size ${AK_SET_SIZE:-unset} (ffi/campaign.machine ${AK_MACHINE_NAME:-not read})"
  echo "# threads    codec: 1 measuring thread; rpc client: tokio 2 workers per A/D/F cell, ak_runtime_new(2) per B/C/E client, k = 1/8/16 callers; rpc server: tokio ${AK_SERVER_THREADS:-4} workers (AK_SERVER_THREADS); calib: 1 thread"
  echo "# runtime    $(rustc --version); $(cargo --version)"
  echo "# incumbent  prost $(awk '/^name = "prost"$/{getline; print $3}' Cargo.lock | tr -d '"'), tonic $(awk '/^name = "tonic"$/{getline; print $3}' Cargo.lock | tr -d '"'), tonic-prost $(awk '/^name = "tonic-prost"$/{getline; print $3}' Cargo.lock | tr -d '"'); criterion $(awk '/^name = "criterion"$/{getline; print $3}' Cargo.lock | tr -d '"')"
  echo "# build      cargo --release (opt-level 3, lto off, codegen-units default), core ak-core as a cdylib linked through the dynamic linker, core features $( [ "$variant" = nounk ] && echo "rpc,init-guard WITHOUT unknown-fields (the no-unknown variant, target-nounk/)" || echo "rpc,init-guard,unknown-fields (the full variant, target/)"); harness guard on; transcoder ak_tc_utf8_trusted (a Rust String is UTF-8)"
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
  got="$OUT/crossings-nounk-current.txt"
  CARGO_TARGET_DIR="$HERE/target-count-nounk" cargo run --release -q -p campaign --no-default-features \
    --features count,init-guard --bin crossings 2>/dev/null > "$got"
  if diff -u gen/crossings-nounk.txt "$got" > "$OUT/crossings-nounk.diff"; then
    echo "crossing counts (no-unknown build): $(wc -l < "$got") rows identical to gen/crossings-nounk.txt"
    rm -f "$OUT/crossings-nounk.diff" "$got"
  else
    echo "STOP: no-unknown crossing counts differ from gen/crossings-nounk.txt (requirement 19); see $OUT/crossings-nounk.diff" >&2
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

bench_exe() {  # bench_exe CARGO-ARGS...
  cargo bench -q -p campaign "$@" --bench codec_suite --no-run --message-format=json 2>/dev/null \
    | python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1
}
NOUNK_FEATURES=(--no-default-features --features init-guard)
build() {
  cargo build --release -q -p campaign --bins 2>/dev/null
  BENCH=$(bench_exe)
  CARGO_TARGET_DIR="$HERE/target-nounk" cargo build --release -q -p campaign "${NOUNK_FEATURES[@]}" --bins 2>/dev/null
  BENCH_NOUNK=$(CARGO_TARGET_DIR="$HERE/target-nounk" bench_exe "${NOUNK_FEATURES[@]}")
  [ -x "$BENCH" ] && [ -x "$BENCH_NOUNK" ] || { echo "no codec bench executable" >&2; exit 1; }
  # The variant of each binary, checked on the core it loads (not assumed from the path).
  for b in "$BENCH:0" "$BENCH_NOUNK:1" "target/release/rpc_client:0" "target-nounk/release/rpc_client:1"; do
    local exe=${b%:*} want=${b##*:} so n
    so=$(ldd "$exe" | grep -o '/[^ ]*libak_core.so')
    n=$(nm -D --defined-only "$so" | grep -c ' T ak_uencode_' || true)
    if { [ "$want" = 1 ] && [ "$n" != 0 ]; } || { [ "$want" = 0 ] && [ "$n" = 0 ]; }; then
      echo "variant mix-up: $exe loads $so ($n ak_uencode_* exports)" >&2; exit 1
    fi
  done
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
      # requirement 11's pool input, reduced for the smoke (the campaign default is twice
      # the last-level cache, AK_LLC_BYTES, 13.75 MiB unless set)
      export AK_POOL_BYTES=${AK_POOL_BYTES:-1048576}
    fi
    codec_run() {  # codec_run L VARIANT EXE
      local L=$1 v=$2 exe=$3 tag="codec"; [ "$v" = nounk ] && tag="codec-nounk"
      local F="$OUT/$tag-launch$L.jsonl" C="$OUT/$tag-launch$L.criterion.log"
      header codec "$v" > "$F.head"
      CRITERION_HOME="$SCRATCH/criterion-$v-launch$L" AK_LAUNCH=$L AK_OUT="$F.body" \
        taskset -c "$AK_CPU_CLIENT" "$exe" > "$C" 2>&1 \
        || { echo "codec launch $L ($v) FAILED: $C" >&2; exit 1; }
      { cat "$F.head"; echo "# criterion's console output (its own summary; the samples are in $(basename "$F"))"; cat "$C"; } > "$C.tmp"
      mv "$C.tmp" "$C"
      cat "$F.head" "$F.body" > "$F"; rm -f "$F.head" "$F.body"
      echo "codec launch $L ($v): $(grep -vc '^#' "$F") sample rows -> $F"
    }
    for L in $(seq 1 "$LAUNCHES"); do
      if [ $((L % 2)) = 1 ]; then codec_run "$L" full "$BENCH"; codec_run "$L" nounk "$BENCH_NOUNK"
      else codec_run "$L" nounk "$BENCH_NOUNK"; codec_run "$L" full "$BENCH"; fi
    done ;;

  rpc)
    cpus_required AK_CPU_CLIENT AK_CPU_SERVER
    need_gate
    build
    CALLS=${AK_RPC_CALLS:-96}; WARM=${AK_RPC_WARMUP:-64}; SWARM=${AK_RPC_SERVER_WARMUP:-64}
    if [ "${AK_SMOKE:-0}" = 1 ]; then CALLS=16; WARM=16; SWARM=16; fi
    export AK_SERVER_THREADS=${AK_SERVER_THREADS:-4}
    # Requirement 13 as amended (R-H33): ONE server process and configuration per launch,
    # serving every cell of BOTH builds' clients, on a Unix domain socket (requirement 17,
    # R-H28). Each client warms it by $SWARM checked calls from each of its transports before
    # round 1, and opens one channel per cell.
    start_server() {  # start_server T L -> SP, SOCK
      SOCK="$SCRATCH/grid-$1-$2.sock"; local RF="$SCRATCH/ready-$1-$2"; rm -f "$RF" "$SOCK"
      taskset -c "$AK_CPU_SERVER" target/release/rpc_server --transport "$1" --socket "$SOCK" --ready-file "$RF" \
        2> "$OUT/rpc-$1-launch$2.server.log" &
      SP=$!
      for _ in $(seq 100); do [ -s "$RF" ] && break; sleep 0.1; done
      [ -s "$RF" ] || { echo "rpc_server ($1, launch $2) did not start" >&2; kill $SP; exit 1; }
    }
    CLARGS=(--server-warm "$SWARM")
    for T in shipped pinned; do
      # Requirement 18's control, per client binary, against its own server: a wrong
      # expected length must abort with no figure.
      start_server "$T" plant
      for v in full nounk; do
        CL=target/release/rpc_client; [ "$v" = nounk ] && CL=target-nounk/release/rpc_client
        rm -f "$SCRATCH/plant.jsonl"
        if taskset -c "$AK_CPU_CLIENT" "$CL" --socket "$SOCK" --transport "$T" "${CLARGS[@]}" \
             --rounds 1 --calls 16 --warmup 16 --out "$SCRATCH/plant.jsonl" --plant > "$OUT/rpc-$T-$v-PLANT.log" 2>&1 \
           || [ -e "$SCRATCH/plant.jsonl" ]; then
          echo "CONTROL FAILED: the planted wrong length did not abort ($T, $v client)" >&2; kill $SP; exit 1
        fi
        echo "rpc $T ($v client): control (planted wrong length) aborted with no output: $(tail -1 "$OUT/rpc-$T-$v-PLANT.log")"
      done
      kill $SP; wait $SP 2>/dev/null || true
      rpc_run() {  # rpc_run L VARIANT
        local L=$1 v=$2 CL=target/release/rpc_client F="$OUT/rpc-$T-launch$1.jsonl"
        [ "$v" = nounk ] && { CL=target-nounk/release/rpc_client; F="$OUT/rpc-$T-nounk-launch$L.jsonl"; }
        header rpc "$v" > "$F.head"
        taskset -c "$AK_CPU_CLIENT" "$CL" --socket "$SOCK" --transport "$T" --launch "$L" "${CLARGS[@]}" \
          --rounds "$ROUNDS" --calls "$CALLS" --warmup "$WARM" --out "$F.body" \
          || { echo "rpc $T launch $L ($v) ABORTED (requirement 18): no figure" >&2; kill $SP; rm -f "$F.head" "$F.body"; exit 1; }
        cat "$F.head" "$F.body" > "$F"; rm -f "$F.head" "$F.body"
        echo "rpc $T launch $L ($v): $(grep -vc '^#' "$F") sample rows -> $F"
      }
      for L in $(seq 1 "$LAUNCHES"); do
        start_server "$T" "$L"
        if [ $((L % 2)) = 1 ]; then rpc_run "$L" full; rpc_run "$L" nounk
        else rpc_run "$L" nounk; rpc_run "$L" full; fi
        kill $SP; wait $SP 2>/dev/null || true
      done
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
