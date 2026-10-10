#!/usr/bin/env bash
# s15 measurement 1: the s14 one-string distribution (UploadResultDataMessage, ASCII 48 and
# Latin-1 48, paths E0 and E1R, 6 rounds) over N processes per configuration, on the D26 code
# (no thread-local storage) under the default JIT configuration and DOTNET_TieredPGO=0, with the
# code before D26 (BASE, a worktree) under the default configuration as the same-session control.
# CONTAINER INSTRUMENTATION. Resumable per process (as gen/s14_dist.sh): a process whose line
# "# p I CFG: rc=0" is in header.txt and whose tsv exists is skipped; a partial one is moved to
# cut/ and rerun. Interleaved (process i of every configuration, then i+1); client CPUs 0,1;
# a quiet machine before each (load1 < 0.5, no process above 10 %).
#   gen/s15_dist.sh OUTDIR BASE_SLICE_DIR [N]
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$SLICE"
OUT="$1"; BASE="$(cd "$2" && pwd)"; N="${3:-20}"; mkdir -p "$OUT/cut"; OUT="$(cd "$OUT" && pwd)"
export MSBUILDDISABLENODEREUSE=1 DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
for tt in "new|$SLICE" "base|$BASE"; do
  t="${tt#*|}"
  ( cd "$t" && dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release ) > "$OUT/build-${tt%%|*}.log" 2>&1 || { echo "build $t failed" >&2; exit 1; }
  cp "$SLICE/target-core/release/libak_core.so" "$t/src/BenchDotNet/bin/Release/net8.0/"
done
CFGS=("new-default|$SLICE|X_S15=1" "new-pgo0|$SLICE|DOTNET_TieredPGO=0" "base-default|$BASE|X_S15=1")
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
echo "# s15_dist.sh invocation $(date -u +%FT%TZ), boot $(uptime -s); new $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec || echo ' + uncommitted'); base $BASE at $(git -C "$BASE" rev-parse --short HEAD)$(git -C "$BASE" diff --quiet HEAD -- src gen || echo ' + uncommitted'); core $(sha256sum "$SLICE/target-core/release/libak_core.so" | cut -c1-16); CPUs 0,1; --strsweep --paths E0,E1R --lengths 48 --contents ascii,latin1 --rounds 6; N=$N" >> "$OUT/header.txt"
for i in $(seq 1 "$N"); do
  for c in "${CFGS[@]}"; do
    IFS='|' read -r name tree env1 <<< "$c"; f="$OUT/$name-$i"
    if grep -q "^# p $i $name: rc=0" "$OUT/header.txt" && [ -s "$f.tsv" ]; then continue; fi
    for x in "$f".*; do [ -e "$x" ] && mv "$x" "$OUT/cut/$(basename "$x")-$(date +%s)"; done
    q=$(quiet); t0=$(date +%s)
    env "$env1" taskset -c 0,1 dotnet "$tree/src/BenchDotNet/bin/Release/net8.0/BenchDotNet.dll" --strsweep "$f.tsv" --paths E0,E1R --lengths 48 --contents ascii,latin1 --rounds 6 > "$f.log" 2>&1; rc=$?
    echo "# p $i $name: rc=$rc $(( $(date +%s) - t0 )) s; tree $tree; $env1; boot $(uptime -s); $q" >> "$OUT/header.txt"
  done
done
echo "# s15_dist.sh done $(date -u +%FT%TZ)" >> "$OUT/header.txt"
