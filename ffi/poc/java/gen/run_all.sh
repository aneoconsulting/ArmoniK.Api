#!/usr/bin/env bash
# Everything, in the order a fresh session should reproduce it. Correctness first, always.
set -eu
cd "$(dirname "$0")/.."
L=${L:-../../logs/java}
mkdir -p "$L"
J17=${J17:-/usr/lib/jvm/java-17-openjdk-amd64}
unset JAVA_TOOL_OPTIONS || true
CP=$(cat deps/cp.txt)
LIB=$PWD/build/jni/libakjni.so
CNT=$PWD/build/jnicnt/libakjni.so
run() { "$J17/bin/java" -cp "build/cls17:$CP" -Dak.lib="$LIB" "$@"; }

./gen/build.sh

# ---- correctness, and nothing is timed before it passes (R2)
run ak.RunConformance -v            > "$L/conformance.log" 2>&1
run ak.RunUnknown                   > "$L/unknown.log" 2>&1
"$J17/bin/java" -cp "build/cls17:$CP" -Dak.lib="$CNT" ak.RunCounts > "$L/counts.log" 2>&1
./gen/boundary.sh                   > "$L/boundary.log" 2>&1
./gen/layout_break.sh               > "$L/layout-guard.log" 2>&1

# ---- the machine, so every absolute is quotable as a multiple of it (R13)
./gen/calibrate.sh                  > "$L/calibration-r13.log" 2>&1
"$J17/bin/java" --add-opens java.base/java.nio=ALL-UNNAMED -cp build/probe \
  -Dak.lib="$PWD/build/probe/libprobe.so" Probe > "$L/crossing.log" 2>&1

# ---- is the incumbent flattered? Before any ratio is quoted.
"$J17/bin/java" -Xms4g -Xmx4g -XX:+UseParallelGC -cp "build/cls17:$CP" \
  -Dak.lib="$LIB" ak.RunBaseline    > "$L/baseline.log" 2>&1

# ---- the clock
./gen/bench.sh -Dak.rounds=36 -Dak.roundns=40000000 -Dak.decode=0 > "$L/encode.log" 2>&1
./gen/bench.sh -Dak.rounds=40 -Dak.roundns=40000000 -Dak.encode=0 > "$L/decode.log" 2>&1
"$J17/bin/java" -Xms2g -Xmx2g -XX:+UseParallelGC -cp "build/cls17:$CP" -Dak.lib="$LIB" \
  -Dak.rounds=40 -Dak.roundns=80000000 ak.RunDelta > "$L/delta.log" 2>&1
./gen/drift.sh                      > "$L/drift.log" 2>&1
ROUNDS=40 ./gen/floor.sh            > "$L/floor.log" 2>&1
ROUNDS=24 ./gen/contentsets.sh      > "$L/contentsets.log" 2>&1
ROUNDS=20 ./gen/deopt.sh            > "$L/deopt.log" 2>&1
echo "all logs in $L"
