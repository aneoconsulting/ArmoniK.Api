#!/usr/bin/env bash
# s14 part 1: the distribution of the one-string sweep (UploadResultDataMessage, ASCII 48 and
# Latin-1 48, paths E0 and E1R) over N processes per JIT configuration. CONTAINER
# INSTRUMENTATION. Resumable per process: a process whose line "# p I CFG: rc=0" is in header.txt
# and whose tsv exists is skipped (the container restarts); a partial one is moved to
# cut/ and rerun. Processes are interleaved (process i of every configuration, then i+1).
#   gen/s14_dist.sh OUTDIR [N]
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$SLICE"
OUT="$1"; N="${2:-20}"; mkdir -p "$OUT/cut"; OUT="$(cd "$OUT" && pwd)"
export MSBUILDDISABLENODEREUSE=1 DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
BD="$SLICE/src/BenchDotNet/bin/Release/net8.0"
dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release > "$OUT/build.log" 2>&1 || { tail "$OUT/build.log"; exit 1; }
cp "$SLICE/target-core/release/libak_core.so" "$BD/"
CFGS=("default:X_S14=1" "pgo0:DOTNET_TieredPGO=0" "tc0:DOTNET_TieredCompilation=0" "osr0:DOTNET_TC_OnStackReplacement=0" "r2r0:DOTNET_ReadyToRun=0")
quiet() {
  local la hot
  for i in $(seq 1 120); do
    la=$(cut -d' ' -f1 /proc/loadavg)
    hot=$(ps -eo pcpu,comm --sort=-pcpu --no-headers | awk '$1 > 10 && $2 != "ps" {print $2"("$1"%)"}' | head -3 | tr '\n' ' ')
    if awk -v l="$la" 'BEGIN{exit !(l < 0.5)}' && [ -z "$hot" ]; then echo "quiet: load1 $la (waited $((i * 5 - 5)) s)"; return 0; fi
    sleep 5
  done
  echo "NOT QUIET after 10 min: load1 $la, hot: $hot"
}
echo "# s14_dist.sh invocation $(date -u +%FT%TZ), boot $(uptime -s), commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec || echo ' + uncommitted'); core $(sha256sum "$BD/libak_core.so" | cut -c1-16); CPUs 0,1; --strsweep --paths E0,E1R --lengths 48 --contents ascii,latin1 --rounds 6; N=$N" >> "$OUT/header.txt"
for i in $(seq 1 "$N"); do
  for c in "${CFGS[@]}"; do
    name="${c%%:*}"; env1="${c#*:}"; f="$OUT/$name-$i"
    if grep -q "^# p $i $name: rc=0" "$OUT/header.txt" && [ -s "$f.tsv" ]; then continue; fi
    for x in "$f".*; do [ -e "$x" ] && mv "$x" "$OUT/cut/$(basename "$x")-$(date +%s)"; done
    q=$(quiet); t0=$(date +%s)
    env "$env1" taskset -c 0,1 dotnet "$BD/BenchDotNet.dll" --strsweep "$f.tsv" --paths E0,E1R --lengths 48 --contents ascii,latin1 --rounds 6 > "$f.log" 2>&1; rc=$?
    echo "# p $i $name: rc=$rc $(( $(date +%s) - t0 )) s; $env1; boot $(uptime -s); $q" >> "$OUT/header.txt"
  done
done
echo "# s14_dist.sh done $(date -u +%FT%TZ)" >> "$OUT/header.txt"
