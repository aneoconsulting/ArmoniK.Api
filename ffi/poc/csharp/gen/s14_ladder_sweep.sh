#!/usr/bin/env bash
# s14 part 2: the E0 -> E1R ladder in the one-string sweep (UploadResultDataMessage; ASCII and
# Latin-1; 40 and 48 units; paths E0 and E1R in every process, so each process carries its own
# E0 control). CONTAINER INSTRUMENTATION. Process kinds (core x env):
#   simd     default core                      -> R0 (E0), R3 (E1R)
#   noguard  default core, AK_STR_NOGUARD=1    -> R3g (E1R without the per-encode guard)
#   generic  core tc-measure-generic           -> R1 (E0 through the generic transcoder path)
#   stub     core tc-measure-u16stub           -> R2 (E1R with the UTF-16 stub; bytes NOT checked)
#   stubng   stub core, AK_STR_NOGUARD=1       -> R2 without the guard
# Configurations: DOTNET_TieredPGO=0, and the default JIT configuration where a process in the
# slow mode of part 1 (E1R at or above 300 ns) is set aside (renamed -slow) and rerun, up to 4
# times, so the default column is of fast-mode processes (said in the header). Resumable per
# process; client CPUs 0,1; quiet wait before each.
#   gen/s14_ladder_sweep.sh OUTDIR [REPS]
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$SLICE"
OUT="$1"; REPS="${2:-3}"; mkdir -p "$OUT/cut"; OUT="$(cd "$OUT" && pwd)"
export MSBUILDDISABLENODEREUSE=1 DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
BD="$SLICE/src/BenchDotNet/bin/Release/net8.0"
dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release > "$OUT/build.log" 2>&1 || { tail "$OUT/build.log"; exit 1; }
cp "$SLICE/target-core/release/libak_core.so" "$BD/"
KINDS=("simd|target-core|X_S14=1" "noguard|target-core|AK_STR_NOGUARD=1" "generic|target-core-mgeneric|X_S14=1" "stub|target-core-mstub|X_S14=1" "stubng|target-core-mstub|AK_STR_NOGUARD=1")
CFGS=("pgo0|DOTNET_TieredPGO=0" "dflt|X_S14D=1")
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
e1r_median() { awk -F'\t' '$4=="E1R" {print $6}' "$1" | sort -n | awk '{a[NR]=$1} END{print a[int((NR+1)/2)]}'; }
echo "# s14_ladder_sweep.sh invocation $(date -u +%FT%TZ), boot $(uptime -s), commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec || echo ' + uncommitted'); CPUs 0,1; --paths E0,E1R --lengths 40,48 --contents ascii,latin1 --rounds 6; reps $REPS" >> "$OUT/header.txt"
for r in $(seq 1 "$REPS"); do
  for cf in "${CFGS[@]}"; do
    cn="${cf%%|*}"; ce="${cf#*|}"
    if [ $((r % 2)) = 1 ]; then ORD=("${KINDS[@]}"); else ORD=(); for ((i=${#KINDS[@]}-1; i>=0; i--)); do ORD+=("${KINDS[$i]}"); done; fi
    for kd in "${ORD[@]}"; do
      IFS='|' read -r kn kc ke <<< "$kd"; name="$kn-$cn-r$r"; f="$OUT/$name"
      if grep -q "^# $name: rc=0" "$OUT/header.txt" && [ -s "$f.tsv" ]; then continue; fi
      for x in "$f".*; do [ -e "$x" ] && mv "$x" "$OUT/cut/$(basename "$x")-$(date +%s)"; done
      for attempt in 1 2 3 4 5; do
        q=$(quiet); t0=$(date +%s)
        env $ce $ke AK_CORE_LIB="$SLICE/$kc/release/libak_core.so" taskset -c 0,1 dotnet "$BD/BenchDotNet.dll" --strsweep "$f.tsv" --paths E0,E1R --lengths 40,48 --contents ascii,latin1 --rounds 6 > "$f.log" 2>&1; rc=$?
        m=$(e1r_median "$f.tsv")
        if [ "$cn" = dflt ] && [ $rc = 0 ] && awk -v e="$m" 'BEGIN{exit !(e>=300)}' && [ $attempt -lt 5 ]; then
          for x in "$f".*; do mv "$x" "$OUT/cut/$(basename "$x")-slow$attempt"; done
          echo "# $name attempt $attempt: SLOW MODE (E1R median $m ns), set aside as cut/*-slow$attempt and rerun; boot $(uptime -s); $q" >> "$OUT/header.txt"
          continue
        fi
        break
      done
      echo "# $name: rc=$rc $(( $(date +%s) - t0 )) s; core $kc; env $ce $ke; E1R median $m ns; boot $(uptime -s); $q" >> "$OUT/header.txt"
    done
  done
done
echo "# s14_ladder_sweep.sh done $(date -u +%FT%TZ)" >> "$OUT/header.txt"
