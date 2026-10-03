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
#   AK_WORKERS      every pool's workers (D14; default 8): AK_SERVER_THREADS, AK_CORE_WORKERS and
#                   AK_HOST_WORKERS default to it
#   AK_RPC_TRANSPORTS / AK_RPC_BUILDS  rpc: narrow a (smoke) run, default "shipped pinned" /
#                   "full nounk"
#   Warm-ups and measurement (requirement 24; campaign default / smoke default; criterion's
#   own warm-up everywhere, FIX-PLAN WP9): codec AK_WARMUP_MS 500 / 5, AK_MEASURE_MS 2000 / 10;
#   rpc AK_RPC_WARMUP_MS 1500 / 5 (directions a, a+read, b) and AK_RPC_WARMUP_LONG_MS 5000 / 5
#   (directions c, d) (req 24 as amended 8c02e7c58: >= 20 calls per calling thread), AK_RPC_MEASURE_MS 2000 / 20, AK_RPC_SAMPLES 10 (criterion's
#   floor), AK_RPC_SERVER_WARMUP 64 / 16 checked calls from each client transport
#   AK_CAMPAIGN_ALLOC  default|pinned (CAMPAIGN req 25 as amended, D9; default `default`).
#                   default: GLIBC_TUNABLES is unset for every process, the main figures.
#                   pinned: GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:
#                   glibc.malloc.mmap_threshold=33554432 on the MEASURED CLIENT processes only
#                   (codec, rpc criterion clients, calib), never on the shared server; output
#                   files are labelled alloc-pinned. Each measured process checks the mode at
#                   start (16 MiB malloc, mallinfo2) and refuses to run on a mismatch.
#   AK_CAMPAIGN_GRID  core|full (CAMPAIGN section 4.0, D18; default core). core runs exactly section
#                   4.0's grid: codec 16 shapes + P2.2 Latin-1/wide + 7 U-* rows, arms incumbent-prod,
#                   core-ffi, host-gen (= core-native in Rust), encode end state (ii) hot + decode-read,
#                   retain (full build) / no-unknown build; RPC cells A, Bf-cb, Cf-cb-retain, Ef-cb-retain,
#                   a+read, b, c P5.4, d 16 MiB, k 1 and 8, full build, ONE transport `armonik` (TCP
#                   127.0.0.1, cell A through packages/rust/armonik-transport, Nagle off read back),
#                   plus Cf-cb on h2-batch (c, d, k 1 and 8); the pinned allocator pass: rpc A and Cf-cb
#                   on c and d at k = 1 only (codec and calib skipped). full runs every row (extras
#                   labelled row = extra), transports shipped and pinned on the Unix socket.
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

# ---- requirement 25 as amended (D9): the allocator mode ------------------------------
ALLOC=${AK_CAMPAIGN_ALLOC:-default}
PINNED_TUNABLES="glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432"
case "$ALLOC" in
  default) ALLOCENV=(env -u GLIBC_TUNABLES); ATAG="" ;;
  pinned) ALLOCENV=(env GLIBC_TUNABLES="$PINNED_TUNABLES"); ATAG="-alloc-pinned" ;;
  *) echo "refused: AK_CAMPAIGN_ALLOC=$ALLOC (default|pinned)" >&2; exit 2 ;;
esac
# nothing but the measured clients ever sees the tunables (the server, builds, warm-ups do not)
unset GLIBC_TUNABLES
export AK_CAMPAIGN_ALLOC=$ALLOC

