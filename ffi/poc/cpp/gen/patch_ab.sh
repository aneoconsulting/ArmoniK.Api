#!/usr/bin/env bash
# A candidate core patch measured from C++ (physical probe, owner 2026-09-30): the HEAD core
# against the patched core, in one-cell `campaign_rpc --profile` processes with no perf attached.
#
#   gen/patch_ab.sh OUT_DIR PATCH_TREE "ENV=V ..." [checks|measure|alloc|all]
#
#   PATCH_TREE   a git worktree of this repository with the patch applied, its slice built there
#                (PATCH_TREE/ffi/poc/cpp/build-campaign and core-build/): the checks run ITS binaries;
#                the timed runs use THIS tree's campaign_rpc with LD_LIBRARY_PATH set to the
#                worktree's campaign core (it precedes the binary's RUNPATH), so the patched and HEAD
#                processes run the same binary and differ only in libak_core.so (each file's
#                header names the core path and sha256 the loader resolved)
#   "ENV=V ..."  the patch's knobs, set on the patched processes only
#
#   checks   (patched core, knobs set) conformance full and no-unknown (byte identity), the codec
#            pre-check of both builds, --semantics 1 of both builds (both send paths)
#   measure  ROUNDS (3) rounds; per round, workload and cell, HEAD and patched in rotated order:
#            cells A, D-retain, Cf-retain, Cf-q-retain alone in the process; d/16MiB k=1 and 8,
#            d/4MiB k=1, c/P5.4 k=1
#   AB_WLS / AB_CELLS narrow the workloads and cells (a follow-up run), stated in runner.log.
#   alloc    one process per (workload, cell, core) with gen/allocprobe.c preloaded: allocations of
#            at least 1 MiB during the loop (not timed: the probe wraps malloc)
# Every phase holds /tmp/ak-physical-bench.lock with its own 8-worker server (poc/rust/serve.sh at
# HEAD); CLIENT 1-4,11-14, SERVER 5-8,15-18, core 8 workers, grpc-core sized for 8 (ncpus_shim).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
FFI=$(cd "$HERE/../.." && pwd)
OUT=${1:?usage: gen/patch_ab.sh OUT_DIR PATCH_TREE "ENV=V ..." [phase]}; PT=${2:?PATCH_TREE}; KNOBS=${3:-}; PHASE=${4:-all}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd); PT=$(cd "$PT" && pwd)
PB=$PT/ffi/poc/cpp/build-campaign; PCORE=$PT/ffi/poc/cpp/core-build/target-camp/release
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1-4,11-14} AK_CPU_SERVER=${AK_CPU_SERVER:-5-8,15-18}
OSSET=${AK_CPU_OS:-0,9,10,19}; ROUNDS=${ROUNDS:-3}; WK=8; GCPUS=8
EXE=$HERE/build-campaign/campaign_rpc; SERVE=$FFI/poc/rust/serve.sh; LOCK=/tmp/ak-physical-bench.lock
LOG=$OUT/runner.log
say() { echo "$*" | tee -a "$LOG"; }
if [ "${5:-}" != --locked ]; then
  case "$PHASE" in all) PH="checks measure alloc" ;; checks|measure|alloc) PH=$PHASE ;; *) echo "phase?" >&2; exit 2 ;; esac
  T0=$(date +%s)
  for p in $PH; do
    t=$(date +%s); flock "$LOCK" bash "$0" "$OUT" "$PT" "$KNOBS" "$p" --locked || exit 1
    say "phase $p: $(( $(date +%s) - t )) s (lock held)"
  done
  say "patch_ab: $(( $(date +%s) - T0 )) s in all"
  exit 0
