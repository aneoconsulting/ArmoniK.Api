#!/usr/bin/env bash
# s13: the string-length sweep (one string per encode, s7/threshold's form) per transcoder core,
# E0 (in-process control) and E1R, 2 reps, core order alternated; client CPUs 0,1; a quiet
# machine before each process. CONTAINER INSTRUMENTATION.
#   [SWEEP_ENV="K=V ..."] gen/s13_sweep.sh OUTDIR
# SWEEP_ENV is set for every timed process (s13: DOTNET_TieredPGO=0, after the default JIT
# configuration put E1R in a slow state in some processes and not others: see the JOURNAL).
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$SLICE"
OUT="$1"; mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
export MSBUILDDISABLENODEREUSE=1
dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -f net8.0 > "$OUT/build.log" 2>&1 || { tail "$OUT/build.log"; exit 1; }
BD="$SLICE/src/BenchDotNet/bin/Release/net8.0"
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
echo "# s13_sweep.sh, CONTAINER INSTRUMENTATION; commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec || echo ' + uncommitted'); $(date -u +%FT%TZ); client CPUs 0,1; lengths 16,32,40,48,64,96,128,192,256,1024,16384; contents ascii,latin1,wide,astral; paths E0,E1R; env: ${SWEEP_ENV:-(none)}" >> "$OUT/header.txt"
CORES=(target-core target-core-tcnaive target-core-tcword)
for r in 1 2; do
  if [ $r = 1 ]; then ORD=("${CORES[@]}"); else ORD=(target-core-tcword target-core-tcnaive target-core); fi
  for c in "${ORD[@]}"; do
    cp "$SLICE/$c/release/libak_core.so" "$BD/"
    q=$(quiet); t0=$(date +%s)
    env ${SWEEP_ENV:-} taskset -c 0,1 dotnet "$BD/BenchDotNet.dll" --strsweep "$OUT/$c-r$r.tsv" --paths E0,E1R --lengths 16,32,40,48,64,96,128,192,256,1024,16384 --contents ascii,latin1,wide,astral > "$OUT/$c-r$r.log" 2>&1; rc=$?
    echo "# rep $r $c (core $(sha256sum "$BD/libak_core.so" | cut -c1-16)): rc=$rc $(( $(date +%s) - t0 )) s; $q" | tee -a "$OUT/header.txt"
  done
done
cp "$SLICE/target-core/release/libak_core.so" "$BD/"
