#!/usr/bin/env bash
# design/SHAPES.md: "A slice that reports one string-path number without saying which
# content set it came from has reported half a number."
#
# Every other figure in this slice is the ASCII set, which is what the manifest pins and
# what every id in the real schema is. These two are where a narrowing transcoder has real
# work to do -- and on the JVM they are also where the TARGET's advantage lives or dies,
# because the target's fast path is the LATIN1 coder and a string above U+00FF is not in it.
#
# P1.2 and P2.2 only: the flat string-dense payload and the shape the control plane moves.
# The rest would be the same mechanism measured again.
set -eu
cd "$(dirname "$0")/.."
R=${ROUNDS:-24}
echo "== the three content sets, on P1.2 and P2.2 =="
echo "# ASCII is the manifest's set. LATIN1 is one code point per character in U+00A0 to"
echo "# U+00FF, which a JVM String still stores compactly. Above U+00FF is U+4E00 and up,"
echo "# inside the BMP so still one char per code point, and it is where a JVM String"
echo "# becomes UTF16 -- so the binding stages twice the bytes and the core runs a"
echo "# different transcoder."
echo
for CS in 0 1 2; do
  for P in P1.2 P2.2; do
    echo "### content set $CS, $P"
    ./gen/bench.sh -Dak.rounds=$R -Dak.roundns=40000000 -Dak.cs=$CS -Dak.only=$P 2>/dev/null \
      | sed -n '/^content set:/p;/^id     bytes/,/^$/p'
  done
done