fi
SCR=$(mktemp -d)
trap '[ -f "$SCR/serve.state" ] && bash "$SERVE" stop > /dev/null 2>&1; rm -rf "$SCR"' EXIT
taskset -c "$OSSET" gcc -O2 -shared -fPIC -o "$SCR/ncpus.so" gen/ncpus_shim.c -ldl || exit 1
taskset -c "$OSSET" gcc -O2 -shared -fPIC -o "$SCR/allocprobe.so" gen/allocprobe.c -ldl || exit 1
WLS=("d16k1 d 16MiB 1 120" "d16k8 d 16MiB 8 20" "d4k1 d 4MiB 1 400" "c54k1 c P5.4 1 400")
CELLS=(A D-retain Cf-retain Cf-q-retain)
# Narrowed runs: AB_WLS (workload names, e.g. "d16k8") and AB_CELLS (cell labels) keep only those.
if [ -n "${AB_WLS:-}" ]; then k=(); for w in "${WLS[@]}"; do case " $AB_WLS " in *" ${w%% *} "*) k+=("$w") ;; esac; done; WLS=("${k[@]}"); fi
if [ -n "${AB_CELLS:-}" ]; then read -r -a CELLS <<< "$AB_CELLS"; fi
NC=${#CELLS[@]}
server_up() {
  export AK_SERVE_STATE=$SCR/serve.state
  AK_SERVER_THREADS=$WK bash "$SERVE" start --out "$SCR/srv" > "$SCR/srv.out" 2>&1 || { cat "$SCR/srv.out"; return 1; }
  bash "$SERVE" warm 64 > /dev/null 2>&1 || return 1
  SOCK=$(sed -n 's/^pinned //p' "$AK_SERVE_STATE")
  say "  server pid $(sed -n 's/^pid //p' "$AK_SERVE_STATE"): $(head -2 "$SCR/srv/rpc-server.log" | tail -1 | sed 's/.*tokio/tokio/')"
}
header() {
  local hc pc
  hc=$(ldd "$EXE" | grep -o '/[^ ]*libak_core\.so'); pc=$(LD_LIBRARY_PATH=$PCORE ldd "$EXE" | grep -o '/[^ ]*libak_core\.so')
  { echo "# patch_ab $1: commit $(git -C "$FFI" rev-parse --short HEAD)$(git -C "$FFI" status --porcelain -- poc/cpp/src poc/cpp/gen poc/codec | grep -q . && echo ' + UNCOMMITTED'), $(date -u +%FT%TZ)"
    echo "# patch tree $PT at $(git -C "$PT" rev-parse --short HEAD), its diff: $(git -C "$PT" diff --stat | tail -1); diff sha256 $(git -C "$PT" diff | sha256sum | cut -c1-16)"
    echo "# knobs on the patched processes: ${KNOBS:-none}; workloads: ${WLS[*]}; cells: ${CELLS[*]}"
    echo "# exe $EXE sha256 $(sha256sum "$EXE" | cut -c1-16)"
    echo "# HEAD core    $hc sha256 $(sha256sum "$hc" | cut -c1-16)"
    echo "# patched core $pc (LD_LIBRARY_PATH=$PCORE) sha256 $(sha256sum "$pc" | cut -c1-16)"
    echo "# CLIENT $AK_CPU_CLIENT SERVER $AK_CPU_SERVER; server 8 workers; core --workers $WK; grpc-core sysconf = $GCPUS (ncpus_shim); pinned; retain"
    echo "# machine $(python3 gen/machine_facts.py "$AK_CPU_CLIENT" "$AK_CPU_SERVER")"; } >> "$LOG"
  [ "$pc" = "$PCORE/libak_core.so" ] || { say "REFUSED: the patched core does not resolve ($pc)"; exit 1; }
}
run_one() {  # run_one CORE(head|patched) CELL DIRS PAY K N OUTFILE [PRELOAD]
  local core=$1 cell=$2 dirs=$3 pay=$4 k=$5 n=$6 f=$7 pre=${8:-$SCR/ncpus.so} envs=()
  [ "$core" = patched ] && envs=(LD_LIBRARY_PATH=$PCORE $KNOBS)
  taskset -c "$AK_CPU_CLIENT" env "${envs[@]}" LD_PRELOAD="$pre" AK_SHIM_NCPUS=$GCPUS "$EXE" --target "unix:$SOCK" \
    --expect 540422 --transport pinned --cells "$cell" --dirs "$dirs" --payloads "$pay" --inflight "$k" --workers $WK \
    --profile "$n" --profile-chunks 10 > "$f" 2>&1 || { tail -3 "$f"; say "FAILED $f"; exit 1; }
}
phase_checks() {
  header checks; server_up || exit 1
  local c rc
  mkdir -p "$OUT/checks"
  {
    echo "# checks of the patched core, knobs: ${KNOBS:-none}; binaries of $PB"
    for c in conformance_a17_shared conformance_nounk_a17; do
      (cd ../../schema/generated && env $KNOBS "$PB/$c" payloads > "$SCR/c" 2>&1); rc=$?
      echo "$c ($(ldd "$PB/$c" | grep -o '/[^ ]*libak_core\.so')): exit $rc: $(grep -E 'checks,' "$SCR/c" | tail -1)"
    done
    python3 gen/u_rows.py ../../corpus/generated "$SCR/rows.tsv" 2>/dev/null
    for c in campaign_codec campaign_codec_nounk; do
      (cd ../../schema/generated && env $KNOBS taskset -c "$AK_CPU_CLIENT" "$PB/$c" --rounds 0 --pool-bytes 1048576 \
        --corpus "$PWD/../../corpus/generated" --rows "$SCR/rows.tsv" > "$SCR/p" 2>&1); rc=$?
      echo "$c: exit $rc $(grep -o '"campaign_codec_gate": {[^}]*}' "$SCR/p")"
    done
    for c in campaign_rpc campaign_rpc_nounk; do
      env $KNOBS taskset -c "$AK_CPU_CLIENT" "$PB/$c" --target "unix:$SOCK" --expect 540422 --transport pinned --semantics 1 > "$SCR/s" 2>&1; rc=$?
      echo "$c --semantics 1: exit $rc $(grep -o '"failed": [0-9]*' "$SCR/s" | tail -1)"; sed 's/^/    /' "$SCR/s" | grep -iE 'reference|framed|failed' | head -40
    done
  } > "$OUT/checks/checks.log" 2>&1
  grep -E "exit" "$OUT/checks/checks.log" | tee -a "$LOG"
  grep -E "exit [^0]| [1-9][0-9]* failures|\"failed\": [1-9]" "$OUT/checks/checks.log" && { say "CHECKS FAILED"; exit 1; }
  bash "$SERVE" stop > /dev/null
}
phase_measure() {
  header measure; server_up || exit 1
  local r w i order core cell
  mkdir -p "$OUT/measure"
  for r in $(seq 1 "$ROUNDS"); do
    for w in "${WLS[@]}"; do
      set -- $w; local name=$1 dirs=$2 pay=$3 k=$4 n=$5
      for i in $(seq 0 $((NC - 1))); do
        cell=${CELLS[$(( (i + r - 1) % NC ))]}
        if [ $(( (i + r) % 2 )) = 0 ]; then order="head patched"; else order="patched head"; fi
        for core in $order; do run_one "$core" "$cell" "$dirs" "$pay" "$k" "$n" "$OUT/measure/$name-$cell-$core-r$r.out"; done
      done
      say "  measure round $r $name done"
    done
  done
  bash "$SERVE" stop > /dev/null
}
phase_alloc() {
  header alloc; server_up || exit 1
  local w cell core
  mkdir -p "$OUT/alloc"
  for w in "${WLS[@]}"; do
    set -- $w; local name=$1 dirs=$2 pay=$3 k=$4 n=$5
    for cell in "${CELLS[@]}"; do
      for core in head patched; do
        run_one "$core" "$cell" "$dirs" "$pay" "$k" "$(( n / 4 > 5 ? n / 4 : 5 ))" "$OUT/alloc/$name-$cell-$core.out" "$SCR/ncpus.so:$SCR/allocprobe.so"
      done
    done
    say "  alloc $name done"
  done
  bash "$SERVE" stop > /dev/null
}
case "$PHASE" in checks) phase_checks ;; measure) phase_measure ;; alloc) phase_alloc ;; esac
