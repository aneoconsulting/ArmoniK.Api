#!/usr/bin/env bash
# s13: the encode grid (encode-core-hot, core-ffi drop and retain, core grid, full build), nine
# variants (E0 / E1R / E1R:128 x simdutf / scalar naive / scalar word cores), 2 reps, resumable
# at CELL granularity (a cell = one variant x one rep = one BDN host process). CONTAINER
# INSTRUMENTATION. The container restarted twice during whole-grid runs (JOURNAL 84), so:
#   * a cell is COMPLETE when header.txt has "# rep R NAME full: rc=0" and its jsonl has EXPECT
#     lines (363: the rows of a complete cell); a complete cell is skipped;
#   * a cell with files but not complete is moved to ../grid-INTERRUPTED/ (suffix -cutN) and rerun;
#   * every cell waits for a quiet machine first (load1 < 0.5, no process above 10 %);
#   * every invocation and every cell's line records the boot time (uptime -s);
#   * after each completed rep the grid directory is committed (git commit -- <paths>).
# Same settings as gen/opt_ab.sh (client CPUs 0,1, warm-up 25 x 40 ms, 6 rounds x 40 ms,
# MemoryDiagnoser, default toolchain, Cases.Verify in every process); the variant's core reaches
# BDN's children through AK_CORE_LIB (src/Harness/CoreLib.cs), recorded per row as core_maps.
#   gen/s13_grid.sh OUTDIR
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$SLICE"
REPO="$(git -C "$SLICE" rev-parse --show-toplevel)"
OUT="$1"; mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"; CUT="$(dirname "$OUT")/grid-INTERRUPTED"; mkdir -p "$CUT"
EXPECT=363
export SCRATCH="${SCRATCH:-$(mktemp -d)}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 MSBUILDDISABLENODEREUSE=1; mkdir -p "$SCRATCH"
export AK_CPU_CLIENT=0,1 AK_CAMPAIGN_GRID=core AK_CAMPAIGN_ALLOC=default AK_BDN_MEMORY=1
export AK_BDN_DIRS=encode-core-hot AK_BDN_ARMS=core-ffi AK_BDN_DROP=1
unset GLIBC_TUNABLES AK_BDN_ONLY AK_BDN_NO_UNKNOWN
VARS=()
for c in "simd:target-core" "naive:target-core-tcnaive" "word:target-core-tcword"; do
  for m in "e0:E0" "e1r:E1R" "e1r128:E1R:128"; do VARS+=("${m%%:*}-${c%%:*}|${m#*:}|${c#*:}"); done
done
BD="$SLICE/src/BenchDotNet/bin/Release/net8.0"
dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release > "$OUT/build-s13grid-$(date -u +%H%M%S).log" 2>&1 || { echo "build failed" >&2; exit 1; }
echo "# s13_grid.sh invocation $(date -u +%FT%TZ), boot $(uptime -s), commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec || echo ' + uncommitted'); client CPUs 0,1; AK_BDN_DIRS=$AK_BDN_DIRS AK_BDN_ARMS=$AK_BDN_ARMS AK_BDN_DROP=1; complete = rc=0 and $EXPECT jsonl lines" >> "$OUT/header.txt"
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
complete() {  # rep name
  grep -q "^# rep $1 $2 full: rc=0" "$OUT/header.txt" && [ -f "$OUT/$2-full-r$1.jsonl" ] && [ "$(wc -l < "$OUT/$2-full-r$1.jsonl")" -eq $EXPECT ]
}
for r in 1 2; do
  if [ $((r % 2)) = 1 ]; then ORD=("${VARS[@]}"); else ORD=(); for ((i=${#VARS[@]}-1; i>=0; i--)); do ORD+=("${VARS[$i]}"); done; fi
  for v in "${ORD[@]}"; do
    IFS='|' read -r name enc coredir <<< "$v"
    if complete $r "$name"; then echo "# rep $r $name: complete before this invocation, skipped" >> "$OUT/header.txt"; continue; fi
    for f in "$OUT/$name-full-r$r".*; do
      [ -e "$f" ] || continue
      k=2; while [ -e "$CUT/$(basename "$f")-cut$k" ]; do k=$((k+1)); done   # cut 1 = the first whole-grid run (grid-INTERRUPTED/*.jsonl)
      mv "$f" "$CUT/$(basename "$f")-cut$k"; echo "# rep $r $name: partial file $(basename "$f") moved to grid-INTERRUPTED/ as -cut$k" >> "$OUT/header.txt"
    done
    core="$SLICE/$coredir/release/libak_core.so"
    cp "$SLICE/target-core/release/libak_core.so" "$BD/"
    q=$(quiet); t0=$(date +%s)
    ( exec env AK_STR_ENC="$enc" AK_AB_CORE="$coredir" AK_CORE_LIB="$core" taskset -c 0,1 dotnet "$BD/BenchDotNet.dll" --launch "$r" --out "$OUT/$name-full-r$r.jsonl" --artifacts "$SCRATCH/bdn-$name-$r" \
      --rounds 6 --warmup 25 --iteration-ms 40 --toolchain process ) > "$OUT/$name-full-r$r.bdn.log" 2>&1
    rc=$?
    echo "# rep $r $name full: rc=$rc $(( $(date +%s) - t0 )) s; core $(sha256sum "$core" | cut -c1-16) (AK_CORE_LIB); boot $(uptime -s); $q; $(grep -m1 '^# correctness' "$OUT/$name-full-r$r.jsonl" | cut -c1-60)" >> "$OUT/header.txt"
    [ $rc = 0 ] || { echo "cell $name rep $r failed rc=$rc" >&2; exit 1; }
  done
  git -C "$REPO" add "$OUT" "$CUT" && git -C "$REPO" commit -q -m "poc(csharp): s13 encode grid, rep $r complete (gen/s13_grid.sh, cell-resumable)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01LNKfW5iBRokcmRRbcZMcZT" -- "$OUT" "$CUT" && echo "# rep $r committed $(git -C "$REPO" rev-parse --short HEAD)" >> "$OUT/header.txt"
done
echo "# s13_grid.sh done $(date -u +%FT%TZ)" >> "$OUT/header.txt"