# ---- CAMPAIGN section 4.0 (D18): the grid ---------------------------------------------
GRID=${AK_CAMPAIGN_GRID:-core}
case "$GRID" in core|full) ;; *) echo "refused: AK_CAMPAIGN_GRID=$GRID (core|full)" >&2; exit 2 ;; esac
export AK_CAMPAIGN_GRID=$GRID

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
  echo "# threads    D14: AK_WORKERS=${AK_WORKERS:-8} sizes every pool unless overridden; codec: 1 measuring thread; rpc client: tokio ${AK_HOST_WORKERS:-${AK_WORKERS:-8}} workers per A/D/F and -cb cell runtime (AK_HOST_WORKERS; ct = current-thread), ak_runtime_new(${AK_CORE_WORKERS:-${AK_WORKERS:-8}}) per B/C/E client (AK_CORE_WORKERS), k = 1/8/16 callers; rpc server: tokio ${AK_SERVER_THREADS:-${AK_WORKERS:-8}} workers (AK_SERVER_THREADS); calib: 1 thread"
  echo "# alloc      AK_CAMPAIGN_ALLOC=$ALLOC ran (modes: default = GLIBC_TUNABLES unset, the main figures; pinned = GLIBC_TUNABLES=$PINNED_TUNABLES on the measured client only, never the server, files labelled alloc-pinned); each measured process re-checks it (16 MiB malloc, mallinfo2) and its own header states the readback; rows carry alloc and minflt"
  echo "# rpc warm-up requirement 24 as amended (8c02e7c58), >= 20 calls per calling thread at the cell's payload, same process and threads: criterion's warm-up, set per benchmark (BenchmarkGroup::warm_up_time), AK_RPC_WARMUP_MS for directions a, a+read, b (campaign default 1500 ms) and AK_RPC_WARMUP_LONG_MS for c, d (campaign default 5000 ms), calls the SAME iter_custom routine on the SAME callers as the measurement: blocking cells (B, C, E and framed twins) k host threads created before the warm-up, each making exactly one call per criterion iteration; async cells (A, D, F, -cb) k tokio tasks per iteration on the cell's runtime, created before the warm-up (AK_HOST_WORKERS workers, the same threads; which worker runs which task is tokio's). Criterion doubles 1, 2, 4, 8, 16 iterations until its WALL time exceeds the warm-up, so >= 31 iterations (>= 31 calls per blocking thread) whenever 15 iterations take less than the warm-up. Long (c, d): the slowest, d/16MiB at k = 8, took 125-227 ms median wall per iteration in the container (worst 461 ms), 15 x 300 ms = 4.5 s -> 5000 ms. Short (a, a+read, b): at k = 16, 17.6-60.3 ms median wall per iteration (worst 96 ms; k = 8: 10.3-24.3, worst 63), 15 x 96 ms = 1.44 s -> 1500 ms (500 ms failed: 30 of 87 k = 16 benchmarks over 33 ms). Container figures, logs/rust/req24-warmup/. A time rule, not a count: the per-sample minflt (req 25) is the check on the machine; not met under AK_SMOKE"
  echo "# grid       AK_CAMPAIGN_GRID=$GRID ran (CAMPAIGN section 4.0, D18; core | full). core: codec = the 16 shapes (P7.1 decode only), Latin-1 and wide on P2.2 only, the 7 U-* rows, arms incumbent-prod (full build, once), core-ffi (push), host-gen = core-native, encode end state (ii) on the hot input + decode-read, retain in the full build and the no-unknown build for core-ffi/core-native; rpc = A, Bf-cb, Cf-cb-retain, Ef-cb-retain, a+read and b (P2.2), c P5.4, d 16 MiB, k 1 and 8, full build, one transport (armonik, TCP 127.0.0.1), plus Cf-cb-retain on h2-batch for c and d at k 1 and 8 (labelled h2 = h2-batch); allocator pass pinned: rpc A and Cf-cb-retain on c and d at k = 1 only. Extras left out under core (run under full, labelled row = extra): codec: core-ffi-pull, armonik, bare decode, the other encode variants, drop, incumbent and armonik in the no-unknown build, content sets on P1.2 and P2.4, the other U-* rows timed; rpc: B, C, D, E, F, Df, Ff, the reference rows, the blocking deliveries, drop, direction a, k = 16, P5.3, d 4 MiB, the no-unknown build, the shipped and pinned transports (Unix socket), h2-batch on other rows, the pinned allocator pass beyond its subset. Each log's own header lists its suite's extras"
  echo "# runtime    $(rustc --version); $(cargo --version)"
  echo "# incumbent  prost $(awk '/^name = "prost"$/{getline; print $3}' Cargo.lock | tr -d '"'), tonic $(awk '/^name = "tonic"$/{getline; print $3}' Cargo.lock | tr -d '"'), tonic-prost $(awk '/^name = "tonic-prost"$/{getline; print $3}' Cargo.lock | tr -d '"'); criterion $(awk '/^name = "criterion"$/{getline; print $3}' Cargo.lock | tr -d '"')"
  echo "# build      cargo --release (opt-level 3, lto off, codegen-units default), core ak-core as a cdylib linked through the dynamic linker, core features $( [ "$variant" = nounk ] && echo "rpc,init-guard WITHOUT unknown-fields (the no-unknown variant, target-nounk/)" || echo "rpc,init-guard,unknown-fields (the full variant, target/)"); harness guard on; transcoder ak_tc_utf8_trusted (a Rust String is UTF-8)"
  echo "# repeats    launches=$LAUNCHES rounds=$ROUNDS smoke=${AK_SMOKE:-0}"
  echo "# warm-ups   (requirement 24; campaign default / smoke default; the environment wins; criterion's own warm-up, no hand-written loop beside it) codec: AK_WARMUP_MS 500 / 5 ms; rpc: AK_RPC_WARMUP_MS 1500 / 5 ms per a, a+read, b benchmark and AK_RPC_WARMUP_LONG_MS 5000 / 5 ms per c, d benchmark, AK_RPC_SERVER_WARMUP 64 / 16 checked calls from each client transport before the first benchmark; calib: iters/10 per arm. The values used are in each log's own header"
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

