#!/usr/bin/env bash
# One bench run, with the JVM flags named rather than defaulted (R7).
#
# `-Xms` equal to `-Xmx` so the heap does not resize inside a round; a fixed young
# generation so a collection is a function of what an arm allocates rather than of how
# far the heap has grown; `-XX:+AlwaysPreTouch` so the first round does not pay for page
# faults the later ones do not. Nothing here tunes an ARM: the flags are the same for
# every arm in the process, which is the only property that matters for a paired ratio.
set -eu
cd "$(dirname "$0")/.."
J=${J:-/usr/lib/jvm/java-17-openjdk-amd64}
CLS=${CLS:-build/cls17}
LIB=${LIB:-$PWD/build/jni/libakjni.so}
unset JAVA_TOOL_OPTIONS || true
CP=$(cat deps/cp.txt)
exec "$J/bin/java" \
  -Xms4g -Xmx4g -XX:+AlwaysPreTouch -XX:+UseParallelGC \
  -cp "$CLS:$CP" -Dak.lib="$LIB" "$@" ak.Bench
