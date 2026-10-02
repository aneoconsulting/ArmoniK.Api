#!/usr/bin/env bash
# EXPERIMENT backward-encode: the codec suite's ENCODE cases, committed core vs backward core,
# alternated process by process in one session, and one short RPC probe (directions c and d).
# CONTAINER INSTRUMENTATION: no figure from this script is a campaign result.
#
#   gen/bwd_bench.sh WORKTREE OUT_DIR
#
# Variant "committed": this checkout's harness and core (poc/rust/target). Variant "backward":
# the worktree's (WORKTREE carries backward-encode.patch: the backward core AND the binding that
# delivers every repeated field last to first; the two cannot be mixed, see the README), built
# in WORKTREE/ffi/poc/rust/target. core-native (ak_rt::Enc, the forward buffer, unchanged) is in
# both processes: it is the in-process control column for build and process drift.
#
# Codec: AK_ONLY=P1..P6 (every encodable payload family and content set; U-* rows excluded),
# AK_CASE_ARMS=core-ffi,core-native, AK_CASE_DIRS=encode, AK_CASE_END=reused-buffer,
# transport-ready-tonic, AK_CASE_INPUT=hot, both unknown modes (drop, retain: full build only).
# Launch n of each variant uses seed n (same case order in both); the order is
# C1 B1 C2 B2 C3 B3. Every process runs the suite's own pre-check first (0 failures required).
# RPC: serve.sh's rpc_server on TCP 127.0.0.1 (pinned configuration), stream_probe cells Cf
# (core client, framed default), C (reference send path), Df (core-ffi encode + tonic) and A
# (tonic + prost: the control), direction d (16 MiB in 2 MiB chunks) and c (P5.4), k = 1,
# alternated C B C B, every call checked (status and server byte count).
set -Eeuo pipefail
WT=${1:?usage: bwd_bench.sh WORKTREE OUT_DIR}; OUT=${2:?out dir}
WT=$(cd "$WT" && pwd); mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; RUST=$(cd "$HERE/.." && pwd)
WR="$WT/ffi/poc/rust"
. "$HERE/machine_header.sh"
CPU=${AK_BWD_CPU:-1}
LAUNCHES=${AK_BWD_LAUNCHES:-3}
bench_exe() {  # root dir -> codec_suite bench executable (built)
  ( cd "$1" && CARGO_TARGET_DIR="$1/target" cargo bench -q -p campaign --bench codec_suite --no-run --message-format=json 2>/dev/null \
    | python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1 )
}
EC=$(bench_exe "$RUST"); EB=$(bench_exe "$WR")
( cd "$RUST" && CARGO_TARGET_DIR="$RUST/target" cargo build --release -q -p campaign --bin stream_probe )
( cd "$WR" && CARGO_TARGET_DIR="$WR/target" cargo build --release -q -p campaign --bin stream_probe )
core_of() { ldd "$1" | grep -o '/[^ ]*libak_core.so'; }
H="$OUT/header.txt"
{
  echo "# backward-encode measurement, $(date -u +%FT%TZ). CONTAINER INSTRUMENTATION, not a campaign result."
  echo "# commit $(git -C "$RUST" rev-parse HEAD)$(git -C "$RUST" diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/rust or poc/codec')"
  echo "# worktree $WT: $(git -C "$WT" rev-parse HEAD) + backward-encode.patch ($(git -C "$WT" diff --shortstat HEAD))"
  echo "# $(rustc --version); nproc $(nproc)"
  machine_header
  echo "# codec processes pinned to CPU $CPU (taskset); nothing else of this session runs meanwhile"
  echo "# committed: $EC (sha256 $(sha256sum "$EC" | cut -c1-16)) loads $(core_of "$EC") (sha256 $(sha256sum "$(core_of "$EC")" | cut -c1-16))"
  echo "# backward:  $EB (sha256 $(sha256sum "$EB" | cut -c1-16)) loads $(core_of "$EB") (sha256 $(sha256sum "$(core_of "$EB")" | cut -c1-16))"
  echo "# codec settings: AK_ONLY=P1,P2,P3,P4,P5,P6 AK_CASE_ARMS=core-ffi,core-native AK_CASE_DIRS=encode AK_CASE_END=reused-buffer,transport-ready-tonic AK_CASE_INPUT=hot AK_SAMPLES=10 AK_WARMUP_MS=${AK_BWD_WARM:-100} AK_MEASURE_MS=${AK_BWD_MEAS:-250}; launches $LAUNCHES per variant, alternated"
} > "$H"
cat "$H"
T0=$(date +%s)
for n in $(seq 1 "$LAUNCHES"); do
  for v in committed backward; do
    E=$EC; [ $v = backward ] && E=$EB
    CH=$(mktemp -d)
    s=$(date +%s)
    CRITERION_HOME="$CH" AK_LAUNCH=$n AK_OUT="$OUT/codec-$v-$n.jsonl" AK_ONLY=P1,P2,P3,P4,P5,P6 \
      AK_CASE_ARMS=core-ffi,core-native AK_CASE_DIRS=encode AK_CASE_END=reused-buffer,transport-ready-tonic AK_CASE_INPUT=hot \
      AK_SAMPLES=10 AK_WARMUP_MS=${AK_BWD_WARM:-100} AK_MEASURE_MS=${AK_BWD_MEAS:-250} AK_NRESAMPLES=1000 \
      taskset -c "$CPU" "$E" --bench > "$OUT/codec-$v-$n.out" 2> "$OUT/codec-$v-$n.err"
    rm -rf "$CH"
    echo "codec $v launch $n: $(( $(date +%s) - s )) s; $(grep -m1 '^# precheck' "$OUT/codec-$v-$n.err"); $(grep -m1 '^# narrowed' "$OUT/codec-$v-$n.err")" | tee -a "$H"
  done
