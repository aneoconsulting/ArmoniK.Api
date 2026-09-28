#!/usr/bin/env bash
# The cpp slice's SHORT optimisation benchmark: one fixed run of about 9 minutes of benchmark
# wall time in this container (4 CPUs), rerun UNCHANGED after each change so that two runs
# compare. It builds as the campaign runner builds (gen/run_campaign.sh: the same CMake
# configuration, -DAK_RPC=ON, Google Benchmark v1.8.3 Release from the checked upstream tag,
# the same targets) and runs the same benchmark executables with the runner's arguments,
# pinned the runner's way, with fewer repetitions, one launch and NO GATE (the owner's
# instruction for the optimisation phase: the correctness gate runs once, at the end of the
# experiment). Every figure it writes is CONTAINER INSTRUMENTATION, not a campaign result.
#
#   gen/opt_bench.sh OUT_DIR
#
# What stays on: each codec process's own byte-identity pre-check (every slot of every group
# checked once before any timing; exit 2 and no figure on a failure), and the RPC client's own
# checks (the C-F pre-check against the incumbent, the c/d request bytes, the d SHA-256 once per
# cell and payload, and every call's status and length; the first failure aborts, exit 3, no
# figure). Nothing else of the gate runs.
#
# Order of work (each step's elapsed time in OUT_DIR/runner.log):
#   1. build: generate.py --check (recorded), Google Benchmark v1.8.3 Release (the runner's
#      gbench_release, extracted from run_campaign.sh), cmake -DAK_RPC=ON, the timed and the
#      counting targets; each binary checked to load the core of its variant.
#   2. crossings (req 19), RECORDED, not fatal (OPT_CROSSINGS=0 skips): counts_a17_shared and
#      counts_nounk against the committed baselines.
#   3. codec, launch 1: payloads (--only P) on the full build, then on the no-unknown build;
#      then the 92 U-* rows (--only U-) on each, at the reduced U settings.
#   4. rpc, launch 1: ONE server (poc/rust/serve.sh, pinned to AK_CPU_SERVER) for the run,
#      warmed; the RPC crossing counts (recorded, not fatal); then per transport (shipped,
#      pinned) the full client, then the no-unknown client.
#   5. summary: gen/opt_summary.py (summary-codec.tsv, variants-codec.tsv, summary-rpc.tsv,
#      tables-codec.md, tables-rpc.md).
#
# H-1 (the pool rows encoded pool[0] only) is fixed in campaign_codec.cpp since the ref run; the
# baseline run (logs/cpp/opt/baseline) predates the fix and its tables mark those rows.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$HERE" || exit 2
[ $# = 1 ] || { echo "usage: $0 OUT_DIR" >&2; exit 2; }
mkdir -p "$1"; OUT="$(cd "$1" && pwd)"
FFI=$(cd "$HERE/../.." && pwd)
REPO=$(git -C "$FFI" rev-parse --show-toplevel)

# ---- fixed settings (change one and the run no longer compares with the others) --------
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1} AK_CPU_SERVER=${AK_CPU_SERVER:-2,3}
P_ROUNDS=5; P_MIN_TIME_S=0.01;  P_WARMUP_S=0.005     # payloads: repetitions, min time, warm-up
U_ROUNDS=3; U_MIN_TIME_S=0.003; U_WARMUP_S=0.001     # U-* rows: reduced (a warm-up > 0: with 0, a cold
                                                     # first call above min_time became the repetition)
POOL=1048576                                         # input=pool bytes: NOT beyond this container's LLC
                                                     # (L2 1 MiB/core, L3 33 MiB); 2 x LLC costs about
                                                     # 0.3 s per pool build, out of the budget
