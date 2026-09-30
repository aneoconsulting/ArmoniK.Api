#!/usr/bin/env bash
# The stability campaign (owner, 2026-09-30): do the main results hold over a long run? Many
# one-cell `campaign_rpc --profile` processes, the current core and the patch stack interleaved,
# no perf attached, under /tmp/ak-physical-bench.lock (held for the whole campaign), with an own
# 8-worker server (poc/rust/serve.sh at HEAD), the allocator pinned, grpc-core sized for 8 CPUs.
#
#   gen/stability.sh OUT_DIR STACK_CORE_DIR [ROUNDS]
#
#   units (arm/cell, knobs):
#     cur/A, cur/D-retain, cur/Cf-retain, cur/Cf-q-retain           this tree's core (HEAD)
#     stk/Cf-retain (AK_SPARES=6 AK_SPARE_LOCK=1), stk/Cf-q-retain (AK_SPARES=24 AK_SPARE_LOCK=1),
#     stk/Cf-zc-retain and stk/Cf-zcw-retain (6 + lock; direction d only)   STACK_CORE_DIR's core
#   workloads: d/16MiB k=1 and 8, d/4MiB k=1, c/P5.4 k=1 and 8
#   order: ROUNDS rounds (default 14); per round and workload every unit once, the unit order
#          rotated by round and workload; one process per unit
#   every process: GLIBC_TUNABLES trim 256 MiB / mmap 32 MiB, core --workers 8, ncpus_shim 8,
#          AK_SERVER_PID (the server's threads sampled around the loop)
#   header: runner.log, machine facts (gen/machine_facts.py: CPU, frequency, isolation and
#          cgroup/IRQ confinement) at the start, after every round and at the end
# Tables: gen/stability_tables.py OUT_DIR.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
FFI=$(cd "$HERE/../.." && pwd)
OUT=${1:?usage: gen/stability.sh OUT_DIR STACK_CORE_DIR [ROUNDS]}; STK=${2:?STACK_CORE_DIR}; ROUNDS=${3:-14}
mkdir -p "$OUT/proc"; OUT=$(cd "$OUT" && pwd); STK=$(cd "$STK" && pwd)
if [ "${ST_LOCKED:-}" != 1 ]; then
  t=$(date +%s); ST_LOCKED=1 flock /tmp/ak-physical-bench.lock bash "$0" "$@"; rc=$?
  echo "stability: $(( $(date +%s) - t )) s (lock held), exit $rc" | tee -a "$OUT/runner.log"; exit $rc
fi
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1-4,11-14} AK_CPU_SERVER=${AK_CPU_SERVER:-5-8,15-18}
OSSET=${AK_CPU_OS:-0,9,10,19}; WK=8; GCPUS=8
EXE=$HERE/build-campaign/campaign_rpc; SERVE=$FFI/poc/rust/serve.sh
CUR=$(dirname "$(ldd "$EXE" | grep -o '/[^ ]*libak_core\.so')")
ENVX="GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432"
LOG=$OUT/runner.log
say() { echo "$*" | tee -a "$LOG"; }
UNITS=("cur|A|" "cur|D-retain|" "cur|Cf-retain|" "cur|Cf-q-retain|"
       "stk|Cf-retain|AK_SPARES=6 AK_SPARE_LOCK=1" "stk|Cf-q-retain|AK_SPARES=24 AK_SPARE_LOCK=1"
       "stk|Cf-zc-retain|AK_SPARES=6 AK_SPARE_LOCK=1" "stk|Cf-zcw-retain|AK_SPARES=6 AK_SPARE_LOCK=1")
