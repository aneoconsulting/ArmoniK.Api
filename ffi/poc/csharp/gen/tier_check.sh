#!/usr/bin/env bash
# JOURNAL 73: does a one-CPU client measure tier-0 code? Container instrumentation, not gated.
# The full build's three codec units on P1.1 and P6.1 (encode-transport-hot, decode-read; no U-*
# rows: AK_BDN_ONLY, AK_BDN_NO_UNKNOWN), the opt_bench codec settings, one BDN run per variant:
#   1cpu            taskset CPU 1, default toolchain (as the VOID baseline)
#   1cpu-delay0     the same, DOTNET_TC_CallCountingDelayMs=0
#   1cpu-tc0        the same, DOTNET_TieredCompilation=0 (full JIT from the first call)
#   2cpu            taskset CPUs 0,1, default toolchain
#   1cpu-grouped    CPU 1, InProcessEmit (the slice's JIT-tier readback runs: the jit check line)
#   2cpu-grouped    CPUs 0,1, InProcessEmit
#   gen/tier_check.sh --out DIR
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT=""; [ "${1:-}" = --out ] && OUT="$2"
[ -n "$OUT" ] || { echo "usage: gen/tier_check.sh --out DIR" >&2; exit 2; }
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
export SCRATCH="${SCRATCH:-$(mktemp -d)}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
export AK_CAMPAIGN_GRID=core AK_CAMPAIGN_ALLOC=default AK_BDN_ONLY=P1.1,P6.1 AK_BDN_NO_UNKNOWN=1
unset GLIBC_TUNABLES DOTNET_TieredCompilation DOTNET_TC_CallCountingDelayMs
B8="$SLICE/src/BenchDotNet/bin/Release/net8.0"
cp "$SLICE/target-core/release/libak_core.so" "$B8/"
{ echo "# tier check (JOURNAL 73), commit $(git -C "$SLICE" rev-parse --short HEAD), $(date -u +%FT%TZ); CONTAINER INSTRUMENTATION, not gated (Cases.Verify on)";
  echo "# cases: full build, units incumbent-prod:default, core-ffi:retain, host-gen:retain; P1.1 and P6.1; encode-transport-hot and decode-read; BDN --warmup 4 --rounds 6 --iteration-ms 40"; } > "$OUT/header.txt"
run() {  # name cpus toolchain env...
  local n=$1 c=$2 tc=$3; shift 3
  local t0; t0=$(date +%s.%N)
  env "$@" taskset -c "$c" dotnet "$B8/BenchDotNet.dll" --launch 1 --out "$OUT/$n.jsonl" --artifacts "$SCRATCH/bdn-$n" \
    --rounds 6 --warmup 4 --iteration-ms 40 --toolchain "$tc" > "$OUT/$n.bdn.log" 2>&1
  echo "# $n: cpus $c, toolchain $tc, env [$*]: rc=$? $(python3 -c "import time;print(f'{time.time()-$t0:.0f}')") s" | tee -a "$OUT/header.txt"
}
run 1cpu 1 process AK_ALLOW_SINGLE_CPU=1
run 1cpu-delay0 1 process AK_ALLOW_SINGLE_CPU=1 DOTNET_TC_CallCountingDelayMs=0
run 1cpu-tc0 1 process AK_ALLOW_SINGLE_CPU=1 DOTNET_TieredCompilation=0
run 2cpu 0,1 process
run 1cpu-grouped 1 grouped AK_ALLOW_SINGLE_CPU=1
run 2cpu-grouped 0,1 grouped
run 1cpu-guard 1 process   # the guard: must refuse (rc != 0, no rows)