LLC=14417920                                         # the runner's default, recorded only
RPC_ROUNDS=3; RPC_MIN_TIME_S=0.04; RPC_WARMUP_S=0.02; SRV_WARM=50
TRANSPORTS="shipped pinned"; LAUNCH=1
# k = 16 dropped 2026-09-28 (step 8d, the owner's budget rule): with the pull and borrow arms the
# run took 613 s; every k=16 repetition was a single batch at 0.04 s anyway (C41).
INFLIGHT=1,8
SETTINGS="launch=$LAUNCH; codec P (--only P): rounds=$P_ROUNDS min_time_s=$P_MIN_TIME_S warmup_s=$P_WARMUP_S; codec U (--only U-, the 92 rows): rounds=$U_ROUNDS min_time_s=$U_MIN_TIME_S warmup_s=$U_WARMUP_S; pool_bytes=$POOL (walked since H-1; not beyond this container's 33 MiB L3); rpc: rounds=$RPC_ROUNDS min_time_s=$RPC_MIN_TIME_S warmup_s=$RPC_WARMUP_S inflight=$INFLIGHT transports=$TRANSPORTS, server warm-up serve.sh warm $SRV_WARM, server tokio workers ${AK_SERVER_THREADS:-4} (serve.sh default); order: codec full-P, nounk-P, full-U, nounk-U; rpc one server, per transport full then nounk client; Google Benchmark random interleaving within each process"

B=${BUILD:-$HERE/build-campaign}
SCRATCH=$(mktemp -d)
LOG="$OUT/runner.log"; : > "$LOG"
say() { echo "$*" | tee -a "$LOG"; }
T0=$(date +%s)
step() { say "[$(( $(date +%s) - T0 ))s] $*"; }
export AK_SERVE_STATE=$SCRATCH/serve.state   # this run's own server, never another's
SERVE=$FFI/poc/rust/serve.sh
cleanup() { [ -f "$AK_SERVE_STATE" ] && bash "$SERVE" stop > /dev/null 2>&1; rm -rf "$SCRATCH"; true; }
trap cleanup EXIT

# ---- the runner's header (requirement 27), extracted from run_campaign.sh ---------------
DIRTY=$(git -C "$REPO" status --porcelain -- ffi/poc/cpp ffi/poc/codec ffi/schema ffi/corpus 2>/dev/null)
COMMIT=$(git -C "$REPO" rev-parse HEAD)
eval "$(sed -n '/^header() {/,/^EOF$/p' gen/run_campaign.sh; echo '}')"
eval "$(sed -n '/^GB_TAG=/,/^}/p' gen/run_campaign.sh)"   # GB_TAG, GB_COMMIT, GBPREFIX, gbench_release
declare -F header > /dev/null && declare -F gbench_release > /dev/null \
  || { echo "could not extract header()/gbench_release() from gen/run_campaign.sh" >&2; exit 2; }
LAUNCHES=1; ROUNDS=0; BUILDS=both   # the header's variables (TRANSPORTS is set above)
MINT=$P_MIN_TIME_S; WARM=$P_WARMUP_S; RPCMINT=$RPC_MIN_TIME_S; RPCWARM=$RPC_WARMUP_S; SRVWARM=$SRV_WARM
CITERS=0
opt_header() {  # opt_header SUITE ROUNDS WHAT: the runner's JSON header, marked, plus two lines
  ROUNDS=$2 header "$1" "$LAUNCH" | OPT_WHAT="$3" OPT_SETTINGS="$SETTINGS" python3 -c '
import json, os, sys
for l in sys.stdin:
    h = json.loads(l[2:])
    h["instrumentation"] = True
    h["opt_bench"] = {"script": "gen/opt_bench.sh", "gated": False, "file": os.environ["OPT_WHAT"],
                      "settings": os.environ["OPT_SETTINGS"],
                      "known_defects": []}
    print("# " + json.dumps(h, sort_keys=True))'
  echo "# settings   $SETTINGS"
  echo "# CONTAINER INSTRUMENTATION, not gated (gen/opt_bench.sh; the correctness gate is not run in the optimisation phase); the codec process's own pre-check and the RPC client's own checks are on"
}
{ opt_header runner 0 runner.log; } > "$OUT/header.txt"
cat "$OUT/header.txt" >> "$LOG"
say "# commit $(git -C "$REPO" rev-parse --short HEAD)${DIRTY:+ + UNCOMMITTED CHANGES}"

