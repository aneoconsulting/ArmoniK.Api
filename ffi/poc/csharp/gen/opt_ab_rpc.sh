#!/usr/bin/env bash
# Narrowed RPC A/B for the optimisation pass (container instrumentation, not gated; every call
# checked). One server (poc/rust/serve.sh, CPUs AK_CPU_SERVER, warm-up OPT_SERVER_WARM), the
# RPC settings of gen/opt_bench.sh (BDN default toolchain, warm-up 10 x 100 ms, 6 rounds x
# 100 ms, client CPUs 0,1, MemoryDiagnoser on, transport armonik, stock h2, full build),
# narrowed by UNITS (e.g. Cf-retain), AK_RPC_ONLY_DIRS and INFLIGHT. Variants alternated by rep.
#   gen/opt_ab_rpc.sh --out DIR --reps N NAME=SLICEDIR [NAME=...]
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="$(git -C "$SLICE" rev-parse --show-toplevel)"
OUT="" REPS=2 VARS=()
while [ $# -gt 0 ]; do
  case "$1" in --out) OUT="$2"; shift ;; --reps) REPS="$2"; shift ;; *=*) VARS+=("$1") ;; *) echo "bad arg $1" >&2; exit 2 ;; esac
  shift
done
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
export SCRATCH="${SCRATCH:-$(mktemp -d)}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1; mkdir -p "$SCRATCH"
export AK_CPU_CLIENT="${AK_CPU_CLIENT:-0,1}" AK_CPU_SERVER="${AK_CPU_SERVER:-2,3}" AK_WORKERS="${AK_WORKERS:-8}"
export AK_SERVER_THREADS="${AK_SERVER_THREADS:-$AK_WORKERS}" AK_CAMPAIGN_GRID=core AK_CAMPAIGN_ALLOC=default AK_BDN_MEMORY=1 AK_H2=stock
unset GLIBC_TUNABLES
UNITS="${UNITS:-Cf-retain}" INFLIGHT="${INFLIGHT:-1,8}"
{ echo "# opt_ab_rpc.sh (narrowed RPC A/B), CONTAINER INSTRUMENTATION, not gated (every call checked); $(date -u +%FT%TZ)"
  echo "# client CPUs $AK_CPU_CLIENT, server $AK_CPU_SERVER; BDN default toolchain, warm-up 10 x 100 ms, 6 rounds x 100 ms, MemoryDiagnoser on; transport armonik, stock h2, full build; units $UNITS, AK_RPC_ONLY_DIRS=${AK_RPC_ONLY_DIRS:-} inflight $INFLIGHT; reps $REPS (order alternated)"
  for v in "${VARS[@]}"; do d="${v#*=}"; d="${d%%@*}"; echo "# variant ${v%%=*}: $d$( [ "${v#*@}" != "$v" ] && echo " env ${v#*@}") at $(git -C "$d" rev-parse --short HEAD)$(git -C "$d" diff --quiet HEAD -- src gen || echo ' + uncommitted')"; done; } >> "$OUT/header.txt"
for v in "${VARS[@]}"; do
  d="${v#*=}"; d="${d%%@*}"
  for t in target-core target-core-nounk target-armonik-client; do [ -e "$d/$t" ] || ln -s "$SLICE/$t" "$d/$t"; done
  ( cd "$d" && dotnet build src/Rpc/akrpc.csproj -c Release > "$OUT/build-${v%%=*}.log" 2>&1 ) || { echo "build ${v%%=*} failed" >&2; exit 1; }
  cp "$SLICE/target-core/release/libak_core.so" "$d/src/Rpc/bin/Release/net8.0/"
done
SERVE="$REPO/ffi/poc/rust/serve.sh"; export AK_SERVE_STATE="$SCRATCH/srv.state"
"$SERVE" build > /dev/null 2>&1 || exit 1
env -u GLIBC_TUNABLES AK_SERVER_TCP=0 "$SERVE" start --out "$SCRATCH/srv" > "$SCRATCH/srv.start" 2>&1 || { cat "$SCRATCH/srv.start"; exit 1; }
trap '"$SERVE" stop > /dev/null 2>&1' EXIT
TL=$(sed -n 's/^tcp //p' "$SCRATCH/srv.start")
"$SERVE" warm "${OPT_SERVER_WARM:-500}" > "$OUT/server-warm.log" 2>&1 || exit 1
# Before every timed process: the 1-minute load average below 0.5 and no process other than the
# server above 10 % CPU (ps: CPU over the process's life), waited for up to 10 min (recorded per
# process in header.txt). The server is excluded: it is idle between processes, but its lifetime
# share stays high after the warm-up; load1 still sees it if it is not idle.
SPID=$(sed -n 's/^pid //p' "$AK_SERVE_STATE")
quiet() {
  local la hot
  for i in $(seq 1 120); do
    la=$(cut -d' ' -f1 /proc/loadavg)
    hot=$(ps -eo pid,pcpu,comm --sort=-pcpu --no-headers | awk -v sp="$SPID" '$1 != sp && $2 > 10 && $3 != "ps" {print $3"("$2"%)"}' | head -3 | tr '\n' ' ')
    if awk -v l="$la" 'BEGIN{exit !(l < 0.5)}' && [ -z "$hot" ]; then echo "quiet: load1 $la, no process but the server above 10 % (waited $((i * 5 - 5)) s)"; return 0; fi
    sleep 5
  done
  echo "NOT QUIET after 10 min: load1 $la, hot: $hot"
}
for r in $(seq 1 "$REPS"); do
  if [ $((r % 2)) = 1 ]; then ORD=("${VARS[@]}"); else ORD=(); for ((i=${#VARS[@]}-1; i>=0; i--)); do ORD+=("${VARS[$i]}"); done; fi
  for v in "${ORD[@]}"; do
    name="${v%%=*}"; sd="${v#*=}"; venv=(); [ "${sd#*@}" != "$sd" ] && IFS=, read -r -a venv <<< "${sd#*@}"; sd="${sd%%@*}"; f="$OUT/$name-r$r.jsonl"; q=$(quiet); t0=$(date +%s)
    ( cd "$sd" && exec env "${venv[@]}" taskset -c "$AK_CPU_CLIENT" dotnet "$sd/src/Rpc/bin/Release/net8.0/akrpc.dll" bench --sock "tcp:$TL" --transport armonik --unit "$UNITS" \
        --launch "$r" --out "$f" --toolchain process --rounds 6 --warmup 10 --iteration-ms 100 --artifacts "$SCRATCH/bdn-$name-$r" --inflight "$INFLIGHT" ) > "$OUT/$name-r$r.bdn.log" 2>&1
    rc=$?
    echo "# rep $r $name: rc=$rc $(( $(date +%s) - t0 )) s, $(grep -c '^{' "$f") rows; $q" | tee -a "$OUT/header.txt"
    [ $rc = 0 ] || exit 1
  done
done
