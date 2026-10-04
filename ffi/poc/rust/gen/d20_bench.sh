#!/usr/bin/env bash
# FIX-PLAN D20: what the utf8_skip mask costs on decode when every bit is 0 (before / after),
# and, as information, what skipping validation saves in the core (every bit set).
# CONTAINER INSTRUMENTATION: no figure from this script is a campaign result.
#
#   gen/d20_bench.sh OUT_DIR BEFORE_FULL_EXE BEFORE_NOUNK_EXE
#
# Three variants, each a codec_suite bench executable in the full and in the no-unknown build:
#   before   the tree before D20 (a worktree at the parent of the D20 commits; its own core)
#   after    this checkout: the D20 core, the committed binding (utf8_skip = 0)
#   skipall  this checkout's D20 core with the binding's seven `utf8_skip: 0` patched to `!0`
#            for the build only (a HARNESS-ONLY toggle; the patch is reverted before the run,
#            and the committed binding is rebuilt): every string bit set, so the core
#            validates nothing. bin d20_toggle proves which mask each build passes (a
#            malformed session_id is rejected by after and accepted by skipall).
# after and skipall load the SAME libak_core.so (the patch does not reach ak-core); before
# loads its own. Before and after cannot share a process (two cores with the same symbols),
# so core-native (the Rust-generated codec, no C ABI, unchanged by D20) runs in every process
# as the in-process control column.
#
# Settings: AK_CAMPAIGN_GRID=core (CAMPAIGN 4.0: the 16 shapes, Latin-1 and wide on P2.2,
# the 7 timed U-* rows; full build mode retain, no-unknown build mode no-unknown),
# AK_CASE_ARMS=core-ffi,core-native, AK_CASE_DIRS=decode-read, AK_SAMPLES=10,
# AK_WARMUP_MS=100, AK_MEASURE_MS=250, pinned to CPU $AK_D20_CPU (default 1) with taskset.
# Launch n of every variant uses seed n (same case order); the variant order rotates per
# launch (1: before after skipall, 2: after skipall before, 3: skipall before after); in each
# slot the full build runs, then the no-unknown build. Every process runs the suite's
# pre-check first (0 failures required).
set -Eeuo pipefail
OUT=${1:?usage: gen/d20_bench.sh OUT_DIR BEFORE_FULL_EXE BEFORE_NOUNK_EXE}
BF=${2:?before full exe}; BN=${3:?before nounk exe}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; RUST=$(cd "$HERE/.." && pwd)
cd "$RUST"
. "$HERE/machine_header.sh"
CPU=${AK_D20_CPU:-1}; LAUNCHES=${AK_D20_LAUNCHES:-3}
exe_of() {
  python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1
}
build() {  # target-dir extra-flags... -> prints the codec_suite exe
  local td=$1; shift
  CARGO_TARGET_DIR="$RUST/$td" cargo bench -q -p campaign "$@" --bench codec_suite --no-run --message-format=json 2>/dev/null | exe_of
}
FULLF=(); NOUNKF=(--no-default-features --features init-guard)
NOUNKH=(--no-default-features --features guard,init-guard)
BIND=(crates/harness/src/generated/binding.rs crates/harness/src/generated/binding_nounk.rs)
git diff --quiet HEAD -- "${BIND[@]}" || { echo "the committed binding is modified; refusing"; exit 1; }
for f in "${BIND[@]}"; do [ "$(grep -c 'utf8_skip: 0,' "$f")" = 7 ] || { echo "$f: not 7 utf8_skip sites"; exit 1; }; done

toggle() {  # target-dir expect harness-flags...
  local td=$1 want=$2; shift 2
  CARGO_TARGET_DIR="$RUST/$td" AK_D20_EXPECT=$want cargo run --release -q -p harness "$@" --bin d20_toggle 2>/dev/null
}
H="$OUT/header.txt"
{
  echo "# D20 utf8_skip decode measurement, $(date -u +%FT%TZ). CONTAINER INSTRUMENTATION, not a campaign result."
  echo "# commit $(git rev-parse HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/rust or poc/codec')"
  echo "# $(rustc --version); nproc $(nproc)"
  machine_header
} > "$H"

