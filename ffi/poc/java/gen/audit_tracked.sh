#!/usr/bin/env bash
# R4's closing rule: "a figure whose harness is not in the tree cannot be defended at all."
#
# The rust slice's published encode column had to be withdrawn because a `.gitignore` rule
# meant for .NET output (`[Bb]in/`) swallowed its measurement binaries, so the commit behind
# its log could not be rebuilt. This asks git what is actually tracked rather than reading
# `.gitignore` and hoping, which is the same mistake one level up.
set -eu
cd "$(dirname "$0")/.."
bad=0
echo "== R4: every source a figure depends on is in the tree =="

# Everything that is not build output must be tracked.
while IFS= read -r f; do
  case "$f" in
    ./build/*|./core/target*|./deps/cp.txt|*/__pycache__/*|*.pyc) continue ;;
  esac
  if ! git ls-files --error-unmatch "$f" >/dev/null 2>&1; then
    echo "  UNTRACKED $f"
    bad=1
  fi
done < <(find . -type f -not -path './.git/*')

# And the generated tree must be what the generator would write now, or a log traces back
# to source that no longer exists.
echo "== the generated tree is current =="
if python3 gen/generate.py --check | grep -q '^STALE'; then
  python3 gen/generate.py --check | grep '^STALE' | sed 's/^/  /'
  bad=1
else
  echo "  ok: $(python3 gen/generate.py --check | grep -c '^ok') files"
fi

[ "$bad" = 0 ] && echo "PASS" || { echo "FAIL"; exit 1; }
