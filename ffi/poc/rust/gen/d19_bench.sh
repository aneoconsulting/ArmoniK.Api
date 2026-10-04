#!/usr/bin/env bash
# D19 instrumentation (CONTAINER: not a campaign result). The UTF-16 transcoder alone, old
# against new, in one process per run (bin tc16_bench): the pre-D19 core binary's
# ak_tc_utf16 (loaded RTLD_LOCAL), the scalar export ak_tc_utf16_scalar of the D19 core, the
# D19 ak_tc_utf16 and the export ak_utf16_to_utf8. Three processes, one after the other; the
# tables (gen/d19_tables.py) give per-process medians and the spread across processes.
#
#   gen/d19_bench.sh <pre-D19 libak_core.so> <out dir>
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
BASE_CORE="${1:?usage: gen/d19_bench.sh <pre-D19 libak_core.so> <out dir>}"
OUT="${2:?out dir}"
mkdir -p "$OUT"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
cargo build --release -q -p harness --bin tc16_bench 2>/dev/null
B="$CARGO_TARGET_DIR/release/tc16_bench"
L=$(ldd "$B" | grep -o '/[^ ]*libak_core.so')
# simdutf picks its kernel at run time; report which one this CPU gets, from the same C++
# sources the crate compiles.
SU=$(ls -d "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/simdutf-0.7.0/cpp | head -1)
T=$(mktemp -d)
printf '#include "simdutf.h"\n#include <cstdio>\nint main(){std::printf("%%s (%%s)\\n",simdutf::get_active_implementation()->name().c_str(),simdutf::get_active_implementation()->description().c_str());}\n' > "$T/k.cpp"
g++ -O1 -std=c++11 -I"$SU" "$T/k.cpp" "$SU/simdutf.cpp" -o "$T/k"
{
  echo "# D19 tc16_bench, container instrumentation; $(date -u +%FT%TZ)"
  echo "# commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/rust or poc/codec'); rustc $(rustc --version | awk '{print $2}'); $(g++ --version | head -1)"
  echo "# cpu: $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'), $(nproc) cpus; simdutf 7.7.1 kernel at run time: $("$T/k")"
  echo "# D19 core: $L sha256 $(sha256sum "$L" | cut -c1-16)"
  echo "# pre-D19 core: $BASE_CORE sha256 $(sha256sum "$BASE_CORE" | cut -c1-16)"
} > "$OUT/header.txt"
rm -rf "$T"
cat "$OUT/header.txt"
for i in 1 2 3; do
  AK_D19_BASE_CORE="$BASE_CORE" "$B" > "$OUT/run-$i.txt"
  echo "run $i: $(grep -c ' | ' "$OUT/run-$i.txt") rows"
done
python3 gen/d19_tables.py "$OUT" > "$OUT/tables.md"
echo "tables: $OUT/tables.md"
