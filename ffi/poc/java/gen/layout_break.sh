#!/usr/bin/env bash
# ABI v1 section 10's guard, seen FAILING.
#
# "A guard with no failing test is a guard nobody has seen work" (README R1's second half,
# learned by the rust slice when a generator-time refusal walked the wrong edges, found
# nothing, refused nothing, and read as working).
#
# This slice is the case section 10 exists for: a Java binding has no compiler on its side,
# so its group offsets come from a layout engine in `gen/java_layout.py` that reproduces
# the SysV x86-64 rules BY HAND. This perturbs one fact in the emitted table, rebuilds only
# that class, and shows the load-time comparison naming it.
set -eu
cd "$(dirname "$0")/.."
J17=${J17:-/usr/lib/jvm/java-17-openjdk-amd64}
unset JAVA_TOOL_OPTIONS || true
CP=$(cat deps/cp.txt)
trap 'python3 gen/generate.py >/dev/null; rm -rf build/clsbreak' EXIT

echo "== ABI v1 section 10: the layout guard, seen failing =="
echo "# perturb ONE offset in the host's hand-computed table and rebuild that class only"
python3 - <<'PY'
p = "src/generated/java17/ak/shapes/Layout.java"
s = open(p).read()
# The first non-zero entry of the flat HOST table. One fact, moved by one byte.
i = s.index("public static final int[] HOST = {")
j = s.index("\n", i)
k = s.index("\n", j + 1)
line = s[j + 1:k]
val = int(line.strip().rstrip(","))
s = s[:j + 1] + line.replace(str(val), str(val + 1), 1) + s[k:]
open(p, "w").write(s)
print("#   HOST[0] %d -> %d" % (val, val + 1))
PY
mkdir -p build/clsbreak
"$J17/bin/javac" -nowarn -encoding UTF-8 -d build/clsbreak -cp "$CP" \
  -sourcepath "src/java:src/generated/java17:src/generated/shared:build/pbjava" \
  src/java/ak/RunConformance.java src/java/ak/FfiArm.java src/java/ak/PbArm.java \
  src/java/ak/BorrowArm.java 2>/dev/null
echo "# now load the binding: the comparison against the core's own export must fire"
set +e
"$J17/bin/java" -cp "build/clsbreak:$CP" -Dak.lib="$PWD/build/jni/libakjni.so" \
  ak.RunConformance 2>&1 | grep -E "section 10|disagree|core |Exception" | head -8
rc=${PIPESTATUS[0]}
set -e
echo "# exit status $rc (non-zero is the guard working)"