# ---- 1. build, as run_campaign.sh's gate builds (targets limited to what runs here) ------
step "build: generate.py --check, Google Benchmark $GB_TAG Release, cmake -DAK_RPC=ON"
if python3 gen/generate.py --check > "$SCRATCH/gen.log" 2>&1; then say "  generate.py --check: every target current"
else say "  generate.py --check: NOT CURRENT (recorded):"; grep -v '^ok' "$SCRATCH/gen.log" | head -5 | tee -a "$LOG"; fi
gbench_release || { say "Google Benchmark release build failed"; exit 1; }
TARGETS="campaign_codec campaign_codec_nounk campaign_rpc campaign_rpc_nounk counts_a17_shared counts_nounk campaign_rpc_count campaign_rpc_count_nounk"
{ cmake -S . -B "$B" -DAK_RPC=ON -Dbenchmark_DIR="$GBPREFIX/lib/cmake/benchmark" \
    && cmake --build "$B" -j"$(nproc)" --target $TARGETS; } > "$OUT/build.log" 2>&1 \
  || { tail -20 "$OUT/build.log"; say "build failed (build.log)"; exit 1; }
bash "$SERVE" build >> "$OUT/build.log" 2>&1 || { say "serve.sh build failed (build.log)"; exit 1; }
say "  built: $TARGETS; the server (poc/rust/serve.sh build)"
ufam() { nm -D --defined-only "$1" 2>/dev/null | grep -cE ' (ak_uencode_|ak_uelem|ak_dec_reset_)'; }
for t in $TARGETS; do
  so=$(ldd "$B/$t" | grep -o '/[^ ]*libak_core\.so' | head -1); n=$(ufam "$so")
  case $t in *nounk*) want=0 ;; *) want=1 ;; esac
  if { [ $want = 0 ] && [ "$n" != 0 ]; } || { [ $want = 1 ] && [ "$n" = 0 ]; }; then
    say "variant mix-up: $t loads $so ($n u-family exports)"; exit 1
  fi
  say "  variant ok: $t -> ${so#$HERE/} ($n u-family exports)"
done
ROWS=$SCRATCH/campaign_unknown_rows.tsv
python3 gen/u_rows.py "$FFI/corpus/generated" "$ROWS" 2>/dev/null
say "  U-* rows: $(wc -l < "$ROWS") (gen/u_rows.py)"

# ---- 2. crossings (recorded, not fatal) -------------------------------------------------
if [ "${OPT_CROSSINGS:-1}" = 1 ]; then
  step "crossings: counting builds, both variants, against the committed baselines"
  for v in full nounk; do
    exe=counts_a17_shared; base=counts-baseline.log; [ $v = nounk ] && { exe=counts_nounk; base=counts-nounk-baseline.log; }
    (cd "$FFI/schema/generated" && "$B/$exe" --corpus "$FFI/corpus/generated" --rows "$ROWS" > "$SCRATCH/cnt.log" 2>&1)
    grep -E '^  [PU]' "$FFI/logs/cpp/$base" > "$SCRATCH/want"; grep -E '^  [PU]' "$SCRATCH/cnt.log" > "$SCRATCH/got"
    if diff "$SCRATCH/want" "$SCRATCH/got" > "$SCRATCH/d"; then
      say "  counts ($v): $(wc -l < "$SCRATCH/got") rows identical to logs/cpp/$base"
    else
      cp "$SCRATCH/cnt.log" "$OUT/counts-$v-current.log"; cp "$SCRATCH/d" "$OUT/counts-$v.diff"
      say "  counts ($v): DIFFER from logs/cpp/$base (kept: counts-$v.diff, counts-$v-current.log)"
    fi
  done
fi