done
echo "# codec benchmark wall: $(( $(date +%s) - T0 )) s" | tee -a "$H"

# ---- RPC: directions c and d through the core, both variants alternated
SS="$OUT/serve.state"
SV=$(AK_SERVE_STATE="$SS" AK_SERVER_TCP=0 AK_SERVER_THREADS=4 AK_CPU_SERVER=3 "$RUST/serve.sh" start --out "$OUT/server")
TL=$(sed -n 's/^tcp //p' <<< "$SV")
trap 'AK_SERVE_STATE="$SS" "$RUST/serve.sh" stop >/dev/null 2>&1 || true' EXIT
echo "# rpc: serve.sh rpc_server pid $(sed -n 's/^pid //p' <<< "$SV") on CPU 3, TCP $TL; probes on CPU 1-2" | tee -a "$H"
T1=$(date +%s)
for n in 1 2; do
  for v in committed backward; do
    P="$RUST/target/release/stream_probe"; [ $v = backward ] && P="$WR/target/release/stream_probe"
    AK_RPC_TARGET="http://$TL" AK_RPC_TRANSPORT=pinned AK_EXPECT_NODELAY=1 AK_OUT="$OUT/rpc-$v-$n.jsonl" \
      AK_PROBE_CELLS=Cf,C,Df,A AK_PROBE_SIZES=16MiB,P5.4 AK_PROBE_K=1 AK_PROBE_ROUNDS=4 AK_PROBE_CALLS=4 AK_PROBE_WARM=2 \
      AK_PROBE_ORDER=rotate AK_PROBE_PROC=0 taskset -c 1-2 "$P" > "$OUT/rpc-$v-$n.out" 2> "$OUT/rpc-$v-$n.err"
    echo "rpc $v process $n: rc 0, $(grep -c '^{' "$OUT/rpc-$v-$n.jsonl") rows; probe loads $(core_of "$P")" | tee -a "$H"
  done
done
echo "# rpc benchmark wall: $(( $(date +%s) - T1 )) s" | tee -a "$H"