# ---- skipall: patch, build, keep copies, revert -------------------------------------------
cleanup() { git checkout -q -- "${BIND[@]}"; }
trap cleanup EXIT
for f in "${BIND[@]}"; do sed -i 's/utf8_skip: 0,/utf8_skip: !0, \/\/ D20 BENCH TOGGLE (never committed): every bit set/' "$f"; done
SF=$(build target "${FULLF[@]}"); SN=$(build target-nounk "${NOUNKF[@]}")
cp "$SF" "$(dirname "$SF")/codec_suite-d20-skipall"; SF="$(dirname "$SF")/codec_suite-d20-skipall"
cp "$SN" "$(dirname "$SN")/codec_suite-d20-skipall"; SN="$(dirname "$SN")/codec_suite-d20-skipall"
{ echo "# toggle, skipall full:  $(toggle target accept | tr '\n' ' ')"
  echo "# toggle, skipall nounk: $(toggle target-nounk accept "${NOUNKH[@]}" | tr '\n' ' ')"; } >> "$H"
cleanup; trap - EXIT
git diff --quiet HEAD -- "${BIND[@]}" || { echo "the binding was not restored"; exit 1; }
# ---- after: the committed binding, rebuilt ------------------------------------------------
AF=$(build target "${FULLF[@]}"); AN=$(build target-nounk "${NOUNKF[@]}")
cp "$AF" "$(dirname "$AF")/codec_suite-d20-after"; AF="$(dirname "$AF")/codec_suite-d20-after"
cp "$AN" "$(dirname "$AN")/codec_suite-d20-after"; AN="$(dirname "$AN")/codec_suite-d20-after"
{ echo "# toggle, after full:    $(toggle target reject | tr '\n' ' ')"
  echo "# toggle, after nounk:   $(toggle target-nounk reject "${NOUNKH[@]}" | tr '\n' ' ')"; } >> "$H"
grep -q "WRONG" "$H" && { cat "$H"; echo "a toggle is not what it should be; nothing timed"; exit 1; }

core_of() { ldd "$1" | grep -o '/[^ ]*libak_core.so'; }
declare -A EXE=([before-full]=$BF [before-nounk]=$BN [after-full]=$AF [after-nounk]=$AN [skipall-full]=$SF [skipall-nounk]=$SN)
{
  for k in before-full before-nounk after-full after-nounk skipall-full skipall-nounk; do
    e=${EXE[$k]}
    echo "# $k: $e (sha256 $(sha256sum "$e" | cut -c1-16)) loads $(core_of "$e") (sha256 $(sha256sum "$(core_of "$e")" | cut -c1-16); pvt setters exported: $(nm -D --defined-only "$(core_of "$e")" | grep -c ' T ak_dec_set_pvt_'))"
  done
  echo "# settings: AK_CAMPAIGN_GRID=core AK_CASE_ARMS=core-ffi,core-native AK_CASE_DIRS=decode-read AK_SAMPLES=10 AK_WARMUP_MS=100 AK_MEASURE_MS=250; $LAUNCHES launches; CPU $CPU (taskset); nothing else of this session runs meanwhile"
} >> "$H"
cat "$H"

T0=$(date +%s)
ORDERS=("before after skipall" "after skipall before" "skipall before after")
for l in $(seq 1 "$LAUNCHES"); do
  for v in ${ORDERS[$(( (l - 1) % 3 ))]}; do
    for b in full nounk; do
      e=${EXE[$v-$b]}
      CH=$(mktemp -d)
      CRITERION_HOME="$CH" AK_LAUNCH=$l AK_OUT="$OUT/codec-$v-$b-$l.jsonl" AK_CAMPAIGN_GRID=core \
        AK_CASE_ARMS=core-ffi,core-native AK_CASE_DIRS=decode-read \
        AK_SAMPLES=10 AK_WARMUP_MS=100 AK_MEASURE_MS=250 AK_NRESAMPLES=1000 \
        taskset -c "$CPU" "$e" --bench > "$OUT/codec-$v-$b-$l.out" 2> "$OUT/codec-$v-$b-$l.err"
      rm -rf "$CH"
      echo "$v $b launch $l: $(grep -m1 '^# precheck' "$OUT/codec-$v-$b-$l.err")" | tee -a "$H"
    done
  done
done
echo "# benchmark wall: $(( $(date +%s) - T0 )) s" | tee -a "$H"
python3 "$HERE/d20_tables.py" "$OUT" > "$OUT/tables.md"
echo "tables: $OUT/tables.md"
