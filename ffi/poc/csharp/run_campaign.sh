#!/usr/bin/env bash
# The csharp slice's campaign runner (design/CAMPAIGN.md section 9, requirement 31).
#
#   run_campaign.sh --suite codec|rpc|calib|gate --out DIR [--launches N] [--rounds N]
#                   [--smoke] [--plant]
#
#   AK_CPU_CLIENT   CPUs of the measured process (codec, calib, the RPC client)   required
#   AK_CPU_SERVER   CPUs of the RPC server process                                required for rpc
#   AK_ALLOW_DIRTY  1 = run on a dirty tree (the header says so); refused otherwise
#
# Warm-ups (CAMPAIGN req 24 as amended 85cfd4826: every warm-up is a runner parameter; campaign
# default / --smoke default):
#   rpc client  BenchmarkDotNet (WP9): AK_RPC_BDN_WARMUP 10 / 1   AK_RPC_BDN_ITERATION_MS 100 / 20
#               AK_RPC_BDN_ROUNDS = --rounds
#   rpc server  the Rust slice's rpc_server (poc/rust/serve.sh, WP10): AK_RPC_SERVER_WARM 2000 / 100
#               (serve.sh warm N), AK_SERVER_THREADS its tokio workers (default AK_WORKERS)
#   rpc subset  AK_RPC_TRANSPORTS "shipped pinned", AK_RPC_BUILDS "full nounk" (small runs only)
#   h2          AK_H2_VARIANTS "stock h2-batch" (D11 as amended): the rpc suite runs once per
#               h2 variant of the core, each with its own gate; every row carries `h2`
#   allocator   AK_CAMPAIGN_ALLOC default / pinned (req 25, D9 as amended 2026-10-03): default = the main
#               figures (GLIBC_TUNABLES unset); pinned = the labelled diagnostic pass
#   pools       AK_WORKERS (campaign.machine; default 8): the core runtime, the .NET thread pool
#               and the server's workers (D8, D14)
#   codec (BDN) AK_BDN_WARMUP 10 / 1   AK_BDN_ITERATION_MS 100 / 2   AK_BDN_ROUNDS = --rounds
#               (WP9 addendum: BDN's own warm-up only; no hand-written pre-warm; the per-case JIT
#               check fails a unit outside --smoke and is reported only in a smoke)
#   toolchain   AK_BDN_GROUPED 0 / 1: 0 = BDN's default toolchain, one child process per case
#               (the campaign, req 22a as amended e6c909630); 1 = InProcessEmit, every case of a
#               unit in one process (smoke and small exploration runs only); both suites
# Every header states the values used.
# Defaults are the campaign's: 3 launches, 5 rounds (requirement 23). --smoke is section 9's
# container smoke run: 1 launch, 1 round, reduced iterations, every figure marked
# instrumentation. --plant (rpc only) runs the requirement-18 control: a wrong expected
# length must abort the client with no sample.
#
# Every timed suite first requires this slice's correctness gate (gen/gate.sh) to have
# passed at this commit in DIR (requirement 26); it runs it if not. Each suite and launch is
# one file, DIR/<suite>-<transport->launch<N>.jsonl: a header ('#' lines) then one JSON
# object per sample (requirements 27-29). The campaign's DIR is ffi/logs/csharp/campaign/
# (requirement 29 as amended at 0e8e9eb).
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(git -C "$SLICE" rev-parse --show-toplevel)"
SUITE="" OUT="" LAUNCHES=3 ROUNDS=5 SMOKE=0 PLANT=0
while [ $# -gt 0 ]; do
  case "$1" in
    --suite) SUITE="$2"; shift ;;
    --out) OUT="$2"; shift ;;
    --launches) LAUNCHES="$2"; shift ;;
    --rounds) ROUNDS="$2"; shift ;;
    --smoke) SMOKE=1; LAUNCHES=1; ROUNDS=1 ;;
    --plant) PLANT=1 ;;
    *) echo "unknown argument $1" >&2; exit 2 ;;
  esac
  shift
