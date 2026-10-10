#!/usr/bin/env bash
# reset-on-entry (owner, 2026-10-10): the measurement. CONTAINER INSTRUMENTATION, absolute
# times only. One process per launch, built with `reset-on-entry` (bin roe_bench: the explicit
# and the roe paths interleaved in that process on the same core, see its header), pinned to
# one CPU; the launches run one after the other with nothing else of this session running.
#
#   gen/roe_bench.sh OUT_DIR [LAUNCHES] [CPU]
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
OUT=${1:?out dir}; LAUNCHES=${2:-3}; CPU=${3:-1}
mkdir -p "$OUT"
export AK_NO_TIMING=1
CARGO_TARGET_DIR="$PWD/target-roe" cargo build --release -q -p campaign --features reset-on-entry --bin roe_bench 2>/dev/null
B="$PWD/target-roe/release/roe_bench"
{
  echo "# commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/rust or poc/codec'); $(rustc --version)"
  echo "# binary $B ($(sha256sum "$B" | cut -c1-16)); core $(ldd "$B" | grep -o '/[^ ]*libak_core.so') ($(sha256sum "$(ldd "$B" | grep -o '/[^ ]*libak_core.so')" | cut -c1-16)), marker exports: $(nm -D --defined-only "$(ldd "$B" | grep -o '/[^ ]*libak_core.so')" | grep -c ' T ak_measure_')"
  echo "# $(grep -m1 'model name' /proc/cpuinfo); $(nproc) CPUs; taskset -c $CPU; $LAUNCHES launches, one after the other (one binary: both paths in each process)"
  echo "# settings: roe_bench --rounds 21 --round-ms 20 (the core grid's inputs, drop and retain)"
} > "$OUT/header.txt"
for l in $(seq 1 "$LAUNCHES"); do
  echo "# launch $l: $(date -u +%FT%TZ), load $(cut -d' ' -f1-3 /proc/loadavg)" >> "$OUT/header.txt"
  taskset -c "$CPU" "$B" --rounds 21 --round-ms 20 > "$OUT/launch-$l.txt"
done
echo "# done $(date -u +%FT%TZ), load $(cut -d' ' -f1-3 /proc/loadavg)" >> "$OUT/header.txt"