bench_exe() {  # bench_exe NAME CARGO-ARGS...
  local name=$1; shift
  cargo bench -q -p campaign "$@" --bench "$name" --no-run --message-format=json 2>/dev/null \
    | AK_BENCH_NAME="$name" python3 -S -c 'import sys,json,os
n=os.environ["AK_BENCH_NAME"]
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")==n and m.get("executable"): print(m["executable"])' | tail -1
}
NOUNK_FEATURES=(--no-default-features --features init-guard)
build() {
  cargo build --release -q -p campaign --bins 2>/dev/null
  BENCH=$(bench_exe codec_suite)
  RPCB=$(bench_exe rpc_suite)
  CARGO_TARGET_DIR="$HERE/target-nounk" cargo build --release -q -p campaign "${NOUNK_FEATURES[@]}" --bins 2>/dev/null
  BENCH_NOUNK=$(CARGO_TARGET_DIR="$HERE/target-nounk" bench_exe codec_suite "${NOUNK_FEATURES[@]}")
  RPCB_NOUNK=$(CARGO_TARGET_DIR="$HERE/target-nounk" bench_exe rpc_suite "${NOUNK_FEATURES[@]}")
  [ -x "$BENCH" ] && [ -x "$BENCH_NOUNK" ] && [ -x "$RPCB" ] && [ -x "$RPCB_NOUNK" ] || { echo "no bench executable" >&2; exit 1; }
  # The variant of each binary, checked on the core it loads (not assumed from the path).
  for b in "$BENCH:0" "$BENCH_NOUNK:1" "$RPCB:0" "$RPCB_NOUNK:1"; do
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
    if [ "$GRID" = core ] && [ "$ALLOC" = pinned ]; then
      echo "codec: not in the core grid's pinned allocator pass (CAMPAIGN 4.0: rpc A and Cf on c and d at k = 1 only); nothing run"; exit 0
    fi
    cpus_required AK_CPU_CLIENT
    need_gate
    crossings
    build
    if [ "${AK_SMOKE:-0}" = 1 ]; then
      export AK_SAMPLES=10 AK_WARMUP_MS=${AK_WARMUP_MS:-5} AK_MEASURE_MS=${AK_MEASURE_MS:-10}
      # requirement 11's pool input, reduced for the smoke (the campaign default is twice
      # the last-level cache, AK_LLC_BYTES, 13.75 MiB unless set)
      export AK_POOL_BYTES=${AK_POOL_BYTES:-1048576}
    fi
    codec_run() {  # codec_run L VARIANT EXE
      local L=$1 v=$2 exe=$3 tag="codec$ATAG"; [ "$v" = nounk ] && tag="codec-nounk$ATAG"
      local F="$OUT/$tag-launch$L.jsonl" C="$OUT/$tag-launch$L.criterion.log"
      header codec "$v" > "$F.head"
      CRITERION_HOME="$SCRATCH/criterion-$v-launch$L" AK_LAUNCH=$L AK_OUT="$F.body" \
        "${ALLOCENV[@]}" taskset -c "$AK_CPU_CLIENT" "$exe" > "$C" 2>&1 \
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
    # FIX-PLAN WP9 (CAMPAIGN req 22a as amended 2026-09-27): the grid runs on criterion
    # (benches/rpc_suite.rs). Warm-up and measurement are criterion's; requirement 24's
    # knobs, campaign default / smoke default, the environment winning in both:
    if [ "${AK_SMOKE:-0}" = 1 ]; then
      RWARM=${AK_RPC_WARMUP_MS:-5}; RWARML=${AK_RPC_WARMUP_LONG_MS:-5}; RMEAS=${AK_RPC_MEASURE_MS:-20}; SWARM=${AK_RPC_SERVER_WARMUP:-16}
      # criterion's bootstrap for its console summary only (the samples are raw)
      export AK_NRESAMPLES=${AK_NRESAMPLES:-1000}
    else
      # Requirement 24 as amended (8c02e7c58): >= 20 calls per calling thread before the
      # first measured value. Criterion's warm-up runs 1, 2, 4, 8, 16 ... iterations (one
      # iteration = one call on each of the k calling threads) until its WALL time exceeds
      # the warm-up, so 20 calls need wall(15 iterations) < warm-up. Slowest cell measured
      # (container, 4 CPUs, 2026-10-03, d/16MiB k = 8): 125-227 ms median wall per iteration,
      # 461 ms the worst single one; 15 x ~300 ms = 4.5 s -> 5000 ms for directions c and d
      # (AK_RPC_WARMUP_LONG_MS). Directions a, a+read, b (AK_RPC_WARMUP_MS): k = 16 took up to
      # 60 ms median, 96 ms worst, 15 x 96 ms = 1.44 s -> 1500 ms (500 ms did not meet it).
      RWARM=${AK_RPC_WARMUP_MS:-1500}; RWARML=${AK_RPC_WARMUP_LONG_MS:-5000}; RMEAS=${AK_RPC_MEASURE_MS:-2000}; SWARM=${AK_RPC_SERVER_WARMUP:-64}
    fi
    RSAMP=${AK_RPC_SAMPLES:-10}
    # D14 (owner, 2026-10-03): every pool is AK_WORKERS workers (campaign.machine via
    # campaign.sh), default 8: the server, the core runtime and the tokio client runtimes.
    export AK_WORKERS=${AK_WORKERS:-8}
    export AK_SERVER_THREADS=${AK_SERVER_THREADS:-$AK_WORKERS} AK_CORE_WORKERS=${AK_CORE_WORKERS:-$AK_WORKERS} AK_HOST_WORKERS=${AK_HOST_WORKERS:-$AK_WORKERS}
    # Requirement 13 as amended (R-H33, and 9f6d579fa / FIX-PLAN WP10): ONE server process per
    # launch, THE server of every slice (poc/rust/serve.sh, interface SERVER.md), serving both
    # configurations on two Unix sockets (requirement 17) and every cell of BOTH builds. The
    # runner starts it, warms it ($SWARM checked calls per direction from a tonic and a core
    # client on each socket), then runs criterion; each benchmark process opens one channel
    # per cell.
    export AK_SERVE_STATE="$SCRATCH/serve.state"
    ./serve.sh build > /dev/null
    # CAMPAIGN 4.0 (D18) and its transport amendment (b58543f7b): the core grid runs ONE
    # configuration, armonik, over TCP 127.0.0.1 (the server's TCP listener); the full grid
    # keeps shipped and pinned on the Unix sockets.
    if [ "$GRID" = core ]; then
      TRANSPORTS=${AK_RPC_TRANSPORTS:-armonik}; BUILDS=${AK_RPC_BUILDS:-full}
      # h2-batch: Cf-cb on c and d (k 1, 8), main allocator pass only
      [ "$ALLOC" = default ] && H2B=${AK_RPC_H2BATCH:-1} || H2B=0
    else
      TRANSPORTS=${AK_RPC_TRANSPORTS:-shipped pinned}; BUILDS=${AK_RPC_BUILDS:-full nounk}; H2B=${AK_RPC_H2BATCH:-0}
    fi
    case " $TRANSPORTS " in *" armonik "*) export AK_SERVER_TCP=${AK_SERVER_TCP:-0} ;; esac
    if [ "$H2B" = 1 ]; then
      gen/h2batch_core.sh "$HERE/target-h2batch" > "$OUT/h2batch-core.build.log" 2>&1 \
        || { echo "h2-batch core build FAILED: $OUT/h2batch-core.build.log" >&2; exit 1; }
      H2B_LIB="$HERE/target-h2batch/release/deps"
      grep -q 'h2 compiled in: h2-batch-src/' "$OUT/h2batch-core.build.log" \
        || { echo "the h2-batch core does not carry the patched h2: $OUT/h2batch-core.build.log" >&2; exit 1; }
      # captured, not piped into grep -q (pipefail: grep's early exit SIGPIPEs ldd)
      L=$(LD_LIBRARY_PATH="$H2B_LIB" ldd "$RPCB"); case "$L" in *"$H2B_LIB/libak_core.so"*) ;;
        *) echo "rpc_suite does not load $H2B_LIB/libak_core.so under LD_LIBRARY_PATH" >&2; exit 1 ;; esac
    fi
    serve_start() {  # serve_start TAG -> SOCK_shipped, SOCK_pinned, TCP_ADDR
      local o; o=$(./serve.sh start --out "$SCRATCH/serve-$1")
      SOCK_shipped=$(echo "$o" | sed -n 's/^shipped //p'); SOCK_pinned=$(echo "$o" | sed -n 's/^pinned //p')
      TCP_ADDR=$(echo "$o" | sed -n 's/^tcp //p')
      cp "$SCRATCH/serve-$1/rpc-server.log" "$OUT/rpc-launch$1.server.log" 2>/dev/null || true
    }
    serve_stop() {  # serve_stop TAG
      cp "$SCRATCH/serve-$1/rpc-server.log" "$OUT/rpc-launch$1.server.log" 2>/dev/null || true
      ./serve.sh stop > /dev/null
    }
    rpc_bench() {  # rpc_bench EXE OUT CRITHOME [NAME=VALUE ...]: one criterion process
      local exe=$1 out=$2 home=$3 sock; shift 3
      sock=SOCK_$T; sock=${!sock:-}
      env AK_RPC_SOCKET="$sock" AK_RPC_TCP="${TCP_ADDR:-}" AK_RPC_TRANSPORT="$T" AK_OUT="$out" CRITERION_HOME="$home" \
          AK_SAMPLES="$RSAMP" AK_WARMUP_MS="$RWARM" AK_WARMUP_LONG_MS="$RWARML" AK_MEASURE_MS="$RMEAS" "$@" \
          "${ALLOCENV[@]}" taskset -c "$AK_CPU_CLIENT" "$exe"
    }
    # Requirement 18's controls, per client binary and transport, against their own server:
    # a wrong expected length must abort with no sample. Once per send path in the warm-up
    # (A: tonic codec, B: core reference, Bf: core framed, Df: tonic Channel framed) and
    # direction (a, c, d), and once INSIDE a criterion benchmark (a failed check panics).
    serve_start plant
    for T in $TRANSPORTS; do
      for v in $BUILDS; do
        EXE=$RPCB; [ "$v" = nounk ] && EXE=$RPCB_NOUNK
        for WP in A B Bf Df; do for WD in a c d; do
          PL="$OUT/rpc-$T-$v-$WP-$WD-PLANT.log"; rm -f "$SCRATCH/plant.jsonl"
          if rpc_bench "$EXE" "$SCRATCH/plant.jsonl" "$SCRATCH/crit-plant" AK_RPC_PLANT=warm AK_RPC_SERVER_WARMUP=1 \
               AK_RPC_WARM_CELLS="$WP" AK_RPC_WARM_DIR="$WD" > "$PL" 2>&1 || [ -e "$SCRATCH/plant.jsonl" ]; then
            echo "CONTROL FAILED: the planted wrong length did not abort ($T, $v client, cell $WP, dir $WD)" >&2; serve_stop plant; exit 1
          fi
          echo "rpc $T ($v client, cell $WP, dir $WD): control (planted wrong length) aborted with no output: $(grep -m1 ABORT "$PL")"
        done; done
        PL="$OUT/rpc-$T-$v-bench-PLANT.log"; rm -f "$SCRATCH/plant.jsonl"
        # the bench plant names a cell the grid times (core: Cf-cb; full: B), and must ABORT
        PC=B; [ "$GRID" = core ] && PC=Cf-cb
        if rpc_bench "$EXE" "$SCRATCH/plant.jsonl" "$SCRATCH/crit-plant" AK_RPC_PLANT=bench AK_RPC_CELLS=$PC AK_RPC_SERVER_WARMUP=0 \
             > "$PL" 2>&1 || [ -e "$SCRATCH/plant.jsonl" ] || ! grep -q ABORT "$PL"; then
          echo "CONTROL FAILED: the planted wrong length inside a criterion benchmark did not abort ($T, $v)" >&2; serve_stop plant; exit 1
        fi
        echo "rpc $T ($v client, inside criterion): control aborted with no output: $(grep -m1 ABORT "$PL")"
        if [ "$T" = armonik ]; then
          # CAMPAIGN 4.0 as amended: a live socket with Nagle on must refuse the run (exit 7, no output)
          PL="$OUT/rpc-$T-$v-nagle-PLANT.log"; rm -f "$SCRATCH/plant.jsonl"
          if rpc_bench "$EXE" "$SCRATCH/plant.jsonl" "$SCRATCH/crit-plant" AK_RPC_PLANT=nagle AK_RPC_CELLS=Cf-cb AK_RPC_SERVER_WARMUP=0 \
               > "$PL" 2>&1 || [ -e "$SCRATCH/plant.jsonl" ] || ! grep -q 'Nagle ON' "$PL"; then
            echo "CONTROL FAILED: a socket with Nagle on did not refuse the run ($T, $v)" >&2; serve_stop plant; exit 1
          fi
          echo "rpc $T ($v client): control (Nagle switched on) refused with no output: $(grep -m1 REFUSED "$PL")"
        fi
      done
    done
    serve_stop plant
    rpc_run() {  # rpc_run L VARIANT
      local L=$1 v=$2 EXE=$RPCB F="$OUT/rpc-$T$ATAG-launch$1.jsonl" X=()
      [ "$v" = nounk ] && { EXE=$RPCB_NOUNK; F="$OUT/rpc-$T-nounk$ATAG-launch$L.jsonl"; }
      # h2-batch (CAMPAIGN 4.0): the same full-build binary on the h2-batch core (LD_LIBRARY_PATH,
      # ahead of its RUNPATH; rpc_suite checks the core it mapped against AK_H2)
      [ "$v" = h2b ] && { F="$OUT/rpc-$T-h2batch$ATAG-launch$L.jsonl"; X=(LD_LIBRARY_PATH="$H2B_LIB" AK_H2=h2-batch); }
      local C="${F%.jsonl}.criterion.log"
      header rpc "$( [ "$v" = h2b ] && echo full || echo "$v")" > "$F.head"
      [ "$v" = h2b ] && echo "# h2         h2-batch core: $(sed -n '2p;3p' "$OUT/h2batch-core.build.log" | tr '\n' ' ')" >> "$F.head"
      # The runner warmed the server (serve.sh warm); any failure discards the launch's
      # output (requirement 18).
      rpc_bench "$EXE" "$F.body" "$SCRATCH/crit-rpc-$T-$v-$L" AK_LAUNCH="$L" AK_RPC_SERVER_WARMUP=0 "${X[@]}" > "$C" 2>&1 \
        || { echo "rpc $T launch $L ($v) ABORTED (requirement 18): no figure; $(grep -m1 ABORT "$C")" >&2; serve_stop "$L"; rm -f "$F.head" "$F.body" "$F"; exit 1; }
      { cat "$F.head"; echo "# criterion's console output (its own summary; the samples are in $(basename "$F"))"; cat "$C"; } > "$C.tmp"; mv "$C.tmp" "$C"
      cat "$F.head" "$F.body" > "$F"; rm -f "$F.head" "$F.body"
      echo "rpc $T launch $L ($v): $(grep -vc '^#' "$F") sample rows -> $F"
    }
    for L in $(seq 1 "$LAUNCHES"); do
      serve_start "$L"
      ./serve.sh warm "$SWARM" > "$OUT/rpc-launch$L.server-warm.log" 2>&1 \
        || { echo "rpc launch $L: the server warm-up failed (requirement 18); see $OUT/rpc-launch$L.server-warm.log" >&2; serve_stop "$L"; exit 1; }
      for T in $TRANSPORTS; do
        # the processes' order (builds, then h2-batch) alternates by launch
        VS="$BUILDS"; [ "$H2B" = 1 ] && VS="$VS h2b"
        if [ $((L % 2)) = 1 ]; then ORDER="$VS"; else ORDER=$(echo "$VS" | tr ' ' '\n' | tac | tr '\n' ' '); fi
        for v in $ORDER; do rpc_run "$L" "$v"; done
      done
      serve_stop "$L"
    done ;;

  calib)
    if [ "$GRID" = core ] && [ "$ALLOC" = pinned ]; then
      echo "calib: not in the core grid's pinned allocator pass (CAMPAIGN 4.0); nothing run"; exit 0
    fi
    cpus_required AK_CPU_CLIENT
    need_gate
    crossings
    build
    ITERS=${AK_CALIB_ITERS:-20000000}
    [ "${AK_SMOKE:-0}" = 1 ] && ITERS=1000000
    for L in $(seq 1 "$LAUNCHES"); do
      F="$OUT/calib$ATAG-launch$L.jsonl"
      header calib > "$F.head"
      "${ALLOCENV[@]}" taskset -c "$AK_CPU_CLIENT" target/release/calib --launch "$L" --rounds "$ROUNDS" --iters "$ITERS" --out "$F.body"
      cat "$F.head" "$F.body" > "$F"; rm -f "$F.head" "$F.body"
      if command -v perf >/dev/null 2>&1; then
        for A in forward forward-reverse; do
          { echo "# perf stat, arm $A, $ITERS iterations (cycles and instructions per iteration = count / $ITERS)"
            perf stat -x, -e cycles,instructions "${ALLOCENV[@]}" taskset -c "$AK_CPU_CLIENT" target/release/calib --only "$A" --iters "$ITERS" 2>&1; } \
            >> "$OUT/calib-perf$ATAG-launch$L.txt"
        done
      else
        echo "# perf is not installed on this machine: no cycles/instructions (requirement 20)" > "$OUT/calib-perf$ATAG-launch$L.txt"
      fi
      echo "calib launch $L: $(grep -vc '^#' "$F") rows -> $F"
    done ;;

  *) echo "unknown suite $SUITE" >&2; exit 2 ;;
esac
