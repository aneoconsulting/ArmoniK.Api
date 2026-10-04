#!/usr/bin/env bash
# Narrowed A/B runs for the optimisation pass (container instrumentation, not gated; each
# process's Cases.Verify on). Same BDN settings as gen/opt_bench.sh's codec suite (default
# toolchain, one child per case, warm-up 25 x 40 ms, 6 rounds of 40 ms, client CPUs 0,1, the
# single-CPU guard on, MemoryDiagnoser on), core grid, narrowed by the caller's AK_BDN_ONLY /
# AK_BDN_DIRS / AK_BDN_NO_UNKNOWN. Variants are alternated: rep r runs them in the given order
# when r is odd, reversed when even; each variant runs its full then its no-unknown build
# (BUILDS overrides: "full", "nounk" or "full nounk").
#
#   gen/opt_ab.sh --out DIR --reps N NAME=SLICEDIR [NAME=...]
#     SLICEDIR: a checkout of ffi/poc/csharp (this one, or a git worktree's at another commit).
#     BDN's default toolchain REBUILDS each child from the project it finds under the current
#     directory, so each variant is built and run from its own tree (cwd = SLICEDIR); its
#     target-core* directories are this slice's (the core is not changed by the pass).
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="" REPS=2 VARS=()
while [ $# -gt 0 ]; do
  case "$1" in --out) OUT="$2"; shift ;; --reps) REPS="$2"; shift ;; *=*) VARS+=("$1") ;; *) echo "bad arg $1" >&2; exit 2 ;; esac
  shift
done
[ -n "$OUT" ] && [ ${#VARS[@]} -gt 0 ] || { echo "usage: gen/opt_ab.sh --out DIR --reps N NAME=SLICEDIR ..." >&2; exit 2; }
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
export SCRATCH="${SCRATCH:-$(mktemp -d)}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
export AK_CPU_CLIENT="${AK_CPU_CLIENT:-0,1}" AK_CAMPAIGN_GRID=core AK_CAMPAIGN_ALLOC=default AK_BDN_MEMORY=1
unset GLIBC_TUNABLES
BUILDS="${BUILDS:-full nounk}"
{ echo "# opt_ab.sh (narrowed A/B), CONTAINER INSTRUMENTATION, not gated (Cases.Verify on); commit $(git -C "$SLICE" rev-parse --short HEAD)$(git -C "$SLICE" diff --quiet HEAD -- src gen || echo ' + uncommitted changes'); $(date -u +%FT%TZ)"
  echo "# client CPUs $AK_CPU_CLIENT; BDN default toolchain, warm-up 25 x 40 ms, 6 rounds x 40 ms, MemoryDiagnoser on; core grid; AK_BDN_ONLY=${AK_BDN_ONLY:-} AK_BDN_DIRS=${AK_BDN_DIRS:-} AK_BDN_NO_UNKNOWN=${AK_BDN_NO_UNKNOWN:-}; builds $BUILDS; reps $REPS (order alternated)"
  for v in "${VARS[@]}"; do d="${v#*=}"; echo "# variant ${v%%=*}: $d at $(git -C "$d" rev-parse --short HEAD)$(git -C "$d" diff --quiet HEAD -- src gen || echo ' + uncommitted')"; done; } >> "$OUT/header.txt"
for v in "${VARS[@]}"; do
  d="${v#*=}"
  for t in target-core target-core-nounk; do [ -e "$d/$t" ] || ln -s "$SLICE/$t" "$d/$t"; done
  for b in $BUILDS; do
    a=""; [ "$b" = nounk ] && a="-p:AkNounk=true"
    ( cd "$d" && dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release $a > "$OUT/build-${v%%=*}-$b.log" 2>&1 ) || { echo "build ${v%%=*} $b failed" >&2; exit 1; }
  done
done
for r in $(seq 1 "$REPS"); do
  if [ $((r % 2)) = 1 ]; then ORD=("${VARS[@]}"); else ORD=(); for ((i=${#VARS[@]}-1; i>=0; i--)); do ORD+=("${VARS[$i]}"); done; fi
  for v in "${ORD[@]}"; do
    name="${v%%=*}"; sd="${v#*=}"
    for b in $BUILDS; do
      if [ "$b" = full ]; then bd="$sd/src/BenchDotNet/bin/Release/net8.0"; core="$SLICE/target-core/release/libak_core.so"; else bd="$sd/src/BenchDotNet/bin-nounk/Release/net8.0"; core="$SLICE/target-core-nounk/release/libak_core.so"; fi
      cp "$core" "$bd/"
      f="$OUT/$name-$b-r$r.jsonl"
      t0=$(date +%s)
      ( cd "$sd" && exec taskset -c "$AK_CPU_CLIENT" dotnet "$bd/BenchDotNet.dll" --launch "$r" --out "$f" --artifacts "$SCRATCH/bdn-$name-$b-$r" \
        --rounds 6 --warmup 25 --iteration-ms 40 --toolchain process ) > "$OUT/$name-$b-r$r.bdn.log" 2>&1
      rc=$?
      echo "# rep $r $name $b: rc=$rc $(( $(date +%s) - t0 )) s; $(grep -m1 '^# correctness' "$f" | cut -c1-60)" | tee -a "$OUT/header.txt"
      [ $rc = 0 ] || exit 1
    done
  done
done