done
[ -n "$SUITE" ] && [ -n "$OUT" ] || { echo "usage: run_campaign.sh --suite codec|rpc|calib|gate --out DIR" >&2; exit 2; }
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
export SCRATCH="${SCRATCH:-$(mktemp -d)}"
export DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
COMMIT="$(git -C "$REPO" rev-parse --short HEAD)"
DIRTY=0
# What the run reads from the working tree: this slice, the schema and the corpus. The core
# and the generator are built from `git archive HEAD` (gen/build_core.sh, gen/gate.sh), so a
# working-tree edit in ffi/poc/codec cannot enter a run and is not part of this check.
CODE="ffi/poc/csharp/src ffi/poc/csharp/gen ffi/poc/csharp/abi ffi/poc/csharp/run_campaign.sh ffi/poc/csharp/Directory.Build.props ffi/poc/csharp/Directory.Build.targets"
# (STATE.md and JOURNAL.md are notes, not inputs of a run.)
if ! git -C "$REPO" diff --quiet HEAD -- $CODE ffi/schema ffi/corpus \
   || [ -n "$(git -C "$REPO" status --porcelain --untracked-files=normal -- ffi/poc/csharp/src ffi/poc/csharp/gen ffi/poc/csharp/run_campaign.sh)" ]; then
  DIRTY=1
  if [ "${AK_ALLOW_DIRTY:-0}" != "1" ]; then echo "refused: the tree is dirty (requirement 27); commit, or AK_ALLOW_DIRTY=1" >&2; exit 3; fi
fi
# CAMPAIGN req 4 (R-H34): the CPU sets come from ffi/campaign.machine, which ffi/campaign.sh
# sources and exports; run on its own, this runner reads the same file (outside --smoke).
# Values already in the environment win, as in campaign.sh.
CPUSRC="environment (ffi/campaign.sh exports ffi/campaign.machine's)"
if [ -z "${AK_CPU_CLIENT:-}" ] || { [ "$SUITE" = rpc ] && [ -z "${AK_CPU_SERVER:-}" ]; }; then
  if [ $SMOKE = 1 ]; then AK_CPU_CLIENT="${AK_CPU_CLIENT:-0}"; AK_CPU_SERVER="${AK_CPU_SERVER:-1}"; CPUSRC="smoke defaults (a container has no campaign CPU sets)"
  elif [ -f "$REPO/ffi/campaign.machine" ]; then
    # shellcheck source=/dev/null
    . "$REPO/ffi/campaign.machine"; CPUSRC="ffi/campaign.machine (${AK_MACHINE_NAME:-unnamed})"
  else echo "AK_CPU_CLIENT (and AK_CPU_SERVER for rpc) must be set (requirement 4)" >&2; exit 2; fi
fi
ncpus() { local n=0 r; local IFS=,; for r in $1; do case "$r" in *-*) n=$((n + ${r#*-} - ${r%-*} + 1)) ;; *) n=$((n + 1)) ;; esac; done; echo $n; }
if [ $SMOKE != 1 ] && [ -n "${AK_SET_SIZE:-}" ]; then
  if [ "$(ncpus "$AK_CPU_CLIENT")" != "$AK_SET_SIZE" ] || { [ -n "${AK_CPU_SERVER:-}" ] && [ "$(ncpus "$AK_CPU_SERVER")" != "$AK_SET_SIZE" ]; }; then
    echo "CLIENT=$AK_CPU_CLIENT SERVER=${AK_CPU_SERVER:-} : the campaign fixes $AK_SET_SIZE CPUs per set (CAMPAIGN req 4)" >&2; exit 2
  fi
fi
# D8 / D14: every worker pool is sized to AK_WORKERS (campaign.machine: AK_SET_SIZE, 8).
export AK_WORKERS="${AK_WORKERS:-8}" AK_CPU_CLIENT
export AK_SERVER_THREADS="${AK_SERVER_THREADS:-$AK_WORKERS}"
H2_VARIANTS="${AK_H2_VARIANTS:-stock h2-batch}"
# CAMPAIGN req 25 / D9 as amended 2026-10-03: the main figures on glibc's default allocator;
# AK_CAMPAIGN_ALLOC=pinned runs the labelled diagnostic pass with the trim and mmap thresholds pinned.
PINNED_TUNABLES="glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432"
export AK_CAMPAIGN_ALLOC="${AK_CAMPAIGN_ALLOC:-default}"
case "$AK_CAMPAIGN_ALLOC" in
  default) unset GLIBC_TUNABLES; ASFX="" ;;
  pinned) export GLIBC_TUNABLES="$PINNED_TUNABLES"; ASFX=".alloc-pinned" ;;
  *) echo "AK_CAMPAIGN_ALLOC must be default or pinned" >&2; exit 2 ;;
