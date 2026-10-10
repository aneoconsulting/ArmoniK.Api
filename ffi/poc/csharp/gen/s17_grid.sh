#!/usr/bin/env bash
# s17 part 2 (owner, 2026-10-10): today's (eager) encode render against the lazy nested render
# (cs_host lazy_nested=True, /p:AkLazy=true), in the same session, CONTAINER INSTRUMENTATION.
# The core-ffi encode rows of the FULL grid (AK_CAMPAIGN_GRID unset: every payload, content set
# and U-* row), encode-core-hot and encode-transport, drop and retain, under E0 and E1R:128:
#   S17_SET=full  [REPS=3]   every row
#   S17_SET=small [REPS=8]   the small rows only (P1.1, P5.1, P7.1 and the core grid's U-* rows)
# A cell = one variant (render x path) x one rep = one BDN host process (BDN's default toolchain,
# one child per case; each child rebuilt with the host's MSBuild properties and checked to be
# the same build and render: CpuClock.BuildCheck). Variants interleaved, the order reversed on
# even reps. Resumable at cell granularity (complete = its rc=0 line in header.txt and a
# non-empty jsonl; a partial cell moved to ../<set>-INTERRUPTED/ as -cutN), a quiet machine
# before each cell (load1 < 0.5, no process above 10 %), client CPUs 0,1, a commit per rep.
#   S17_SET=full|small gen/s17_grid.sh OUTDIR
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$SLICE"
REPO="$(git -C "$SLICE" rev-parse --show-toplevel)"
OUT="$1"; SET="${S17_SET:?S17_SET=full|small}"
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"; CUT="$(dirname "$OUT")/$SET-INTERRUPTED"; mkdir -p "$CUT"
[ -e "$CUT/NOTE.txt" ] || echo "s17 $SET grid: cells cut by a container restart are moved here by gen/s17_grid.sh (suffix -cutN); empty when none was." > "$CUT/NOTE.txt"
export SCRATCH="${SCRATCH:-$(mktemp -d)}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 MSBUILDDISABLENODEREUSE=1; mkdir -p "$SCRATCH"
export AK_CPU_CLIENT=0,1 AK_CAMPAIGN_ALLOC=default AK_BDN_MEMORY=1
export AK_BDN_ARMS=core-ffi AK_BDN_DROP=1 AK_BDN_DIRS=encode-core-hot,encode-transport
unset GLIBC_TUNABLES AK_BDN_NO_UNKNOWN AK_BDN_ONLY AK_CAMPAIGN_GRID AK_BDN_BUILD AK_BDN_RENDER
SMALL="P1.1,P5.1,P7.1,U-nested-before,U-deep-u-repeated,U-oneof-u-repeated,U-wire-ListTaskSummaryResponse-tasks-as-wt5,U-wire-UploadResultDataMessage-upload-as-wt5,U-wire-ListMetricsResponse-batches-as-wt0,U-wire-DualResponse-left-as-wt5"
[ "$SET" = small ] && export AK_BDN_ONLY="$SMALL"
DEFREPS=3; [ "$SET" = small ] && DEFREPS=8
B_E="$SLICE/src/BenchDotNet/bin/Release/net8.0"; B_L="$SLICE/src/BenchDotNet/bin-lazy/Release/net8.0"
VARS=("eager-e0|$B_E|E0" "lazy-e0|$B_L|E0" "eager-e1r128|$B_E|E1R:128" "lazy-e1r128|$B_L|E1R:128")   # name|bin|AK_STR_ENC
dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release > "$OUT/build-eager-$(date -u +%H%M%S).log" 2>&1 || { echo "build eager failed" >&2; exit 1; }
dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkLazy=true > "$OUT/build-lazy-$(date -u +%H%M%S).log" 2>&1 || { echo "build lazy failed" >&2; exit 1; }
CORE="$SLICE/target-core/release/libak_core.so"
echo "# s17_grid.sh ($SET) invocation $(date -u +%FT%TZ), boot $(uptime -s); $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec/gen || echo ' + uncommitted'); client CPUs 0,1; full grid (AK_CAMPAIGN_GRID unset); AK_BDN_DIRS=$AK_BDN_DIRS AK_BDN_ARMS=$AK_BDN_ARMS AK_BDN_ONLY=${AK_BDN_ONLY:-} ; core $(sha256sum "$CORE" | cut -c1-16) (target-core, AK_CORE_LIB) for every cell; eager = bin/, lazy = bin-lazy/ (/p:AkLazy=true); complete = rc=0 line in header.txt and a jsonl" >> "$OUT/header.txt"
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
  grep -q "^# rep $1 $2 full: rc=0" "$OUT/header.txt" && [ -s "$OUT/$2-full-r$1.jsonl" ]
}
for r in $(seq 1 "${REPS:-$DEFREPS}"); do
  if [ $((r % 2)) = 1 ]; then ORD=("${VARS[@]}"); else ORD=(); for ((i=${#VARS[@]}-1; i>=0; i--)); do ORD+=("${VARS[$i]}"); done; fi
  for v in "${ORD[@]}"; do
    IFS='|' read -r name BD enc <<< "$v"
    if complete $r "$name"; then echo "# rep $r $name: complete before this invocation, skipped" >> "$OUT/header.txt"; continue; fi
    for f in "$OUT/$name-full-r$r".*; do
      [ -e "$f" ] || continue
      k=2; while [ -e "$CUT/$(basename "$f")-cut$k" ]; do k=$((k+1)); done
      mv "$f" "$CUT/$(basename "$f")-cut$k"; echo "# rep $r $name: partial file $(basename "$f") moved to $SET-INTERRUPTED/ as -cut$k" >> "$OUT/header.txt"
    done
    cp "$CORE" "$BD/"
    q=$(quiet); t0=$(date +%s)
    ( exec env AK_STR_ENC="$enc" AK_AB_CORE=target-core AK_CORE_LIB="$CORE" taskset -c 0,1 dotnet "$BD/BenchDotNet.dll" --launch "$r" --out "$OUT/$name-full-r$r.jsonl" --artifacts "$SCRATCH/bdn-$name-$r" \
      --rounds 6 --warmup 25 --iteration-ms 40 --toolchain process ) > "$OUT/$name-full-r$r.bdn.log" 2>&1
    rc=$?
    echo "# rep $r $name full: rc=$rc $(( $(date +%s) - t0 )) s; bin $(echo "$BD" | sed "s|$SLICE/||"); AK_STR_ENC=$enc; core $(sha256sum "$CORE" | cut -c1-16) (AK_CORE_LIB); boot $(uptime -s); $q; $(grep -m1 '^# build' "$OUT/$name-full-r$r.jsonl" | cut -c1-200)" >> "$OUT/header.txt"
    [ $rc = 0 ] || { echo "cell $name rep $r failed rc=$rc" >&2; exit 1; }
  done
  git -C "$REPO" add "$OUT" "$CUT" && git -C "$REPO" commit -q -m "poc(csharp): s17 $SET grid (eager vs lazy nested encode render), rep $r complete (gen/s17_grid.sh, cell-resumable)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01LNKfW5iBRokcmRRbcZMcZT" -- "$OUT" "$CUT" && echo "# rep $r committed $(git -C "$REPO" rev-parse --short HEAD)" >> "$OUT/header.txt"
done
echo "# s17_grid.sh ($SET) done $(date -u +%FT%TZ)" >> "$OUT/header.txt"
