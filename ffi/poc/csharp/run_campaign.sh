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
CODE="ffi/poc/csharp/src ffi/poc/csharp/gen ffi/poc/csharp/abi ffi/poc/csharp/run_campaign.sh ffi/poc/csharp/Directory.Build.props"
# (STATE.md and JOURNAL.md are notes, not inputs of a run.)
if ! git -C "$REPO" diff --quiet HEAD -- $CODE ffi/schema ffi/corpus \
   || [ -n "$(git -C "$REPO" status --porcelain --untracked-files=normal -- ffi/poc/csharp/src ffi/poc/csharp/gen ffi/poc/csharp/run_campaign.sh)" ]; then
  DIRTY=1
  if [ "${AK_ALLOW_DIRTY:-0}" != "1" ]; then echo "refused: the tree is dirty (requirement 27); commit, or AK_ALLOW_DIRTY=1" >&2; exit 3; fi
fi
if [ -z "${AK_CPU_CLIENT:-}" ] || { [ "$SUITE" = rpc ] && [ -z "${AK_CPU_SERVER:-}" ]; }; then
  if [ $SMOKE = 1 ]; then AK_CPU_CLIENT="${AK_CPU_CLIENT:-0}"; AK_CPU_SERVER="${AK_CPU_SERVER:-1}"; CPUNOTE=" (smoke defaults)"
  else echo "AK_CPU_CLIENT (and AK_CPU_SERVER for rpc) must be set (requirement 4)" >&2; exit 2; fi
fi
R8="$SLICE/src/Rpc/bin/Release/net8.0"
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
  echo "# cpu sets:      CLIENT=$AK_CPU_CLIENT SERVER=${AK_CPU_SERVER:-n/a}${CPUNOTE:-}; pinning by taskset"
  echo "# runtime:       .NET $(dotnet --list-runtimes | awk '/NETCore.App/{print $2}' | tr '\n' ' ')(SDK $(dotnet --version)); target net8.0, Release; tiering and PGO at their net8.0 defaults unless DOTNET_* is set: TieredCompilation=${DOTNET_TieredCompilation:-default} TieredPGO=${DOTNET_TieredPGO:-default}; workstation GC, concurrent (default)"
  echo "# core:          libak_core.so shared, cargo --release, features $1 (init-guard ON, as in every gate), built from git archive HEAD ffi/poc/codec"
  if [ "$SUITE" = codec ]; then echo "# repeats:       $LAUNCHES launch(es) x $ROUNDS BDN actual iteration(s) per case; arm blocks, order rotated per launch (req 22)"; else echo "# repeats:       $LAUNCHES launch(es) x $ROUNDS round(s); arms/cells interleaved per round, rotated"; fi
}

ensure_core() {
  if [ ! -f "$SLICE/target-core/release/libak_core.so" ] || [ ! -f "$SLICE/target-core-count/release/libak_core.so" ]; then
    "$SLICE/gen/build_core.sh" > "$OUT/build-core.log" 2>&1 || { cat "$OUT/build-core.log"; exit 1; }
  fi
}
build() {
  ( cd "$SLICE" && dotnet build src/Rpc/Rpc.csproj -c Release > "$SCRATCH/campaign-build.out" 2>&1 ) || { tail -30 "$SCRATCH/campaign-build.out"; exit 1; }
  ( cd "$SLICE" && dotnet build src/Harness/Harness.csproj -c Release -f net8.0 >> "$SCRATCH/campaign-build.out" 2>&1 ) || { tail -30 "$SCRATCH/campaign-build.out"; exit 1; }
}
gate_first() {  # requirement 26
  # A passed gate is reused when the commit it ran at has the SAME content as HEAD in every
  # path a run depends on (other slices commit to the branch concurrently).
  if [ -f "$OUT/gate.log" ] && grep -q "^GATE PASSED" "$OUT/gate.log" && ! grep -q "^# branch HEAD: .*uncommitted" "$OUT/gate.log"; then
    local g; g="$(sed -n 's/^# branch HEAD: \([0-9a-f]*\).*/\1/p' "$OUT/gate.log" | head -1)"
    if [ -n "$g" ] && git -C "$REPO" diff --quiet "$g" HEAD -- $CODE ffi/poc/codec ffi/schema ffi/corpus; then
      echo "# gate:          passed at $g, identical to $COMMIT in ffi/poc/csharp, poc/codec, schema, corpus ($OUT/gate.log)"
      return
    fi
  fi
  "$SLICE/gen/gate.sh" > "$OUT/gate.log" 2>&1
  if ! grep -q "^GATE PASSED" "$OUT/gate.log"; then echo "the correctness gate FAILED ($OUT/gate.log): no figure is produced" >&2; exit 1; fi
  echo "# gate:          passed at $COMMIT ($OUT/gate.log)"
}

