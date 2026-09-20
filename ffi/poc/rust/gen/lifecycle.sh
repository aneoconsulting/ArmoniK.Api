#!/usr/bin/env bash
# ABI v1 section 3: the lifecycle, end to end, and what its one rule costs.
#
# 1. the generator is current, and there is one core;
# 2. the lifecycle cases, guard OFF -- the state machine, the installs, the races;
# 3. the lifecycle cases, guard ON  -- and `uninit-first` now returns AK_ERR_UNINITIALIZED,
#    which is section 3's central claim exercised for the first time in this branch;
# 4. correctness with the guard on: byte identity across all four arms, unchanged;
# 5. the PRICE of the guard: the same benchmark in both builds, three runs each.
#
# Sections 2/3 and 5 are two builds of one source tree, so R4's "every ratio inside one
# process" cannot hold across them. What carries the comparison instead is the in-process
# control R4 asks for: `prost` and `core-native` are in every run and NEITHER has the
# guard, so if they move between the builds the machine moved and the guard's column is
# not readable.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

echo "===== 0. configuration ====="
rustc --version
nproc
grep -m1 'model name' /proc/cpuinfo || true

echo
echo "===== 1. generator is current, and there is one core ====="
python3 gen/generate.py --check
../codec/gen/one_core.sh | tail -3

echo
echo "===== 2. build both arms ====="
cargo build --release -q -p harness --bin lifecycle --bin bench --bin conformance
CARGO_TARGET_DIR=target-ig cargo build --release -q -p harness --features init-guard \
  --bin lifecycle --bin bench --bin conformance
echo "# what each binary actually loaded, from ldd and not from the build log:"
ldd target/release/lifecycle | grep ak_core
ldd target-ig/release/lifecycle | grep ak_core

echo
echo "===== 3. the lifecycle, guard OFF ====="
./target/release/lifecycle

echo
echo "===== 4. the lifecycle, guard ON (section 3 as specified) ====="
./target-ig/release/lifecycle

echo
echo "===== 5. correctness with the guard on ====="
echo "# The guard adds a branch to every entry point. Byte identity is re-run rather than"
echo "# assumed, on the build that has it."
./target-ig/release/conformance 2>/dev/null | tail -3

# Section 6, the PRICE of the guard, is `gen/guardprice.sh`: it is four `bench` runs and it
# must have the box to itself. Two benchmarks on one machine corrupt each other silently
# (README section 11) and this session lost one run to exactly that, so it is a separate
# script rather than a tail nobody notices is running.
