#!/usr/bin/env bash
# ABI v1 open decision 3's validator, which is what the decode-side number was about.
#
# Correctness first, and here it is the whole job: replacing a validator only means
# anything if the replacement rejects exactly what the original rejects. Byte identity
# cannot see this -- a manifest is made of things that encode, so it carries no malformed
# input at all -- which is why the check is an exhaustive differential test against an
# independent oracle rather than a corpus.
set -u
cd "$(dirname "$0")/.."
bad=0
for b in utf8check_a17 utf8check_c11; do
  echo "=========== $b ==========="
  ./build/$b || bad=$((bad + 1))
  echo
done
echo "utf8: $bad build(s) failed"
exit $((bad ? 1 : 0))
