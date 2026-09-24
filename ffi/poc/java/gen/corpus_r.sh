#!/usr/bin/env bash
# R-E4: arm R (the generated pure-Java codec) against ffi/corpus, on the target (JDK 17,
# java17 tree) and on the floor (JDK 8, java8 tree), plus the two rule gaps the corpus
# cannot reach in arm R's scope, each asked of protobuf C++ as well. Confirmation only:
# nothing here fixes arm R (FIX-PLAN WP5 ports it onto the shared generator).
set -u
cd "$(dirname "$0")/.."
J17=${J17:-/usr/lib/jvm/java-17-openjdk-amd64}
J8=${J8:-/usr/lib/jvm/java-8-openjdk-amd64}
unset JAVA_TOOL_OPTIONS || true
CP=$(cat deps/cp.txt)
export AK_CORPUS_TIMEOUT_MS=${AK_CORPUS_TIMEOUT_MS:-5000}
echo "== R-E4: arm R against ffi/corpus; tree commit ${AK_COMMIT:-unknown}"
echo "   per-row timeout ${AK_CORPUS_TIMEOUT_MS} ms, whole run under timeout 1800 s"
for arm in "target $J17 build/cls17" "floor $J8 build/cls8"; do
  set -- $arm
  echo
  echo "################ $1: $2, $3"
  timeout 1800 python3 -S gen/corpus_r.py "$2/bin" "$3"
  echo "exit $?"
done

echo
echo "################ rule gaps outside the corpus's reach (target)"
mkdir -p build/rulegaps
"$J17/bin/java" -cp "build/cls17:$CP" ak.RunRuleGaps build/rulegaps
PROTOC=build/tools/protoc-3.19.0
C=../../corpus/generated
echo "-- protobuf C++ ($($PROTOC --version)) reading the same two vectors, corpus.proto:"
echo "merge-singular (ResultRaw):"
$PROTOC -I $C --decode=armonik.ffi.corpus.v1.ResultRaw $C/corpus.proto \
  < build/rulegaps/merge-singular.bin | sed 's/^/   /'
echo "merge-oneof (Probe):"
$PROTOC -I $C --decode=armonik.ffi.corpus.v1.Probe $C/corpus.proto \
  < build/rulegaps/merge-oneof.bin | sed 's/^/   /'
echo "-- implicit-presence double, -0.0: the encode test gen/java_codec.py emits for a"
echo "   singular implicit double (read, not run: shapes.proto has no such field, so no"
echo "   generated arm R code contains one):"
grep -n 'N.ZERO\[f.kind\]' gen/java_codec.py | sed 's/^/   gen\/java_codec.py:/'
grep -n '^ZERO' gen/javanames.py | sed 's/^/   gen\/javanames.py:/'
