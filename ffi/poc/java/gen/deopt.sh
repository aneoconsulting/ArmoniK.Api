#!/usr/bin/env bash
# README R9's measurement hazard, TESTED on this JDK rather than inherited from a report.
#
# "On JDK 21 and later a single String.format with a numeric conversion permanently
# deoptimises every char narrowing loop in the process, which is protobuf-java's own
# encoder." Every harness in this slice formats after its last measurement because of that
# sentence. This runs the encode bench twice, identical but for one `String.format("%d%n",
# 1)` executed before the first measurement, so the hazard is a number here instead of a
# rule nobody has re-checked.
#
# It runs on BOTH JDK 17 and JDK 21, because the hazard is stated for 21 and later and this
# slice's target is 17: if it reproduces on 21 and not on 17, that is worth knowing, and if
# it reproduces on neither then a rule the whole branch designs around can be retired.
set -eu
cd "$(dirname "$0")/.."
R=${ROUNDS:-20}
P=${ONLY:-P1.2}
# The content set matters more here than anywhere else in the slice. The hazard is about
# CHAR NARROWING, and an ASCII payload gives a narrowing loop nothing to narrow that a
# fast path does not already handle; the set above U+00FF is where protobuf-java's own
# encoder walks chars one at a time. Both are run.
CS=${CS:-0}
for J in /usr/lib/jvm/java-17-openjdk-amd64 /usr/lib/jvm/java-21-openjdk-amd64; do
  echo "=============================================================="
  echo "== README R9's deoptimisation hazard on $(unset JAVA_TOOL_OPTIONS; \
    "$J/bin/java" -version 2>&1 | grep -v 'Picked up' | head -1)"
  echo "=============================================================="
  for D in 0 1; do
    echo "### ak.deopt=$D  ($P, content set $CS)"
    J="$J" ./gen/bench.sh -Dak.rounds=$R -Dak.roundns=40000000 -Dak.decode=0 \
      -Dak.only=$P -Dak.deopt=$D -Dak.cs=$CS 2>/dev/null \
      | sed -n '/^id     bytes/,/^$/p'
  done
done
echo
echo "# Read the `base ns/op` column: it is protobuf-java's own absolute, and the hazard"
echo "# is a claim about protobuf-java's encoder specifically. A deopt=1 run whose base"
echo "# column is materially slower than its deopt=0 twin reproduces the hazard."
