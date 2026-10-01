#!/usr/bin/env bash
# The TCP worker sweep unit (owner, 2026-10-01), end to end, each step under the bench lock:
#   1. build: the stock campaign targets (core rpc,init-guard and its no-unknown twin, campaign_rpc(_nounk),
#      campaign_codec(_nounk), conformance) and the h2-batch cores (CMake core_camp_h2batch, driving
#      poc/codec/h2-batch/build.sh, and core_camp_nounk_h2batch); sha256 and the h2 compiled in of every core;
#   2. gen/deferred_checks.sh on both variants over TCP (DC_STEP1_CORE=1: conformance and pre-check load the
#      variant's core); any failure stops the chain before timing;
#   3. gen/tcp_attrib.sh OUT sweep (SW_STOCK, SW_BATCH), tables gen/sweep_tables.py.
#   gen/sweep_chain.sh OUT_DIR       (SC_ONLY_SWEEP=1: skip steps 1 and 2, when they already ran and stand)
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
OUT=${1:?OUT_DIR}; mkdir -p "$OUT/build" "$OUT/checks"; OUT=$(cd "$OUT" && pwd)
CB=$HERE/core-build; ST=$CB/target-camp/release; STN=$CB/target-camp-nounk/release
HB=$CB/target-camp-h2batch/release; HBN=$CB/target-camp-nounk-h2batch/release
if [ "${SC_ONLY_SWEEP:-}" != 1 ]; then
echo "== 1 build $(date -u +%FT%TZ)"
flock /tmp/ak-physical-bench.lock taskset -c 0,9,10,19 nix-shell gen/shell.nix --run '
  set -e
  cmake --build build-campaign -j4 --target campaign_rpc campaign_rpc_nounk campaign_codec campaign_codec_nounk conformance_a17_shared conformance_nounk_a17
  cmake --build build-campaign -j1 --target core_camp_h2batch
  cmake --build build-campaign -j1 --target core_camp_nounk_h2batch' > "$OUT/build/build.log" 2>&1 || { tail -20 "$OUT/build/build.log"; echo "BUILD FAILED"; exit 1; }
git -C "$HERE" status --porcelain -- ../codec/Cargo.lock | grep -q . && { echo "poc/codec/Cargo.lock left modified"; exit 1; }
{ echo "# cores built $(date -u +%FT%TZ) at $(git -C "$HERE" rev-parse --short HEAD) (codec p1 82f3712a, h2-batch e4853d55, h2-batch.patch $(sha256sum ../codec/h2-batch/h2-batch.patch | cut -c1-16))"
  for v in "stock $ST" "stock-nounk $STN" "h2-batch $HB" "h2-batch-nounk $HBN"; do
    set -- $v
    echo "$1: $2/libak_core.so sha256 $(sha256sum "$2/libak_core.so" | cut -c1-64); h2 compiled in: $(strings "$2/libak_core.so" | grep -o '/[^ ]*/src/codec/framed_write\.rs' | sed 's|/src/codec/framed_write\.rs||' | sort -u | tr '\n' ' '); AK_H2_COALESCE string: $(grep -c AK_H2_COALESCE "$2/libak_core.so")"
  done
  echo "campaign_rpc resolves: $(ldd build-campaign/campaign_rpc | grep -o '/[^ ]*libak_core\.so')"; } > "$OUT/build/cores.txt"
cat "$OUT/build/cores.txt"
echo "== 2 checks $(date -u +%FT%TZ)"
F=0
DC_STEP1_CORE=1 taskset -c 0,9,10,19 bash gen/deferred_checks.sh "$OUT/checks/checks-stock.log" "$HERE/../../.." "$ST" "$STN" "" || F=1
DC_STEP1_CORE=1 taskset -c 0,9,10,19 bash gen/deferred_checks.sh "$OUT/checks/checks-h2-batch.log" "$HERE/../../.." "$HB" "$HBN" "AK_H2_COALESCE=16" || F=1
[ $F = 0 ] || { echo "CHECKS FAILED: no timing"; exit 1; }
fi
echo "== 3 sweep $(date -u +%FT%TZ)"
SW_STOCK=$ST SW_BATCH=$HB TA_NOTE="TCP worker sweep (owner, 2026-10-01): core at p1 (82f3712a), two h2 variants (stock; h2-batch e4853d55 at AK_H2_COALESCE=16); Docker running, netfilter modules loaded (a machine condition, below); Cf-q on the landed ring of 6, no ring knob${SC_NOTE:+. $SC_NOTE}" \
  taskset -c 0,9,10,19 bash gen/tcp_attrib.sh "$OUT" sweep "$ST" "$ST" "$HB" || { echo "SWEEP FAILED"; exit 1; }
python3 gen/sweep_tables.py "$OUT" > "$OUT/tables.md" && echo "tables: $OUT/tables.md"
