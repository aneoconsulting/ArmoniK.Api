#!/usr/bin/env bash
# The correctness gate on README 5.2's three arms, with every registered arm listed.
#
# Written for R-D5 (FIX-PLAN WP4 item 6): the pull arms `ffi-pull` and `ffi-pull-walk` were
# registered in RunConformance and timed in decode-pull.log, but no committed gate log
# named them. This script runs the gate on
#   a  java17 tree on the JDK 17 target runtime
#   b  java8  tree on the JDK 17 runtime
#   c  java8  tree on the JDK 8 floor runtime
# and prints, per arm, how many rows each registered arm took part in, so a log that
# passes with an arm silently absent cannot be mistaken for one that ran it.
#
# Also the counting build's pull section (reverse crossings must be zero), which is the
# "is it running" check for the pull family: a pull arm that fell back to push would pass
# byte identity and show reverse crossings here.
#
# No timing is taken.
set -eu
cd "$(dirname "$0")/.."
J17=${J17:-/usr/lib/jvm/java-17-openjdk-amd64}
J8=${J8:-/usr/lib/jvm/java-8-openjdk-amd64}
unset JAVA_TOOL_OPTIONS || true
# The generator state the build used: the core snapshot's poc/codec/gen (gen/build.sh).
[ -z "${AK_CODECGEN:-}" ] && [ -d build/snap/ffi/poc/codec/gen ] && export AK_CODECGEN=$PWD/build/snap/ffi/poc/codec/gen
CP=$(cat deps/cp.txt)
LIB=$PWD/build/jni/libakjni.so
CNT=$PWD/build/jnicnt/libakjni.so

echo "== java slice correctness gate, three arms (README 5.2), no timing =="
echo "tree commit: ${AK_COMMIT:-$(git rev-parse --short HEAD)}$(git diff --quiet HEAD -- . ../codec/gen || echo ' + uncommitted changes')"
echo "$(cat build/core-rev.txt 2>/dev/null)   core built with: $(grep -ho ',"features":"\[[^]]*\]' core-build/current/target/release/.fingerprint/ak-core-*/lib-ak_core.json | sort -u | tr -d '\\' | tr '\n' ' ')"
"$J17/bin/java" -version 2>&1 | head -1 | sed 's/^/target runtime: /'
"$J8/bin/java"  -version 2>&1 | head -1 | sed 's/^/floor runtime:  /'
echo "shim: $LIB  sha256 $(sha256sum "$LIB" | cut -c1-16)"
echo "core: $(ldd "$LIB" | awk '/libak_core/ {print $3}')"
echo

fail=0
for arm in "a $J17 build/cls17" "b $J17 build/cls8" "c $J8 build/cls8"; do
  set -- $arm
  out=$(mktemp)
  rc=0
  "$2/bin/java" -cp "$3:$CP" -Dak.lib="$LIB" ak.RunConformance -v > "$out" 2>&1 || rc=$?
  echo "### arm $1   runtime $2   classes $3   exit $rc"
  grep -E "^java.version|^NOTE" "$out" | sed 's/^/  /'
  echo "  rows per arm (a row is one line naming the arm: encode, pairwise, round trip):"
  for a in R ffi ffi-nobatch ffi-zeroed ffi-nobatch-zeroed ffi-pull ffi-pull-walk ffi-retain ffi-pull-retain ffi-borrow pbj; do
    n=$(grep -cE "^  P[0-9.]+  ($a  |.* $a$|$a ==|.*== $a$)" "$out" || true)
    rt=$(grep -cE "^  P[0-9.]+  $a  round trip ok" "$out" || true)
    bad=$(grep -E "^  P[0-9.]+  " "$out" | grep -E "( |^)$a( |$)" | grep -ciE "fail|mismatch|differ|error" || true)
    printf "    %-20s rows=%-4s round-trip-ok=%-4s failing=%s\n" "$a" "$n" "$rt" "$bad"
  done
  grep -E "checked=|^   !" "$out" | sed 's/^/  /'
  [ "$rc" = 0 ] || fail=1
  if [ -n "${AK_KEEP:-}" ]; then
    cp "$out" "$AK_KEEP/conformance-arm-$1.log"
  fi
  rm -f "$out"
  echo
done

echo "### unknown-field vectors, arm a"
"$J17/bin/java" -cp "build/cls17:$CP" -Dak.lib="$LIB" ak.RunUnknown 2>&1 | grep -E "checked=" | sed 's/^/  /'
echo
echo "### the pull family is running: counting core, reverse crossings (arm a)"
"$J17/bin/java" -cp "build/cls17:$CP" -Dak.lib="$CNT" ak.RunCounts 2>&1 \
  | sed -n '/== the pull family/,$p' | sed 's/^/  /'
echo
[ "$fail" = 0 ] && echo "GATE: PASS on arms a, b, c" || { echo "GATE: FAIL"; exit 1; }
