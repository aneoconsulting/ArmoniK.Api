#!/usr/bin/env bash
# Narrowed A/B runs for the optimisation pass (container instrumentation, not gated; each
# process's Cases.Verify on). Same BDN settings as gen/opt_bench.sh's codec suite (default
# toolchain, one child per case, warm-up 25 x 40 ms, 6 rounds of 40 ms, client CPUs 0,1, the
# single-CPU guard on, MemoryDiagnoser on), core grid, narrowed by the caller's AK_BDN_ONLY /
# AK_BDN_DIRS / AK_BDN_NO_UNKNOWN. Variants are alternated: rep r runs them in the given order
# when r is odd, reversed when even; each variant runs its full then its no-unknown build
# (BUILDS overrides: "full", "nounk" or "full nounk").
#
#   gen/opt_ab.sh --out DIR --reps N [--first-rep R] NAME=SLICEDIR [NAME=...]
#     SLICEDIR: a checkout of ffi/poc/csharp (this one, or a git worktree's at another commit).
#     BDN's default toolchain REBUILDS each child from the project it finds under the current
#     directory, so each variant is built and run from its own tree (cwd = SLICEDIR); its
#     target-core* directories are this slice's (the core is not changed by the pass).
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="" REPS=2 REP0=1 VARS=()
while [ $# -gt 0 ]; do
  case "$1" in --out) OUT="$2"; shift ;; --reps) REPS="$2"; shift ;; --first-rep) REP0="$2"; shift ;; *=*) VARS+=("$1") ;; *) echo "bad arg $1" >&2; exit 2 ;; esac
  shift
done
[ -n "$OUT" ] && [ ${#VARS[@]} -gt 0 ] || { echo "usage: gen/opt_ab.sh --out DIR --reps N NAME=SLICEDIR ..." >&2; exit 2; }
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
export SCRATCH="${SCRATCH:-$(mktemp -d)}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1; mkdir -p "$SCRATCH"
export AK_CPU_CLIENT="${AK_CPU_CLIENT:-0,1}" AK_CAMPAIGN_GRID=core AK_CAMPAIGN_ALLOC=default AK_BDN_MEMORY=1
unset GLIBC_TUNABLES
BUILDS="${BUILDS:-full nounk}"
{ echo "# opt_ab.sh (narrowed A/B), CONTAINER INSTRUMENTATION, not gated (Cases.Verify on); commit $(git -C "$SLICE" rev-parse --short HEAD)$(git -C "$SLICE" diff --quiet HEAD -- src gen || echo ' + uncommitted changes'); $(date -u +%FT%TZ)"
  echo "# client CPUs $AK_CPU_CLIENT; BDN default toolchain, warm-up 25 x 40 ms, 6 rounds x 40 ms, MemoryDiagnoser on; core grid; AK_BDN_ONLY=${AK_BDN_ONLY:-} AK_BDN_DIRS=${AK_BDN_DIRS:-} AK_BDN_NO_UNKNOWN=${AK_BDN_NO_UNKNOWN:-} AK_BDN_ARMS=${AK_BDN_ARMS:-} AK_BDN_DROP=${AK_BDN_DROP:-}; builds $BUILDS; reps $REPS (order alternated)"
  echo "# cores: target-core $(sha256sum "$SLICE/target-core/release/libak_core.so" | cut -c1-16), target-core-nounk $(sha256sum "$SLICE/target-core-nounk/release/libak_core.so" | cut -c1-16); reps $REP0..$REPS in this invocation"
  for v in "${VARS[@]}"; do d="${v#*=}"; d="${d%%@*}"; echo "# variant ${v%%=*}: $d$( [ "${v#*@}" != "$v" ] && echo " env ${v#*@}") at $(git -C "$d" rev-parse --short HEAD)$(git -C "$d" diff --quiet HEAD -- src gen || echo ' + uncommitted')"; done; } >> "$OUT/header.txt"
for v in "${VARS[@]}"; do
  d="${v#*=}"; d="${d%%@*}"
  for t in target-core target-core-nounk; do [ -e "$d/$t" ] || ln -s "$SLICE/$t" "$d/$t"; done
  for b in $BUILDS; do
    a=""; [ "$b" = nounk ] && a="-p:AkNounk=true"
    ( cd "$d" && dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release $a > "$OUT/build-${v%%=*}-$b.log" 2>&1 ) || { echo "build ${v%%=*} $b failed" >&2; exit 1; }
  done
done
# Before every timed process: the 1-minute load average below 0.5 and no other process above
# 10 % CPU, waited for up to 10 min (recorded per process in header.txt).
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
# --first-rep R: run reps R..REPS only (a long run split over several invocations into one
# directory, so an interruption loses one rep, not the run; the order alternation follows r).
for r in $(seq "$REP0" "$REPS"); do
  if [ $((r % 2)) = 1 ]; then ORD=("${VARS[@]}"); else ORD=(); for ((i=${#VARS[@]}-1; i>=0; i--)); do ORD+=("${VARS[$i]}"); done; fi
  for v in "${ORD[@]}"; do
    name="${v%%=*}"; sd="${v#*=}"; venv=(); [ "${sd#*@}" != "$sd" ] && IFS=, read -r -a venv <<< "${sd#*@}"; sd="${sd%%@*}"
    for b in $BUILDS; do
      if [ "$b" = full ]; then bd="$sd/src/BenchDotNet/bin/Release/net8.0"; core="$SLICE/target-core/release/libak_core.so"; else bd="$sd/src/BenchDotNet/bin-nounk/Release/net8.0"; core="$SLICE/target-core-nounk/release/libak_core.so"; fi
      cp "$core" "$bd/"
      f="$OUT/$name-$b-r$r.jsonl"
      q=$(quiet)
      t0=$(date +%s)
      ( cd "$sd" && exec env "${venv[@]}" taskset -c "$AK_CPU_CLIENT" dotnet "$bd/BenchDotNet.dll" --launch "$r" --out "$f" --artifacts "$SCRATCH/bdn-$name-$b-$r" \
        --rounds 6 --warmup 25 --iteration-ms 40 --toolchain process ) > "$OUT/$name-$b-r$r.bdn.log" 2>&1
      rc=$?
      echo "# rep $r $name $b: rc=$rc $(( $(date +%s) - t0 )) s; $q; $(grep -m1 '^# correctness' "$f" | cut -c1-60)" | tee -a "$OUT/header.txt"
      [ $rc = 0 ] || exit 1
    done
  done
done