WLS=("d16k1 d 16MiB 1 120" "d16k8 d 16MiB 8 20" "d4k1 d 4MiB 1 400" "c54k1 c P5.4 1 400" "c54k8 c P5.4 8 50")
SCR=$(mktemp -d)
trap '[ -f "$SCR/serve.state" ] && bash "$SERVE" stop > /dev/null 2>&1; rm -rf "$SCR"' EXIT
taskset -c "$OSSET" gcc -O2 -shared -fPIC -o "$SCR/ncpus.so" gen/ncpus_shim.c -ldl || exit 1
export AK_SERVE_STATE=$SCR/serve.state
AK_SERVER_THREADS=$WK bash "$SERVE" start --out "$SCR/srv" > "$SCR/srv.out" 2>&1 || { cat "$SCR/srv.out"; exit 1; }
bash "$SERVE" warm 64 > /dev/null 2>&1 || exit 1
SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE"); SPID=$(sed -n 's/^pid //p' "$AK_SERVE_STATE")
facts() { python3 gen/machine_facts.py "$AK_CPU_CLIENT" "$AK_CPU_SERVER" "$SPID"; }
{ echo "# stability: commit $(git -C "$FFI" rev-parse --short HEAD)$(git -C "$FFI" status --porcelain -- poc/cpp/src poc/cpp/gen poc/codec | grep -q . && echo ' + UNCOMMITTED'), $(date -u +%FT%TZ), rounds $ROUNDS"
  echo "# exe $EXE sha256 $(sha256sum "$EXE" | cut -c1-16)"
  echo "# arm cur: $CUR/libak_core.so sha256 $(sha256sum "$CUR/libak_core.so" | cut -c1-16); arm stk: $STK/libak_core.so sha256 $(sha256sum "$STK/libak_core.so" | cut -c1-16) (LD_LIBRARY_PATH)"
  echo "# units: ${UNITS[*]}"
  echo "# workloads: ${WLS[*]}; every process: $ENVX, core --workers $WK, grpc-core sysconf $GCPUS (ncpus_shim), pinned transport, retain"
  echo "# server pid $SPID: $(head -2 "$SCR/srv/rpc-server.log" | tail -1 | sed 's/.*tokio/tokio/'); CLIENT $AK_CPU_CLIENT SERVER $AK_CPU_SERVER"
  echo "# machine_start $(facts)"; } >> "$LOG"
T0=$(date +%s); N=${#UNITS[@]}; seq_no=0
for r in $(seq 1 "$ROUNDS"); do
  wi=0
  for w in "${WLS[@]}"; do
    set -- $w; name=$1; dirs=$2; pay=$3; k=$4; n=$5
    for i in $(seq 0 $((N - 1))); do
      u=${UNITS[$(( (i + r + wi) % N ))]}
      arm=${u%%|*}; rest=${u#*|}; cell=${rest%%|*}; knobs=${rest#*|}
      case "$cell" in *-zc*) [ "$dirs" != d ] && continue ;; esac
      lib=$CUR; [ "$arm" = stk ] && lib=$STK
      # the server must still be the one started here, pinned to AK_CPU_SERVER (a re-pinning of user
      # processes during the campaign moved it once: logs/cpp/opt/physical-probe/stability, round 10)
      sa=$(sed -n 's/^Cpus_allowed_list:[[:space:]]*//p' /proc/$SPID/status 2>/dev/null)
      if [ "$(python3 -c "import sys
def r(s):
    o=set()
    for p in s.split(','):
        if '-' in p: a,b=p.split('-'); o.update(range(int(a),int(b)+1))
        elif p: o.add(int(p))
    return o
print(int(r(sys.argv[1])==r(sys.argv[2])))" "$sa" "$AK_CPU_SERVER")" != 1 ]; then
        say "ABORTED before process $((seq_no + 1)): the server's affinity is '$sa', not AK_CPU_SERVER $AK_CPU_SERVER"; exit 3
      fi
      seq_no=$((seq_no + 1))
      f=$OUT/proc/$(printf '%04d' $seq_no)-r$r-$name-$arm-$cell.out
      taskset -c "$AK_CPU_CLIENT" env LD_LIBRARY_PATH="$lib" $knobs $ENVX AK_SERVER_PID="$SPID" LD_PRELOAD="$SCR/ncpus.so" \
        AK_SHIM_NCPUS=$GCPUS "$EXE" --target "unix:$SOCK" --expect 540422 --transport pinned --cells "$cell" --dirs "$dirs" \
        --payloads "$pay" --inflight "$k" --workers $WK --profile "$n" --profile-chunks 10 > "$f" 2>&1 \
        || { tail -3 "$f"; say "FAILED $f"; exit 1; }
    done
    wi=$((wi + 1))
  done
  echo "# machine_after_round $r $(date -u +%FT%TZ) $(facts)" >> "$LOG"
  say "  round $r done, $(( $(date +%s) - T0 )) s"
done
echo "# machine_end $(facts)" >> "$LOG"
say "benchmark wall time: $(( $(date +%s) - T0 )) s, $seq_no processes"
python3 gen/stability_tables.py "$OUT" > "$OUT/tables.md" && say "tables: $OUT/tables.md"