# ---- 3. codec ---------------------------------------------------------------------------
codec_run() {  # codec_run TAG EXE BUILD ONLY ROUNDS MIN_TIME WARMUP
  local tag=$1 exe=$2 bld=$3 only=$4 r=$5 mt=$6 wu=$7 t rc
  local F="$OUT/$tag.jsonl" G="$SCRATCH/$tag.gbench.json" C="$SCRATCH/$tag.console"
  t=$(date +%s)
  (cd "$FFI/schema/generated" && taskset -c "$AK_CPU_CLIENT" "$B/$exe" --launch "$LAUNCH" --rounds "$r" \
     --min-time-s "$mt" --warmup-s "$wu" --only "$only" --corpus "$FFI/corpus/generated" --rows "$ROWS" \
     --gbench-out "$G" --pool-bytes "$POOL" > "$C" 2>&1); rc=$?
  if [ $rc != 0 ] || [ ! -s "$G" ]; then
    cp "$C" "$OUT/$tag.FAILED.console"; say "codec $tag FAILED (exit $rc; pre-check or run): $tag.FAILED.console"; exit 1
  fi
  { MINT=$mt WARM=$wu opt_header codec "$r" "$tag.jsonl ($exe --only $only)"
    echo "# this file  $exe --launch $LAUNCH --rounds $r --min-time-s $mt --warmup-s $wu --only $only --pool-bytes $POOL --corpus corpus/generated --rows (gen/u_rows.py)"
    grep '^#' "$C"
    python3 gen/gbench_to_jsonl.py "$G" "$LAUNCH" "$bld"; } > "$F" 2>/dev/null \
    || { say "codec $tag: the Google Benchmark output was refused"; exit 1; }
  gzip -9c "$G" > "$OUT/$tag.gbench.json.gz"
  say "  codec $tag: $(grep -o '"campaign_codec_gate": {[^}]*}' "$C"); $(grep -c '^{' "$F") samples; $(( $(date +%s) - t ))s -> $(basename "$F")"
}
step "codec: payloads, full build"
codec_run codec-P campaign_codec full P $P_ROUNDS $P_MIN_TIME_S $P_WARMUP_S
step "codec: payloads, no-unknown build"
codec_run codec-nounk-P campaign_codec_nounk no-unknown P $P_ROUNDS $P_MIN_TIME_S $P_WARMUP_S
step "codec: U-* rows, full build (reduced)"
codec_run codec-U campaign_codec full U- $U_ROUNDS $U_MIN_TIME_S $U_WARMUP_S
step "codec: U-* rows, no-unknown build (reduced)"
codec_run codec-nounk-U campaign_codec_nounk no-unknown U- $U_ROUNDS $U_MIN_TIME_S $U_WARMUP_S

# ---- 4. rpc: one server for the run, both transports, both clients ----------------------
step "rpc: the server (poc/rust/serve.sh start, pinned to $AK_CPU_SERVER), warm $SRV_WARM"
bash "$SERVE" start --out "$SCRATCH/srv" > "$SCRATCH/srv.out" 2>&1 || { cat "$SCRATCH/srv.out"; say "the server did not start"; exit 1; }
SOCK_shipped=$(awk '$1=="shipped"{print $2}' "$SCRATCH/srv.out"); SOCK_pinned=$(awk '$1=="pinned"{print $2}' "$SCRATCH/srv.out")
EXP=$(sed -n 's/.*P2.2 \([0-9]*\) B.*/\1/p' "$SCRATCH/srv/rpc-server.log" | head -1)
[ -n "$EXP" ] && [ -n "$SOCK_shipped" ] && [ -n "$SOCK_pinned" ] || { say "the server did not report its sockets"; exit 1; }
sock_of() { [ "$1" = pinned ] && echo "unix:$SOCK_pinned" || echo "unix:$SOCK_shipped"; }
taskset -c "$AK_CPU_CLIENT" bash "$SERVE" warm "$SRV_WARM" > "$SCRATCH/warm.log" 2>&1 || { cat "$SCRATCH/warm.log"; say "server warm-up failed"; exit 1; }
say "  server: P2.2 $EXP B; $(tr '\n' ' ' < "$SCRATCH/warm.log" | cut -c1-200)"
if [ "${OPT_CROSSINGS:-1}" = 1 ]; then
  for v in "" _nounk; do
    want=$FFI/logs/cpp/rpc-counts$( [ -n "$v" ] && echo -nounk ).log
    taskset -c "$AK_CPU_CLIENT" "$B/campaign_rpc_count$v" --target "$(sock_of shipped)" --expect "$EXP" \
      --transport shipped --count 4 > "$SCRATCH/rc.log" 2>&1
    if diff <(grep -E '^  [BCDE]' "$want") <(grep -E '^  [BCDE]' "$SCRATCH/rc.log") > "$SCRATCH/rd"; then
      say "  rpc counts$v: $(grep -cE '^  [BCDE]' "$SCRATCH/rc.log") rows identical to logs/cpp/$(basename "$want")"
    else
      cp "$SCRATCH/rc.log" "$OUT/rpc-counts$v-current.log"; cp "$SCRATCH/rd" "$OUT/rpc-counts$v.diff"
      say "  rpc counts$v: DIFFER from logs/cpp/$(basename "$want") (kept)"
    fi
  done
