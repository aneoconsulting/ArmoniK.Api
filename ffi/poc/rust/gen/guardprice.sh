#!/usr/bin/env bash
# What ABI v1 section 3's "every entry point requires `ak_init`" costs on the hot path.
#
# Two builds of one source tree, `--features init-guard` on and off, two `bench` runs each.
# SEPARATE FROM gen/lifecycle.sh on purpose: this is the only part of the lifecycle work
# that is timed, it needs the box to itself, and two benchmarks on one machine corrupt each
# other silently (README section 11). This session lost a run to exactly that, so the
# script refuses to start if anything else is already benchmarking.
#
# R4: the builds cannot share a process, so the in-process control R4 requires is carried
# instead -- `prost` and `core-native` are in every run and NEITHER has the guard. If they
# move between the builds, the machine moved and the guard's column is not readable.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

if pgrep -x bench >/dev/null 2>&1; then
  echo "REFUSING: a bench process is already running. Two benchmarks on one box corrupt" >&2
  echo "each other silently and the numbers still come out (README section 11)." >&2
  exit 2
fi

echo "===== 0. configuration ====="
rustc --version
nproc
grep -m1 'model name' /proc/cpuinfo || true

echo
echo "===== 1. build both arms ====="
# R-D9: `init-guard` is a DEFAULT harness feature, so the OFF arm turns the defaults off
# and keeps the accessor `guard`. Before this fix both arms carried the init guard.
CARGO_TARGET_DIR=target-noig cargo build --release -q -p harness --no-default-features --features guard --bin bench
CARGO_TARGET_DIR=target-ig cargo build --release -q -p harness --features init-guard --bin bench
echo "# what each binary actually loaded, from ldd and not from the build log:"
ldd target-noig/release/bench | grep ak_core
ldd target-ig/release/bench | grep ak_core

echo
echo "===== 2. two runs of each build, serialised ====="
for i in 1 2; do
  echo
  echo "----- guard OFF run $i -----"
  ./target-noig/release/bench 2>/dev/null | sed -n '/^payload/,/^# prost/p'
done
for i in 1 2; do
  echo
  echo "----- guard ON run $i -----"
  ./target-ig/release/bench 2>/dev/null | sed -n '/^payload/,/^# prost/p'
done
