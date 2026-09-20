#!/usr/bin/env bash
# C16: the P1.2 decode outlier, characterised.
#
# Every bench log this slice produced showed the `ffi` arm's first one or two rounds on
# P1.2 decode about 25 to 40 percent high, with rounds 3 to 9 flat. It was recorded,
# printed per round rather than hidden in a range, and left unexplained across several
# work units.
#
# This script is the experiment. Each block is a condition, and the per-round ratio list
# is the answer -- the outlier is visible or it is not, so nothing here needs a threshold
# or a judgement call.
set -u
cd "$(dirname "$0")/.."
B=./build

row() {  # $1 = label, rest = env assignments
  local label=$1; shift
  printf '  %-46s ' "$label"
  env "$@" AK_BENCH_ONLY=P1.2 "$B/bench_a17_shared" 2>/dev/null \
    | awk '/^P1\.2   dec  ffi /{print $NF}'
}

echo "== C16: what makes the first rounds of P1.2 decode slow? =="
echo
echo "Per-round ffi/pb on P1.2 decode, 9 rounds. The steady value is about 0.54 and the"
echo "outlier value is about 0.67-0.69. It is USUALLY rounds 1 and 2 -- that is what made it"
echo "look like a warm-up -- but not always: on a busier box it also lands on later rounds."
echo "What is invariant is the pair of values, not which round takes the high one."
echo
echo "-- is it the machine? Six runs, two linkages --"
for i in 1 2 3; do
  printf '  %-46s ' "shared, run $i"
  AK_BENCH_ONLY=P1.2 "$B/bench_a17_shared" 2>/dev/null | awk '/^P1\.2   dec  ffi /{print $NF}'
done
for i in 1 2 3; do
  printf '  %-46s ' "static, run $i"
  AK_BENCH_ONLY=P1.2 "$B/bench_a17_static" 2>/dev/null | awk '/^P1\.2   dec  ffi /{print $NF}'
done
echo "  -> the two VALUES are deterministic and identical across linkages and runs. Load"
echo "     changes how many rounds take the high value, not what the high value is, so the"
echo "     first suspicion -- this container's own stale background pollers, stopped in"
echo "     W10 -- is refuted as the cause: an idle box still shows it."
echo
echo "-- is it the UTF-8 validator? bench_a17_scalarv is the same source with AK_UTF8=0 --"
printf '  %-46s ' "AK_UTF8=1 (the table, shipped)"
AK_BENCH_ONLY=P1.2 "$B/bench_a17_shared" 2>/dev/null | awk '/^P1\.2   dec  ffi /{print $NF}'
printf '  %-46s ' "AK_UTF8=0 (the old scalar validator)"
AK_BENCH_ONLY=P1.2 "$B/bench_a17_scalarv" 2>/dev/null | awk '/^P1\.2   dec  ffi /{print $NF}'
echo "  -> NOT the same shape, and the difference is the point. With the old scalar"
echo "     validator the row is FLAT and slow (about 0.62); with the table validator it is"
echo "     fast (0.54) with the first two rounds at 0.67. The validator is not the CAUSE --"
echo "     the next block shows what is -- but it is why the outlier is visible: a decode"
echo "     dominated by a slow validator hides a fixed per-iteration allocator cost, and"
echo "     making the validator 2x faster turned that cost into a visible fraction."
echo
echo "-- is it the ALLOCATOR? glibc's mmap threshold is 128 KB by default and ADAPTS: when"
echo "   an mmap'd block is freed the threshold rises to its size, so later allocations of"
echo "   that size come from the recycled heap instead of a fresh mmap --"
row "default (adaptive threshold)"
row "MALLOC_MMAP_THRESHOLD_=4096  (always mmap)" MALLOC_MMAP_THRESHOLD_=4096
row "MMAP_THRESHOLD_=512M (never mmap)" MALLOC_MMAP_THRESHOLD_=536870912
row "...and TRIM_THRESHOLD_=512M too" MALLOC_MMAP_THRESHOLD_=536870912 MALLOC_TRIM_THRESHOLD_=536870912
echo "  -> pinning both thresholds removes the outlier AND keeps the steady state."
echo "     Forcing always-mmap reproduces the outlier's VALUE in the later rounds too."
echo
echo "-- the mechanism, counted rather than inferred: minor page faults per process --"
python3 - <<'PY'
import os, resource, subprocess
def run(env):
    e = dict(os.environ); e.update(env); e["AK_BENCH_ONLY"] = "P1.2"
    b = resource.getrusage(resource.RUSAGE_CHILDREN)
    subprocess.run(["./build/bench_a17_shared"], env=e, stdout=subprocess.DEVNULL,
                   stderr=subprocess.DEVNULL)
    a = resource.getrusage(resource.RUSAGE_CHILDREN)
    return a.ru_minflt - b.ru_minflt
