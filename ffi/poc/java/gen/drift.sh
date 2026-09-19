#!/usr/bin/env bash
# The across-build drift control, lifted from the cpp slice rather than re-derived.
#
# That slice measured a worst across-build ratio drift of 0.240 with a semantically
# NEUTRAL perturbation -- larger than three of its own conclusions -- and the branch now
# treats the bar as a branch-level output rather than a C++ one. A slice without it cannot
# say which of its conclusions survive a rebuild.
#
# The perturbation here is Java's version of the rust slice's k exported no-ops: k dead
# static methods nothing calls, added to a class every arm loads. It changes the class
# file, the constant pool and the method ordering, and changes no behaviour at all.
#
# It drifts the DELTA instrument rather than the main bench, on purpose. The main bench
# rebuilds a pool of protobuf messages between rounds so its baseline is honest, and the
# collector that follows is already larger than most of what is being looked for; drifting
# it would measure that twice. The delta instrument has no incumbent and nothing allocating
# between rounds, so what moves between two builds of it moved for a reason that is the
# build.
set -eu
cd "$(dirname "$0")/.."
J17=${J17:-/usr/lib/jvm/java-17-openjdk-amd64}
unset JAVA_TOOL_OPTIONS || true
CP=$(cat deps/cp.txt)
ROUNDS=${ROUNDS:-20}
ONLY=${ONLY:-}

echo "== across-build drift: the same source, perturbed neutrally =="
echo "# k dead static methods in ak.Pad, which nothing calls and which change the class"
echo "# file, its constant pool and its method ordering. k in {0, 3, 11}."
echo

for K in 0 3 11; do
  python3 - "$K" <<'PY'
import sys
k = int(sys.argv[1])
body = "\n".join(
    "  static long pad%d(long x) { return x * %dL + %d; }" % (i, i + 7, i)
    for i in range(k))
src = '''package ak;

/** The across-build drift control's perturbation: k dead static methods nothing calls.
 *  Semantically neutral by construction, which is the point -- a ratio that moves between
 *  two builds of this file moved for a reason that is not the code. */
public final class Pad {
  private Pad() {}
%s
}
''' % body
open("src/java/ak/Pad.java", "w").write(src)
PY
  rm -rf build/drift$K
  mkdir -p build/drift$K
  "$J17/bin/javac" -nowarn -encoding UTF-8 -d "build/drift$K" -cp "$CP" \
    -sourcepath "src/java:src/generated/java17:src/generated/shared:build/pbjava" \
    $(find src/java src/generated/java17 src/generated/shared -name '*.java') \
    $(find build/pbjava -name '*.java') 2>/dev/null
  echo "# --- build k=$K ---"
  CP2=$(cat deps/cp.txt)
  "$J17/bin/java" -Xms2g -Xmx2g -XX:+UseParallelGC -cp "build/drift$K:$CP2" \
    -Dak.lib="$PWD/build/jni/libakjni.so" -Dak.rounds=$ROUNDS \
    -Dak.roundns=40000000 ${ONLY:+-Dak.only=$ONLY} ak.RunDelta 2>/dev/null \
    | sed -n '/^id     iters/,/^$/p'
done
rm -f src/java/ak/Pad.java
rm -rf build/drift0 build/drift3 build/drift11
echo
echo "# The drift bar for this slice is the worst per-cell move across the three builds."
echo "# Read it as: no conclusion smaller than this survives a rebuild."
