#!/usr/bin/env bash
# s15 measurements 2 and 3 (D26: no thread-local storage), CONTAINER INSTRUMENTATION, each with
# the code before D26 (BASE, a worktree at the commit before s15) in the same session:
#   S15_SET=ladder: the s14 ladder rows (P2.2 three content sets, P2.4, U-deep-u-repeated;
#     encode-core-hot, core-ffi, drop and retain), R0 (E0), R3 (E1R), R3g (E1R, AK_STR_NOGUARD=1)
#     on the D26 code, R0 and R3 on BASE; under DOTNET_TieredPGO=0 and the default JIT configuration;
#   S15_SET=codec: the core-ffi codec grid (the core grid's directions, encode-transport-hot and
#     decode-read, retain and drop; E0 and the FSM as today), the D26 code and BASE, default JIT
#     configuration.
# Resumable at CELL granularity as gen/s14_grid.sh (a cell = one variant x one rep = one BDN host
# process; complete = its rc=0 line in header.txt and a non-empty jsonl; a partial cell moved to
# ../<set>-INTERRUPTED/ as -cutN), a quiet machine before each cell, the boot time on each line,
# a commit after each rep. Each variant runs FROM ITS OWN TREE (cwd), because BDN's default
# toolchain rebuilds every child from the project under the current directory; the core is this
# slice's target-core for both (AK_CORE_LIB, recorded per row as core_maps).
#   S15_SET=ladder|codec [REPS=3] gen/s15_grid.sh OUTDIR BASE_SLICE_DIR
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$SLICE"
REPO="$(git -C "$SLICE" rev-parse --show-toplevel)"
OUT="$1"; BASE="$(cd "$2" && pwd)"; SET="${S15_SET:?S15_SET=ladder|codec}"
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"; CUT="$(dirname "$OUT")/$SET-INTERRUPTED"; mkdir -p "$CUT"
[ -e "$CUT/NOTE.txt" ] || echo "s15 $SET grid: cells cut by a container restart are moved here by gen/s15_grid.sh (suffix -cutN); empty when none was." > "$CUT/NOTE.txt"
export SCRATCH="${SCRATCH:-$(mktemp -d)}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 MSBUILDDISABLENODEREUSE=1; mkdir -p "$SCRATCH"
export AK_CPU_CLIENT=0,1 AK_CAMPAIGN_GRID=core AK_CAMPAIGN_ALLOC=default AK_BDN_MEMORY=1
export AK_BDN_ARMS=core-ffi AK_BDN_DROP=1
unset GLIBC_TUNABLES AK_BDN_NO_UNKNOWN AK_BDN_DIRS AK_BDN_ONLY
VARS=()   # name|tree|AK_STR_ENC|extra env (one K=V or X=1, or two)
if [ "$SET" = ladder ]; then
  export AK_BDN_DIRS=encode-core-hot AK_BDN_ONLY=P2.2,P2.4,U-deep-u-repeated
  for cfg in "pgo0|DOTNET_TieredPGO=0" "dflt|X_S15=1"; do
    c="${cfg%%|*}"; e="${cfg#*|}"
    VARS+=("new-r0-$c|$SLICE|E0|$e" "new-r3-$c|$SLICE|E1R|$e" "new-r3g-$c|$SLICE|E1R|$e AK_STR_NOGUARD=1" "base-r0-$c|$BASE|E0|$e" "base-r3-$c|$BASE|E1R|$e")
  done
else
  VARS=("new|$SLICE|E0|X_S15=1" "base|$BASE|E0|X_S15=1")
fi
for tt in "new|$SLICE" "base|$BASE"; do
  ( cd "${tt#*|}" && dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release ) > "$OUT/build-${tt%%|*}-$(date -u +%H%M%S).log" 2>&1 || { echo "build ${tt%%|*} failed" >&2; exit 1; }
done
echo "# s15_grid.sh ($SET) invocation $(date -u +%FT%TZ), boot $(uptime -s); new $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec || echo ' + uncommitted'); base $BASE at $(git -C "$BASE" rev-parse --short HEAD); client CPUs 0,1; AK_BDN_DIRS=${AK_BDN_DIRS:-(core grid)} AK_BDN_ONLY=${AK_BDN_ONLY:-} AK_BDN_ARMS=$AK_BDN_ARMS AK_BDN_DROP=1; core $(sha256sum "$SLICE/target-core/release/libak_core.so" | cut -c1-16) for both; complete = rc=0 line in header.txt and a jsonl" >> "$OUT/header.txt"
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
for r in $(seq 1 "${REPS:-3}"); do
  if [ $((r % 2)) = 1 ]; then ORD=("${VARS[@]}"); else ORD=(); for ((i=${#VARS[@]}-1; i>=0; i--)); do ORD+=("${VARS[$i]}"); done; fi
  for v in "${ORD[@]}"; do
    IFS='|' read -r name tree enc xenv <<< "$v"
    if complete $r "$name"; then echo "# rep $r $name: complete before this invocation, skipped" >> "$OUT/header.txt"; continue; fi
    for f in "$OUT/$name-full-r$r".*; do
      [ -e "$f" ] || continue
      k=2; while [ -e "$CUT/$(basename "$f")-cut$k" ]; do k=$((k+1)); done   # cut numbering from 2, as gen/s14_grid.sh
      mv "$f" "$CUT/$(basename "$f")-cut$k"; echo "# rep $r $name: partial file $(basename "$f") moved to $SET-INTERRUPTED/ as -cut$k" >> "$OUT/header.txt"
    done
    core="$SLICE/target-core/release/libak_core.so"; BD="$tree/src/BenchDotNet/bin/Release/net8.0"
    cp "$core" "$BD/"
    q=$(quiet); t0=$(date +%s)
    ( cd "$tree" && exec env $xenv AK_STR_ENC="$enc" AK_AB_CORE=target-core AK_CORE_LIB="$core" taskset -c 0,1 dotnet "$BD/BenchDotNet.dll" --launch "$r" --out "$OUT/$name-full-r$r.jsonl" --artifacts "$SCRATCH/bdn-$name-$r" \
      --rounds 6 --warmup 25 --iteration-ms 40 --toolchain process ) > "$OUT/$name-full-r$r.bdn.log" 2>&1
    rc=$?
    echo "# rep $r $name full: rc=$rc $(( $(date +%s) - t0 )) s; tree $tree; core $(sha256sum "$core" | cut -c1-16) (AK_CORE_LIB); env $xenv; boot $(uptime -s); $q; $(grep -m1 '^# correctness' "$OUT/$name-full-r$r.jsonl" | cut -c1-60)" >> "$OUT/header.txt"
    [ $rc = 0 ] || { echo "cell $name rep $r failed rc=$rc" >&2; exit 1; }
  done
  git -C "$REPO" add "$OUT" "$CUT" && git -C "$REPO" commit -q -m "poc(csharp): s15 $SET grid, rep $r complete (gen/s15_grid.sh, cell-resumable)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01LNKfW5iBRokcmRRbcZMcZT" -- "$OUT" "$CUT" && echo "# rep $r committed $(git -C "$REPO" rev-parse --short HEAD)" >> "$OUT/header.txt"
done
echo "# s15_grid.sh ($SET) done $(date -u +%FT%TZ)" >> "$OUT/header.txt"