fi
for T in $TRANSPORTS; do
  for bld in full no-unknown; do
    exe=campaign_rpc; tag=rpc-$T; [ $bld = no-unknown ] && { exe=campaign_rpc_nounk; tag=rpc-$T-nounk; }
    step "rpc: transport $T, $exe"
    F="$OUT/$tag.jsonl" G="$SCRATCH/$tag.gbench.json" C="$SCRATCH/$tag.console"; t=$(date +%s)
    timeout 1800 taskset -c "$AK_CPU_CLIENT" "$B/$exe" --target "$(sock_of "$T")" --expect "$EXP" --transport "$T" \
      --launch "$LAUNCH" --rounds "$RPC_ROUNDS" --min-time-s "$RPC_MIN_TIME_S" --warmup-s "$RPC_WARMUP_S" \
      --inflight "$INFLIGHT" --gbench-out "$G" > "$C" 2>&1; rc=$?
    if [ $rc != 0 ] || [ ! -s "$G" ]; then
      cp "$C" "$OUT/$tag.FAILED.console"
      say "rpc $T ($exe) ABORTED (exit $rc; requirement 18): no figure; $(grep -m1 'CALL CHECK' "$C")"; exit 1
    fi
    { opt_header rpc "$RPC_ROUNDS" "$tag.jsonl ($exe, transport $T)"
      echo "# this file  $exe --target <$T socket> --expect $EXP --transport $T --launch $LAUNCH --rounds $RPC_ROUNDS --min-time-s $RPC_MIN_TIME_S --warmup-s $RPC_WARMUP_S --inflight $INFLIGHT"
      echo "# server: poc/rust rpc_server (tonic; SERVER.md), one process for the whole rpc step, pinned to $AK_CPU_SERVER, warmed by serve.sh warm $SRV_WARM"
      sed 's/^/# server: /' "$SCRATCH/srv/rpc-server.log"
      grep '^#' "$C"
      python3 gen/gbench_to_jsonl.py "$G" "$LAUNCH" "$bld" rpc; } > "$F" 2>/dev/null \
      || { say "rpc $tag: the Google Benchmark output was refused"; exit 1; }
    gzip -9c "$G" > "$OUT/$tag.gbench.json.gz"
    say "  rpc $tag: $(grep -c '^{' "$F") samples; $(( $(date +%s) - t ))s -> $(basename "$F")"
  done
done
bash "$SERVE" stop > /dev/null 2>&1
cp "$SCRATCH/srv/rpc-server.log" "$OUT/rpc-server.log"
say "  server stopped: $(tail -1 "$OUT/rpc-server.log" | cut -c1-200)"

# ---- 5. summary -------------------------------------------------------------------------
step "summary"
python3 gen/opt_summary.py "$OUT" 2>&1 | tee -a "$LOG"
step "done"
