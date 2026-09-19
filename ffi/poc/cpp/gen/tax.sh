#!/usr/bin/env bash
# ABI v1 open decision 1, the batching predicate, made TRANSFERABLE.
#
# "Batching is a small loss in C++" is a statement about a 1.85 ns crossing, and it is
# about to enter a five-language specification where the same crossing costs about 8 ns on
# FFM, 12 on .NET 8 and 98 through JNI. `bench_a17_tax` adds a calibrated delay in front of
# every forward entry-point call, so the crossing can be priced up and the verdict read as
# a crossover: "it inverts at X ns" rather than "it inverts somewhere".
set -u
cd "$(dirname "$0")/.." || exit 2
R=${1:-5}
for ns in 0 2 4 8 12 24 98; do
  echo "===== crossing tax ${ns} ns per forward entry-point call ====="
  AK_TAX_NS=$ns AK_BENCH_ONLY=P1.2,P2.2,P2.3,P3.1 ./build/bench_a17_tax "$R" 2>&1 \
    | sed -n '/CROSSING TAX/,$p' \
    | sed -n '/BATCHING predicate/,/^$/p;/^CROSSING TAX/,+3p'
  echo
done
