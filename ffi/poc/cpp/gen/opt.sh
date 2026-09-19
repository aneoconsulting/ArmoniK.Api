#!/usr/bin/env bash
# Finding 2's third half: the C++ side was built at -O2 with no NDEBUG while the core is a
# cargo release build at opt-level 3. NDEBUG is now on everywhere; this asks whether the
# remaining gap between the no-boundary control and the C ABI arm is a function of the
# OPTIMISATION LEVEL, by rebuilding the whole C++ side at -O3 and re-reading the rows the
# control misbehaves on. "Two languages, two compilers" is not an explanation until the
# level is held constant.
set -u
cd "$(dirname "$0")/.." || exit 2
R=${1:-5}
rm -rf build-o3 && mkdir -p build-o3
(cd build-o3 && cmake .. -G Ninja -DAK_OPT=-O3 >/dev/null && ninja bench_a17_shared >/dev/null 2>&1)
echo "== -O2 -DNDEBUG =="
./build/bench_a17_shared "$R" 2>&1 | grep -E "^P(1\.2|2\.2|2\.3|3\.1|6\.1)   dec  (pb|native|ffi) "
echo
echo "== -O3 -DNDEBUG, the same sources =="
./build-o3/bench_a17_shared "$R" 2>&1 | grep -E "^P(1\.2|2\.2|2\.3|3\.1|6\.1)   dec  (pb|native|ffi) "
echo
echo "Both columns are ratios to their OWN in-process pb, so they are comparable across"
echo "the two builds only to the bar gen/drift.sh publishes."
