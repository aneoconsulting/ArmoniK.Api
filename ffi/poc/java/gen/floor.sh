#!/usr/bin/env bash
# README section 5.2: the floor measured as three arms, so that "the floor costs X"
# separates what the missing APIs cost from what the old runtime costs.
#
#   a  target implementation on the target runtime   -- the headline; every ratio comes
#                                                      from here
#   b  FLOOR implementation on the TARGET runtime    -- what the floor's missing APIs cost,
#                                                      runtime held constant. The only fair
#                                                      floor-against-target ratio
#   c  floor implementation on the floor runtime     -- what a pinned consumer actually
#                                                      gets. A standalone number, never a
#                                                      ratio against a
#
# In Java the two implementations are two EMITTED SOURCE TREES from one generator, which is
# what README 5.1's first condition asks for and what Java's lack of a preprocessor forces.
# The whole divergence is how the binding reaches a String's code units: see `ak.Str17`.
set -eu
cd "$(dirname "$0")/.."
J17=${J17:-/usr/lib/jvm/java-17-openjdk-amd64}
J8=${J8:-/usr/lib/jvm/java-8-openjdk-amd64}
unset JAVA_TOOL_OPTIONS || true
CP=$(cat deps/cp.txt)
LIB=$PWD/build/jni/libakjni.so
R=${ROUNDS:-24}

echo "== README 5.2: the floor as three arms =="
echo "arm a: src/generated/java17 on JDK 17   (the headline)"
echo "arm b: src/generated/java8  on JDK 17   (the floor's APIs, runtime held constant)"
echo "arm c: src/generated/java8  on JDK 8    (what a pinned consumer gets; standalone)"
echo
"$J17/bin/java" -version 2>&1 | head -1 | sed 's/^/  target runtime: /'
"$J8/bin/java" -version 2>&1 | head -1 | sed 's/^/  floor runtime:  /'
echo

echo "## the floor is a CORRECTNESS gate first (README R8). Both trees, both runtimes."
for arm in "a $J17 build/cls17" "b $J17 build/cls8" "c $J8 build/cls8"; do
  set -- $arm
  echo "### arm $1"
  "$2/bin/java" -cp "$3:$CP" -Dak.lib="$LIB" ak.RunConformance 2>&1 | tail -3 | sed 's/^/  /'
done

echo
echo "## and only then a clock."
echo "#"
echo "# Arm b is measured INSIDE arm a's process: the floor implementation is emitted into"
echo "# a package of its own (ak.floor) over the same facade types, so the floor-against-"
echo "# target ratio is paired inside one round (R4) rather than formed across two"
echo "# processes. Arm c is a separate process by construction and stands alone."
echo "#"
echo "# In arm c ak.floor and ak.shapes are the SAME emitted source, so its ARM B rows are"
echo "# a positive control: they must read zero, and what they read instead is this"
echo "# harness's noise floor on that runtime."
for arm in "a $J17 build/cls17" "b $J17 build/cls8" "c $J8 build/cls8"; do
  set -- $arm
  echo
  echo "### arm $1  ($2, $3)"
  "$2/bin/java" -Xms2g -Xmx2g -XX:+UseParallelGC -cp "$3:$CP" -Dak.lib="$LIB" \
    -Dak.rounds=$R -Dak.roundns=60000000 -Dak.floor=1 ak.RunDelta 2>&1 \
    | sed -n '/^id     iters/,/^$/p;/ARM B/,/^$/p' | sed 's/^/  /'
done
