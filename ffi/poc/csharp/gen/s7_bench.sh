#!/usr/bin/env bash
# D21 step 7 one-process benches (CONTAINER INSTRUMENTATION): the string-length sweep over every
# path, the kernel experiments (the same sweep under .NET vector-width and simdutf
# implementation switches), and the pin microbench. Each process waits for a quiet machine.
#   gen/s7_bench.sh OUTDIR [sweep|kernels|pinbench ...]
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$SLICE"
OUT="$1"; shift; mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
export MSBUILDDISABLENODEREUSE=1
dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -f net8.0 > "$OUT/build.log" 2>&1 || { tail "$OUT/build.log"; exit 1; }
B="$SLICE/src/BenchDotNet/bin/Release/net8.0/BenchDotNet.dll"
cp "$SLICE/target-core/release/libak_core.so" "$(dirname "$B")/"
quiet() {
  local la hot
  for i in $(seq 1 120); do
    la=$(cut -d' ' -f1 /proc/loadavg)
    hot=$(ps -eo pcpu,comm --sort=-pcpu --no-headers | awk '$1 > 10 && $2 != "ps" {print $2"("$1"%)"}' | head -3 | tr '\n' ' ')
    if awk -v l="$la" 'BEGIN{exit !(l < 0.5)}' && [ -z "$hot" ]; then echo "quiet: load1 $la, no process above 10 % (waited $((i * 5 - 5)) s)"; return 0; fi
    sleep 5
  done
  echo "NOT QUIET after 10 min: load1 $la, hot: $hot"
}
one() {  # name env... -- args...
  local name="$1"; shift; local envs=(); while [ "$1" != "--" ]; do envs+=("$1"); shift; done; shift
  local q; q=$(quiet); local t0; t0=$(date +%s)
  env "${envs[@]}" taskset -c 0,1 dotnet "$B" "$@" > "$OUT/$name.log" 2>&1; local rc=$?
  echo "# $name: ${envs[*]} | $*: rc=$rc $(( $(date +%s) - t0 )) s; $q" | tee -a "$OUT/header.txt"
}
{ echo "# s7_bench.sh, CONTAINER INSTRUMENTATION; commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec/gen || echo ' + uncommitted'); $(date -u +%FT%TZ); client CPUs 0,1"
  echo "# core $(sha256sum target-core/release/libak_core.so | cut -c1-16); CPU $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2) family $(grep -m1 'cpu family' /proc/cpuinfo | cut -d: -f2) model $(grep -m1 '^model\s*:' /proc/cpuinfo | cut -d: -f2)"; } >> "$OUT/header.txt"
ALL="E0,E1,E2,E3,E3L,E1R,E1C"
for w in "$@"; do
  case "$w" in
    sweep) one sweep -- --strsweep "$OUT/sweep.tsv" --paths "$ALL" ;;
    kernels)
      one kernels-probe -- --kernels
      for e in "K=default" "DOTNET_EnableAVX512F=0" "DOTNET_PreferredVectorBitWidth=512" "SIMDUTF_FORCE_IMPLEMENTATION=westmere" "SIMDUTF_FORCE_IMPLEMENTATION=fallback"; do
        n=$(echo "$e" | tr '=' '-'); one "k-$n" "$e" -- --strsweep "$OUT/k-$n.tsv" --paths E0,E1,E3 --lengths 64,1024,16384 --contents ascii,latin1,wide
      done ;;
    pinbench) one pinbench -- --pinbench "$OUT/pinbench.tsv" ;;
  esac
done