esac
# CAMPAIGN req 11 (R-H29): the pool input is sized from the last-level cache (2 x AK_LLC_BYTES
# of retained graphs; default 13.75 MB, the reference i9-7900X). A smoke run uses a small pool.
export AK_LLC_BYTES="${AK_LLC_BYTES:-14417920}"
# CAMPAIGN req 22a as amended (e6c909630): grouping is a switch, on only for small runs.
GROUPED="${AK_BDN_GROUPED:-$SMOKE}"
if [ "$GROUPED" = 1 ]; then TOOLCHAIN=grouped; else TOOLCHAIN=process; fi
[ $SMOKE = 1 ] && export AK_POOL_BYTES="${AK_POOL_BYTES:-65536}"
R8="$SLICE/src/Rpc/bin/Release/net8.0"
# WP5 step 10: the NO-UNKNOWN build (/p:AkNounk=true, unknown fields compiled out of the
# binding and of its core, target-core*-nounk), side by side with the full one.
RN8="$SLICE/src/Rpc/bin-nounk/Release/net8.0"
HN8="$SLICE/src/Harness/bin-nounk/Release/net8.0"
B8="$SLICE/src/BenchDotNet/bin/Release/net8.0"
BN8="$SLICE/src/BenchDotNet/bin-nounk/Release/net8.0"
# The builds of a launch, in its order: full first on odd launches, no-unknown first on even.
builds_of() { if [ $(( $1 % 2 )) = 1 ]; then echo "full nounk"; else echo "nounk full"; fi; }
H8="$SLICE/src/Harness/bin/Release/net8.0"

