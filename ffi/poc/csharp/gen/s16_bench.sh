#!/usr/bin/env bash
# s16 (owner, 2026-10-10): reset-on-entry against the explicit resets, both paths IN ONE
# PROCESS (BenchDotNet --roebench, the AkRoeBench build, on the variant core target-core-roe:
# the explicit path is the default binding with the core's reset-on-entry switched off on its
# contexts, the variant path the binding rendered with reset_on_entry=True, and E2 the explicit
# path again as the A/A control). CONTAINER INSTRUMENTATION. Processes: NFULL over every core-ffi
# case of the core grid's encode-core-hot and decode-read rows (drop and retain), NSMALL over
# the small rows only (P1.1, P5.1, P7.1 and the U-* rows), interleaved; each process 21 rounds x
# 20 ms per path per case after a 1 s warm-up, then the reset calls alone. Resumable per process
# (a process whose "# p NAME: rc=0" line is in header.txt and whose tsv exists is skipped; a
# partial one is moved to cut/); a quiet machine before each; client CPUs 0,1.
#   gen/s16_bench.sh OUTDIR [NFULL [NSMALL]]
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$SLICE"
OUT="$1"; NFULL="${2:-5}"; NSMALL="${3:-10}"; mkdir -p "$OUT/cut"; OUT="$(cd "$OUT" && pwd)"
export MSBUILDDISABLENODEREUSE=1 DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
RB="$SLICE/src/BenchDotNet/bin-roebench/Release/net8.0"
dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkRoeBench=true > "$OUT/build.log" 2>&1 || { tail "$OUT/build.log"; exit 1; }
CORE="$SLICE/target-core-roe/release/libak_core.so"; cp "$CORE" "$RB/"
SMALL="P1.1,P5.1,P7.1,U-nested-before,U-deep-u-repeated,U-oneof-u-repeated,U-wire-ListTaskSummaryResponse-tasks-as-wt5,U-wire-UploadResultDataMessage-upload-as-wt5,U-wire-ListMetricsResponse-batches-as-wt0,U-wire-DualResponse-left-as-wt5"
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
echo "# s16_bench.sh invocation $(date -u +%FT%TZ), boot $(uptime -s), commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec || echo ' + uncommitted'); core $(sha256sum "$CORE" | cut -c1-16) (target-core-roe, AK_CORE_LIB); CPUs 0,1; --roebench --rounds 21 --block-ms 20 --warm-ms 1000; NFULL=$NFULL NSMALL=$NSMALL; small rows: $SMALL" >> "$OUT/header.txt"
PROCS=()
for i in $(seq 1 $(( NFULL > NSMALL ? NFULL : NSMALL ))); do
  [ "$i" -le "$NFULL" ] && PROCS+=("full-$i|")
  [ "$i" -le "$NSMALL" ] && PROCS+=("small-$i|$SMALL")
done
for pr in "${PROCS[@]}"; do
  name="${pr%%|*}"; only="${pr#*|}"; f="$OUT/$name"
  if grep -q "^# p $name: rc=0" "$OUT/header.txt" && [ -s "$f.tsv" ]; then continue; fi
  for x in "$f".*; do [ -e "$x" ] && mv "$x" "$OUT/cut/$(basename "$x")-$(date +%s)"; done
  q=$(quiet); t0=$(date +%s)
  env AK_CAMPAIGN_GRID=core AK_BDN_DROP=1 AK_ROE_ONLY="$only" AK_CORE_LIB="$CORE" taskset -c 0,1 dotnet "$RB/BenchDotNet.dll" --roebench "$f.tsv" --rounds 21 --block-ms 20 --warm-ms 1000 > "$f.log" 2>&1; rc=$?
  echo "# p $name: rc=$rc $(( $(date +%s) - t0 )) s; boot $(uptime -s); $q; $(tail -n 1 "$f.log" | cut -c1-60)" >> "$OUT/header.txt"
done
echo "# s16_bench.sh done $(date -u +%FT%TZ)" >> "$OUT/header.txt"