d = run({})
p = run({"MALLOC_MMAP_THRESHOLD_": "536870912", "MALLOC_TRIM_THRESHOLD_": "536870912"})
print("  default allocator   minor faults: %9d" % d)
print("  thresholds pinned   minor faults: %9d" % p)
print("  -> %.1fx more, which is the mmap path faulting the block in and giving it back" % (d / max(p, 1)))
PY
echo
echo "-- the residual, which this does NOT explain --"
echo "   Running another payload first sometimes removes it and sometimes does not:"
# THREE runs per predecessor, not one. The first version of this block ran each once,
# reported "P1.1 removes it, the others do not", and the very next run of the same command
# put P1.1's outlier at round 8 instead of rounds 1-2. One run of a noisy thing is not a
# range -- the same correction T7 in the concurrency suite needed.
for pre in P1.1 P4.1; do
  for i in 1 2 3; do
    printf '  %-46s ' "$pre then P1.2, run $i"
    AK_BENCH_ONLY=$pre,P1.2 "$B/bench_a17_shared" 2>/dev/null \
      | awk '/^P1\.2   dec  ffi /{print $NF}'
  done
done
echo "   A predecessor does not reliably remove the outlier -- it MOVES it. With P1.1 first"
echo "   it may be absent, or appear at some later round; with P4.1 first it stays in rounds"
echo "   1 and 2. That is consistent with an allocator-state effect whose timing depends on"
echo "   what was allocated before, and NOT with a deterministic per-message warm-up, which"
echo "   is what a single run of this block first suggested."
echo
echo "== what this settles and what it does not =="
echo
echo "SETTLED, by removal: the outlier is page-fault cost on glibc's mmap path. Pinning the"
echo "two thresholds removes it and keeps the steady state, and the default allocator takes"
echo "an order of magnitude more minor faults. It is a property of the HOST ALLOCATOR under"
echo "this harness, not of the core, not of the ABI and not of the machine."
echo
echo "NOT SETTLED: exactly WHEN the threshold adapts. The outlier is usually rounds 1 and 2"
echo "but a different allocation history can move it to a later round or remove it, and this"
echo "does not predict which. What would settle it: a malloc hook logging size and"
echo "mmap-or-not per call, compared across predecessor orders. That is a morning's work"
echo "answering a question about glibc rather than about the ABI, which is why it stops here."
echo "The practical consequence does not depend on the residual: the outlier is allocator"
echo "page-fault cost, it is bounded by the flat rows above, and two environment variables"
echo "remove it."
echo
echo "CONSEQUENCE for the figures: none are withdrawn. This slice reports min-of-rounds and"
echo "prints every round, so no published number was taken from an outlier round -- which is"
echo "what printing them was for. The caveat it adds is real though: a C++ consumer decoding"
echo "large messages pays an allocator cost that the default glibc tuning only amortises"
echo "after the first few messages, and two MALLOC_ environment variables remove it."