sysf() { [ -r "$1" ] && cat "$1" 2>/dev/null || echo "n/a"; }
header() {  # requirement 27: the machine and the build, in every log
  echo "# csharp campaign runner, suite $SUITE"
  echo "# commit:        $COMMIT$([ $DIRTY = 1 ] && echo ' DIRTY (AK_ALLOW_DIRTY=1): not a campaign log')"
  [ $SMOKE = 1 ] && echo "# SMOKE RUN (CAMPAIGN.md section 9) in a container: EVERY FIGURE BELOW IS INSTRUMENTATION, NOT A RESULT (README 1.1)"
  echo "# utc:           $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# cpu:           $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //'); $(nproc --all) logical CPUs; kernel $(uname -r)"
  echo "# smt:           $(sysf /sys/devices/system/cpu/smt/active) (active); governor: $(sysf /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor); turbo: no_turbo=$(sysf /sys/devices/system/cpu/intel_pstate/no_turbo) boost=$(sysf /sys/devices/system/cpu/cpufreq/boost)"
  echo "# isolation:     cmdline isolcpus/nohz_full: $(tr ' ' '\n' < /proc/cmdline | grep -E '^(isolcpus|nohz_full)=' | tr '\n' ' ' || true)cpuset: $(sysf /sys/fs/cgroup/cpuset.cpus.effective)"
  echo "# cpu sets:      CLIENT=$AK_CPU_CLIENT SERVER=${AK_CPU_SERVER:-n/a} ($(ncpus "$AK_CPU_CLIENT") and $([ -n "${AK_CPU_SERVER:-}" ] && ncpus "$AK_CPU_SERVER" || echo 0) CPUs; set size fixed at ${AK_SET_SIZE:-unset}); from $CPUSRC; pinning by taskset"
  echo "# toolchain:     BenchmarkDotNet $([ "$GROUPED" = 1 ] && echo "GROUPED (InProcessEmit, one process per unit: the small-run switch AK_BDN_GROUPED=1, req 22a as amended e6c909630)" || echo "default toolchain, one child process per case (the campaign's isolation, req 22a as amended e6c909630)")"
  echo "# pools:         AK_WORKERS=$AK_WORKERS (D8, D14): the core runtime's workers, the .NET thread pool's worker minimum and maximum (rpc), the server's tokio workers AK_SERVER_THREADS=$AK_SERVER_THREADS"
  echo "# allocator:     $AK_CAMPAIGN_ALLOC (AK_CAMPAIGN_ALLOC; GLIBC_TUNABLES=${GLIBC_TUNABLES:-unset} for every client process). CAMPAIGN req 25 / D9 as amended 2026-10-03: the MAIN figures run glibc's default allocator, as production does (AK_CAMPAIGN_ALLOC=default, GLIBC_TUNABLES unset); AK_CAMPAIGN_ALLOC=pinned is the labelled diagnostic pass (GLIBC_TUNABLES=$PINNED_TUNABLES), its files suffixed .alloc-pinned. The core's buffers and transport allocate through glibc malloc in this slice's process (no shim of its own); managed objects are on the .NET GC heap. Every row carries alloc and minflt (minor page faults of the process over the iteration; per call = minflt / iters), each process refuses to run if its GLIBC_TUNABLES does not match AK_CAMPAIGN_ALLOC. The RPC server runs the default allocator in both passes (started with GLIBC_TUNABLES unset)"
  echo "# llc:           AK_LLC_BYTES=$AK_LLC_BYTES; pool input >= ${AK_POOL_BYTES:-$((2 * AK_LLC_BYTES))} bytes of retained graphs (req 11)"
  echo "# runtime:       .NET $(dotnet --list-runtimes | awk '/NETCore.App/{print $2}' | tr '\n' ' ')(SDK $(dotnet --version)); target net8.0, Release; tiering and PGO at their net8.0 defaults unless DOTNET_* is set: TieredCompilation=${DOTNET_TieredCompilation:-default} TieredPGO=${DOTNET_TieredPGO:-default}; workstation GC, concurrent (default)"
  echo "# core:          libak_core.so shared, cargo --release, features $1 (init-guard ON, as in every gate), built from git archive HEAD ffi/poc/codec"
  if [ "$SUITE" = codec ]; then echo "# repeats:       $LAUNCHES launch(es) x $ROUNDS BDN actual iteration(s) per case, each with its own process CPU and wall (req 21); one process per arm:mode unit, unit order a seeded shuffle per launch (req 22); ratios from per-launch medians (req 30)"; else echo "# repeats:       $LAUNCHES launch(es) x $ROUNDS round(s); cells in a seeded shuffle per round (req 22); ratios from per-launch medians (req 30)"; fi
}

ensure_core() {
  if [ ! -f "$SLICE/target-core/release/libak_core.so" ] || [ ! -f "$SLICE/target-core-count/release/libak_core.so" ] \
     || [ ! -f "$SLICE/target-core-nounk/release/libak_core.so" ] || [ ! -f "$SLICE/target-core-count-nounk/release/libak_core.so" ] \
     || [ ! -f "$SLICE/target-core-h2b/release/libak_core.so" ] || [ ! -f "$SLICE/target-core-nounk-h2b/release/libak_core.so" ]; then
    "$SLICE/gen/build_core.sh" > "$OUT/build-core.log" 2>&1 || { cat "$OUT/build-core.log"; exit 1; }
  fi
}
build() {
  ( cd "$SLICE" && dotnet build src/Rpc/akrpc.csproj -c Release > "$SCRATCH/campaign-build.out" 2>&1 ) || { tail -30 "$SCRATCH/campaign-build.out"; exit 1; }
  ( cd "$SLICE" && dotnet build src/Harness/Harness.csproj -c Release -f net8.0 >> "$SCRATCH/campaign-build.out" 2>&1 ) || { tail -30 "$SCRATCH/campaign-build.out"; exit 1; }
  ( cd "$SLICE" && dotnet build src/Rpc/akrpc.csproj -c Release -p:AkNounk=true >> "$SCRATCH/campaign-build.out" 2>&1 ) || { tail -30 "$SCRATCH/campaign-build.out"; exit 1; }
  ( cd "$SLICE" && dotnet build src/Harness/Harness.csproj -c Release -f net8.0 -p:AkNounk=true >> "$SCRATCH/campaign-build.out" 2>&1 ) || { tail -30 "$SCRATCH/campaign-build.out"; exit 1; }
}
gate_first() {  # requirement 26; per h2 variant (AK_H2, gen/gate.sh), gate.log / gate.h2-batch.log
  local gl="$OUT/gate.log"; [ "${AK_H2:-stock}" != stock ] && gl="$OUT/gate.$AK_H2.log"
  # A passed gate is reused when the commit it ran at has the SAME content as HEAD in every
  # path a run depends on (other slices commit to the branch concurrently).
  if [ -f "$gl" ] && grep -q "^GATE PASSED" "$gl" && ! grep -q "^# branch HEAD: .*uncommitted" "$gl"; then
    local g; g="$(sed -n 's/^# branch HEAD: \([0-9a-f]*\).*/\1/p' "$gl" | head -1)"
    if [ -n "$g" ] && git -C "$REPO" diff --quiet "$g" HEAD -- $CODE ffi/poc/codec ffi/schema ffi/corpus; then
      echo "# gate:          h2 ${AK_H2:-stock}: passed at $g, identical to $COMMIT in ffi/poc/csharp, poc/codec, schema, corpus ($gl)"
      return
    fi
  fi
  # Every gate run writes its OWN log, named by commit, time, suite and plant, and is never
  # overwritten (a failed gate at 253f487 was lost when the next run, the --plant control's,
  # re-ran the gate into the same file). gate.log is only ever a copy of a PASSED run.
  local lg; lg="$OUT/gate-$COMMIT-$(date -u +%Y%m%dT%H%M%SZ)-$SUITE-${AK_H2:-stock}$([ $PLANT = 1 ] && echo -PLANT).log"
  "$SLICE/gen/gate.sh" > "$lg" 2>&1
  if ! grep -q "^GATE PASSED" "$lg"; then echo "the correctness gate FAILED ($lg): no figure is produced" >&2; exit 1; fi
  cp "$lg" "$gl"
  echo "# gate:          h2 ${AK_H2:-stock}: passed at $COMMIT ($lg, copied to $gl)"
}

case "$SUITE" in
  gate)
    "$SLICE/gen/gate.sh" > "$OUT/gate.log" 2>&1; rc=$?
    tail -1 "$OUT/gate.log"; exit $rc ;;
  codec)
    # The codec suite runs under BenchmarkDotNet (CAMPAIGN.md 22a): src/BenchDotNet, InProcessEmit
    # toolchain. A launch is one pinned process per unit "arm:mode", in the launch's order
    # (BenchDotNet --list-units: arms rotated by launch, modes within an arm too; req 22): with
    # every case in one process BDN's live heap made each of its forced GCs slower and the
    # per-case overhead tripled (JOURNAL 51). Every unit appends its own header block and rows
    # to DIR/codec-launch<N>.jsonl; BDN's console log per unit is DIR/codec-launch<N>.<unit>.bdn.log.
    # BDN's artifacts directory holds only a copy of that log, so it stays in SCRATCH.
    GATE="$(gate_first)" || exit 1; ensure_core; build
    ( cd "$SLICE" && dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release >> "$SCRATCH/campaign-build.out" 2>&1 ) || { tail -30 "$SCRATCH/campaign-build.out"; exit 1; }
    ( cd "$SLICE" && dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkNounk=true >> "$SCRATCH/campaign-build.out" 2>&1 ) || { tail -30 "$SCRATCH/campaign-build.out"; exit 1; }
    cp "$SLICE/target-core/release/libak_core.so" "$B8/"
    cp "$SLICE/target-core-nounk/release/libak_core.so" "$BN8/"
    EXTRA=(--rounds "${AK_BDN_ROUNDS:-$ROUNDS}" --toolchain "$TOOLCHAIN"); [ $SMOKE = 1 ] && EXTRA+=(--smoke)
    [ -n "${AK_BDN_WARMUP:-}" ] && EXTRA+=(--warmup "$AK_BDN_WARMUP")
    [ -n "${AK_BDN_ITERATION_MS:-}" ] && EXTRA+=(--iteration-ms "$AK_BDN_ITERATION_MS")
    # A smoke run keeps 6 of the U-* rows (spread evenly), every direction and arm of each.
    [ $SMOKE = 1 ] && export AK_BDN_UROWS="${AK_BDN_UROWS:-6}"
    for l in $(seq 1 "$LAUNCHES"); do
      f="$OUT/codec-launch$l$ASFX.jsonl"
      { header "rpc,init-guard (full) and rpc,init-guard without unknown-fields (no-unknown)"; echo "$GATE"; echo "# builds, in this launch's order: $(builds_of "$l") (WP5 step 10; each process checks its core is its variant)"; echo "# h2:            stock (target-core*; the codec suite makes no transport call, so the h2 variant is not a dimension of it; rows carry h2=stock)"; } > "$f"
      for bld in $(builds_of "$l"); do
        if [ "$bld" = full ]; then BX="$B8"; else BX="$BN8"; fi
        for u in $(dotnet "$BX/BenchDotNet.dll" --launch "$l" --list-units); do
          ul="$OUT/codec-launch$l$ASFX.${u/:/-}.bdn.log"
          # R-H19: BDN's console log carries figures; a smoke's is headed as instrumentation.
          { [ $SMOKE = 1 ] && echo "# SMOKE RUN in a container: EVERY FIGURE IN THIS LOG IS INSTRUMENTATION, NOT A RESULT (README 1.1)"; } > "$ul"
          # R-H18: a unit whose JIT check fails exits non-zero, so the launch stops here.
          taskset -c "$AK_CPU_CLIENT" dotnet "$BX/BenchDotNet.dll" --launch "$l" --unit "$u" --out "$f" --artifacts "$SCRATCH/bdn-launch$l" "${EXTRA[@]}" \
            >> "$ul" 2>&1 || { echo "codec launch $l unit $u ($bld) failed, or its JIT check failed ($f, $ul)" >&2; exit 1; }
        done
      done
    done ;;
  calib)
    GATE="$(gate_first)" || exit 1; ensure_core; build
    # Requirement 19: the crossing counts of the counting build must equal the committed ones.
    cp "$SLICE/target-core-count/release/libak_core.so" "$H8/"
    AK_CROSSINGS_EXPECT="$SLICE/gen/crossings.txt" dotnet "$H8/harness.dll" coreffi > "$OUT/calib-crossing-counts.log" 2>&1 \
      || { echo "crossing counts differ from gen/crossings.txt: the run stops ($OUT/calib-crossing-counts.log)" >&2; exit 1; }
    cp "$SLICE/target-core-count-nounk/release/libak_core.so" "$HN8/"
    AK_CROSSINGS_EXPECT="$SLICE/gen/crossings-nounk.txt" dotnet "$HN8/harness.dll" coreffi > "$OUT/calib-crossing-counts-nounk.log" 2>&1 \
      || { echo "no-unknown crossing counts differ from gen/crossings-nounk.txt: the run stops ($OUT/calib-crossing-counts-nounk.log)" >&2; exit 1; }
    cp "$SLICE/target-core-nounk/release/libak_core.so" "$HN8/"
    cp "$SLICE/target-core/release/libak_core.so" "$H8/"
    cp "$SLICE/target-core/release/libak_core.so" "$R8/"
    ITERS=10000000; [ $SMOKE = 1 ] && ITERS=100000
    for l in $(seq 1 "$LAUNCHES"); do
      f="$OUT/calib-launch$l$ASFX.jsonl"
      { header "rpc,init-guard"; echo "$GATE"; echo "# crossing counts: equal to gen/crossings.txt (calib-crossing-counts.log) and, no-unknown build, to gen/crossings-nounk.txt (calib-crossing-counts-nounk.log)"; } > "$f"
      if command -v perf > /dev/null; then
        # Requirement 20: cycles and instructions per iteration, from perf stat, per process.
        taskset -c "$AK_CPU_CLIENT" perf stat -x, -e cycles,instructions -o "$OUT/calib-launch$l.perf" \
          dotnet "$R8/akrpc.dll" campaign --suite calib --launch "$l" --rounds "$ROUNDS" --iters "$ITERS" >> "$f" 2>&1 || exit 1
        sed 's/^/# perf stat: /' "$OUT/calib-launch$l.perf" >> "$f"
      else
        echo "# perf stat: perf is NOT installed on this machine; cycles/instructions not recorded (requirement 20 unmet here)" >> "$f"
        taskset -c "$AK_CPU_CLIENT" dotnet "$R8/akrpc.dll" campaign --suite calib --launch "$l" --rounds "$ROUNDS" --iters "$ITERS" >> "$f" 2>&1 || exit 1
      fi
    done ;;
  rpc)
    # D11 as amended: one gate per h2 variant (gen/gate.sh with AK_H2), before any figure.
    declare -A GATES
    for h in $H2_VARIANTS; do
      case "$h" in stock|h2-batch) ;; *) echo "unknown h2 variant $h" >&2; exit 2 ;; esac
      GATES[$h]="$(AK_H2=$h gate_first)" || exit 1
    done
    ensure_core; build
    WARM=2000; [ $SMOKE = 1 ] && WARM=100
    WARM="${AK_RPC_SERVER_WARM:-$WARM}"
    # WP9 (req 22a as amended): the client is BenchmarkDotNet (akrpc bench, RpcBench.cs), one
    # pinned process per unit = cell; its warm-up is BDN's (jitting stage, pilot, warm-up
    # iterations; req 24): campaign 10 warm-up iterations of 100 ms, smoke 1 of 20 ms.
    if [ $SMOKE = 1 ]; then BW=1; BI=20; else BW=10; BI=100; fi
    BDNARGS=(--toolchain "$TOOLCHAIN" --rounds "${AK_RPC_BDN_ROUNDS:-$ROUNDS}" --warmup "${AK_RPC_BDN_WARMUP:-$BW}" --iteration-ms "${AK_RPC_BDN_ITERATION_MS:-$BI}" --artifacts "$SCRATCH/bdn-rpc")
    # Req 13 as amended (R-H33): ONE server process per launch, serving every cell of both
    # builds over both transport configurations (two Kestrel hosts in it, one socket each),
    # warmed by $WARM calls per direction from each client transport (Grpc.Net, the core's)
    # on each socket before any client's round 1. Req 12 as amended: the full client runs
    # A B C-retain C-drop D-retain D-drop E-retain E-drop F-retain F-drop (+ extras), the
    # no-unknown client A B C-nounk D-nounk E-nounk F-nounk. Transports and builds in an order
    # alternated by launch.
    # FIX-PLAN WP10 (CAMPAIGN req 13 as amended): the server is the Rust slice's tonic
    # rpc_server, THE server of every slice (poc/rust/SERVER.md), built, started, warmed and
    # stopped through poc/rust/serve.sh, once per launch, pinned to AK_CPU_SERVER by serve.sh.
    SERVE="$REPO/ffi/poc/rust/serve.sh"
    export AK_SERVE_STATE="$SCRATCH/ak-rpc-server.state" AK_CPU_SERVER
    "$SERVE" build > "$OUT/rpc-server-build.log" 2>&1 || { tail -20 "$OUT/rpc-server-build.log"; exit 1; }
    for l in $(seq 1 "$LAUNCHES"); do
      sdir="$OUT/rpc-launch$l.server"; mkdir -p "$sdir"
      # FIX-PLAN WP13 (D10): every timed cell over TCP 127.0.0.1. The server's TCP listener
      # (AK_SERVER_TCP=0, any free port) runs its PINNED configuration only, so "shipped" and
      # "pinned" are the CLIENT's configuration against that one listener.
      env -u GLIBC_TUNABLES AK_SERVER_TCP=0 "$SERVE" start --out "$sdir" > "$sdir/start.out" 2>&1 || { cat "$sdir/start.out"; exit 1; }
      TL=$(sed -n 's/^tcp //p' "$sdir/start.out"); SPID=$(sed -n 's/^pid //p' "$sdir/start.out")
      [ -n "$TL" ] || { echo "the server printed no TCP listener ($sdir/start.out)" >&2; "$SERVE" stop; exit 1; }
      slog="$sdir/rpc-server.log"
      "$SERVE" warm "$WARM" > "$OUT/rpc-launch$l.server-warm.log" 2>&1 \
        || { echo "server warm-up failed ($OUT/rpc-launch$l.server-warm.log)" >&2; "$SERVE" stop; exit 1; }
      if [ $(( l % 2 )) = 1 ]; then TS="shipped pinned"; else TS="pinned shipped"; fi
      # The owner's small-test rule (2026-09-27): AK_RPC_TRANSPORTS / AK_RPC_BUILDS narrow a
      # smoke or exploration run to one transport or one build; the campaign runs all.
      [ -n "${AK_RPC_TRANSPORTS:-}" ] && TS="$AK_RPC_TRANSPORTS"
      sock="tcp:$TL"
      if [ $(( l % 2 )) = 1 ]; then HS="$H2_VARIANTS"; else HS="$(echo $H2_VARIANTS | tr ' ' '\n' | tac | tr '\n' ' ')"; fi
      for h2 in $HS; do
      # The variant's cores into both client builds (gen/build_core.sh: target-core*-h2b);
      # every client process checks the loaded core's h2 equals AK_H2 (RpcBench) and labels its rows.
      hs=""; [ "$h2" = h2-batch ] && hs="-h2b"
      cp "$SLICE/target-core$hs/release/libak_core.so" "$R8/"
      cp "$SLICE/target-core-nounk$hs/release/libak_core.so" "$RN8/"
      export AK_H2="$h2"
      for t in $TS; do
        for bld in ${AK_RPC_BUILDS:-$(builds_of "$l")}; do
          if [ "$bld" = full ]; then RX="$R8"; sfx=""; else RX="$RN8"; sfx=".nounk"; fi
          f="$OUT/rpc-$t-$h2-launch$l$sfx$ASFX.jsonl"
          [ $PLANT = 1 ] && f="$OUT/rpc-$t-$h2-launch$l$sfx$ASFX.PLANT.jsonl"
          { header "rpc,init-guard$([ "$bld" = nounk ] && echo ' without unknown-fields (no-unknown build)')$([ "$h2" = h2-batch ] && echo ', h2 patched to h2-batch (poc/codec/h2-batch)')"; echo "${GATES[$h2]}";
            echo "# client build: $bld (WP5 step 10); h2 $h2 (D11 as amended); this launch's order: h2 $HS, transports $TS, builds $(builds_of "$l")";
            echo "# network:       TCP 127.0.0.1 ($TL), the server's TCP listener (D10, FIX-PLAN WP13): it runs the PINNED server configuration only (4 MiB windows, adaptive off, TCP_NODELAY on accept), so shipped and pinned differ on the CLIENT side only; Nagle off on every client socket, read back in each process (see the '# network' line of each unit)";
            echo "# server:        the Rust slice's tonic rpc_server (FIX-PLAN WP10, poc/rust/SERVER.md), one process for this launch (pid $SPID, $slog, pinned to ${AK_CPU_SERVER:-unpinned} by serve.sh, workers AK_SERVER_THREADS=$AK_SERVER_THREADS), every h2 variant, build and client transport against its TCP listener; warmed first by serve.sh warm $WARM ($OUT/rpc-launch$l.server-warm.log: $WARM checked calls per direction a, b, c and $(( (WARM + 3) / 4 )) on d, from a tonic and a core client, on its two Unix sockets and its TCP listener)"; } > "$f"
          if [ $PLANT = 1 ]; then
            # Req 18's controls, per send path and per direction (WP8): a wrong expected length
            # on a, c and d, and a wrong expected SHA-256 on d, each on one cell at a time
            # (A Grpc.Net, B the core's reference path, Bf its framed path, C the move path,
            # D Grpc.Net + core-ffi); each through akrpc bench (WP9): the benchmark must fail,
            # the process exit non-zero, and no sample be written;
            # every one must abort with no sample.
            if [ "$bld" = full ]; then DC=D-drop; CC=C-drop; else DC=D-nounk; CC=C-nounk; fi
            for pc in "len a" "len c" "len d" "digest d"; do
              set -- $pc
              for cell in A B Bf $CC $DC; do
                [ "$2" = a ] && [ "$cell" = Bf ] && continue   # direction a has no framed twin
                AK_CAMPAIGN_PLANT=$1 AK_CAMPAIGN_PLANT_DIR=$2 taskset -c "$AK_CPU_CLIENT" dotnet "$RX/akrpc.dll" bench --sock "$sock" --transport "$t" \
                  --unit "$cell" --launch "$l" --inflight 1 --out "$f.$1-$2-$cell" "${BDNARGS[@]}" > "$f.$1-$2-$cell.bdn.log" 2>&1; rc=$?
                if [ $rc -eq 0 ]; then echo "CONTROL PASSED: plant $1 on $2 did not abort on $cell ($f.$1-$2-$cell)" >&2; "$SERVE" stop; exit 1; fi
                echo "control ($t, h2 $h2, launch $l, $bld, plant $1, direction $2, cell $cell): aborted as required, $(grep -c '^{' "$f.$1-$2-$cell") samples: $(grep -m1 ABORT "$f.$1-$2-$cell" | cut -c1-160)" | tee -a "$f"
                rm -f "$f.$1-$2-$cell" "$f.$1-$2-$cell.bdn.log"
              done
            done
            # D10 (WP13): Nagle left on in both client transports; the TCP_NODELAY readback in
            # each case's setup must fail A (Grpc.Net) and B (the core) with no sample.
            for cell in A B; do
              AK_CAMPAIGN_PLANT=nagle taskset -c "$AK_CPU_CLIENT" dotnet "$RX/akrpc.dll" bench --sock "$sock" --transport "$t" \
                --unit "$cell" --launch "$l" --inflight 1 --out "$f.nagle-$cell" "${BDNARGS[@]}" > "$f.nagle-$cell.bdn.log" 2>&1; rc=$?
              if [ $rc -eq 0 ]; then echo "CONTROL PASSED: Nagle on did not abort $cell ($f.nagle-$cell)" >&2; "$SERVE" stop; exit 1; fi
              echo "control ($t, h2 $h2, launch $l, $bld, plant nagle, cell $cell): aborted as required, $(grep -c '^{' "$f.nagle-$cell") samples: $(grep -m1 'TCP_NODELAY read back' "$f.nagle-$cell.bdn.log" | cut -c1-160)" | tee -a "$f"
              rm -f "$f.nagle-$cell" "$f.nagle-$cell.bdn.log"
            done
            continue
          fi
          for u in $(dotnet "$RX/akrpc.dll" bench --launch "$l" --list-units); do
            ul="$OUT/rpc-$t-$h2-launch$l$sfx$ASFX.$u.bdn.log"
            { [ $SMOKE = 1 ] && echo "# SMOKE RUN in a container: EVERY FIGURE IN THIS LOG IS INSTRUMENTATION, NOT A RESULT (README 1.1)"; } > "$ul"
            taskset -c "$AK_CPU_CLIENT" dotnet "$RX/akrpc.dll" bench --sock "$sock" --transport "$t" --unit "$u" --launch "$l" --out "$f" "${BDNARGS[@]}" >> "$ul" 2>&1; rc=$?
            if [ $rc -ne 0 ]; then
              # Req 18 / 22a: one failed benchmark discards the launch's output: no figure.
              for x in "$OUT"/rpc-*-launch$l*.jsonl; do [ -f "$x" ] && mv "$x" "$x.DISCARDED"; done
              echo "rpc $t h2 $h2 launch $l ($bld) unit $u failed ($ul): the launch's output is discarded (*.DISCARDED)" >&2; "$SERVE" stop; exit 1
            fi
          done
        done
      done
      done
      "$SERVE" stop > /dev/null
    done ;;
  *) echo "unknown suite $SUITE" >&2; exit 2 ;;
esac
echo "done: $SUITE -> $OUT"