case "$SUITE" in
  gate)
    "$SLICE/gen/gate.sh" > "$OUT/gate.log" 2>&1; rc=$?
    tail -1 "$OUT/gate.log"; exit $rc ;;
  codec)
    # The codec suite runs under BenchmarkDotNet (CAMPAIGN.md 22a): src/BenchDotNet, InProcessEmit
    # toolchain, one pinned process per launch; the arm order rotates with the launch (req 22).
    # BDN's console log goes to DIR/codec-launch<N>.bdn.log, its artifacts to DIR/bdn-launch<N>/.
    GATE="$(gate_first)"; ensure_core; build
    ( cd "$SLICE" && dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release >> "$SCRATCH/campaign-build.out" 2>&1 ) || { tail -30 "$SCRATCH/campaign-build.out"; exit 1; }
    B8="$SLICE/src/BenchDotNet/bin/Release/net8.0"
    cp "$SLICE/target-core/release/libak_core.so" "$B8/"
    EXTRA=(--rounds "$ROUNDS"); [ $SMOKE = 1 ] && EXTRA=(--smoke)
    for l in $(seq 1 "$LAUNCHES"); do
      f="$OUT/codec-launch$l.jsonl"
      { header "rpc,init-guard"; echo "$GATE"; } > "$f"
      taskset -c "$AK_CPU_CLIENT" dotnet "$B8/BenchDotNet.dll" --launch "$l" --out "$f" --artifacts "$OUT/bdn-launch$l" "${EXTRA[@]}" \
        > "$OUT/codec-launch$l.bdn.log" 2>&1 || { echo "codec launch $l failed ($f, $OUT/codec-launch$l.bdn.log)" >&2; exit 1; }
    done ;;
  calib)
    GATE="$(gate_first)"; ensure_core; build
    # Requirement 19: the crossing counts of the counting build must equal the committed ones.
    cp "$SLICE/target-core-count/release/libak_core.so" "$H8/"
    AK_CROSSINGS_EXPECT="$SLICE/gen/crossings.txt" dotnet "$H8/harness.dll" coreffi > "$OUT/calib-crossing-counts.log" 2>&1 \
      || { echo "crossing counts differ from gen/crossings.txt: the run stops ($OUT/calib-crossing-counts.log)" >&2; exit 1; }
    cp "$SLICE/target-core/release/libak_core.so" "$H8/"
    cp "$SLICE/target-core/release/libak_core.so" "$R8/"
    ITERS=10000000; [ $SMOKE = 1 ] && ITERS=100000
    for l in $(seq 1 "$LAUNCHES"); do
      f="$OUT/calib-launch$l.jsonl"
      { header "rpc,init-guard"; echo "$GATE"; echo "# crossing counts: equal to gen/crossings.txt (calib-crossing-counts.log)"; } > "$f"
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
    GATE="$(gate_first)"; ensure_core; build
    cp "$SLICE/target-core/release/libak_core.so" "$R8/"
    CALLS=64; [ $SMOKE = 1 ] && CALLS=16
    for t in shipped pinned; do
      for l in $(seq 1 "$LAUNCHES"); do
        f="$OUT/rpc-$t-launch$l.jsonl"
        [ $PLANT = 1 ] && f="$OUT/rpc-$t-launch$l.PLANT.jsonl"
        sock="/tmp/ak-cs-campaign-$$.sock"; rm -f "$sock"   # a Unix socket path is at most 108 bytes
        { header "rpc,init-guard"; echo "$GATE"; } > "$f"
        taskset -c "$AK_CPU_SERVER" dotnet "$R8/akrpc.dll" campaign --suite rpc-server --sock "$sock" --transport "$t" > "$OUT/rpc-$t-launch$l.server.log" 2>&1 &
        SPID=$!
        for i in $(seq 1 100); do [ -S "$sock" ] && break; sleep 0.1; done
        [ -S "$sock" ] || { echo "server did not start" >&2; kill $SPID; exit 1; }
        [ $PLANT = 1 ] && export AK_CAMPAIGN_PLANT=len
        taskset -c "$AK_CPU_CLIENT" dotnet "$R8/akrpc.dll" campaign --suite rpc --sock "$sock" --transport "$t" \
          --launch "$l" --rounds "$ROUNDS" --calls "$CALLS" >> "$f" 2>&1; rc=$?
        unset AK_CAMPAIGN_PLANT
        kill $SPID; wait $SPID 2>/dev/null
        if [ $PLANT = 1 ]; then
          if [ $rc -eq 0 ]; then echo "CONTROL PASSED: a wrong length did not abort ($f)" >&2; exit 1; fi
          echo "control ($t, launch $l): aborted as required, $(grep -c '^{' "$f") samples written"
          continue
        fi
        [ $rc -eq 0 ] || { echo "rpc $t launch $l aborted ($f)" >&2; exit 1; }
      done
    done ;;
  *) echo "unknown suite $SUITE" >&2; exit 2 ;;
esac
echo "done: $SUITE -> $OUT"
