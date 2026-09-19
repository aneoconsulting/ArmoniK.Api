#!/usr/bin/env bash
# Every file a figure in this slice depends on must be in the tree.
#
# The rust slice lost a published headline to a `.gitignore` rule that swallowed its
# benchmark binaries (its D19), and it found out by reading what `git status` did NOT list.
# So this asks git, from the repository root, rather than reading the ignore file: if a
# source, a generator, a build script or a harness is untracked or ignored, it fails.
set -u
cd "$(dirname "$0")/../../../.." || exit 2
bad=0
while IFS= read -r f; do
  if ! git ls-files --error-unmatch "$f" >/dev/null 2>&1; then
    why=$(git check-ignore -v "$f" 2>/dev/null)
    echo "UNTRACKED $f   ${why:+(ignored by $why)}"
    bad=$((bad + 1))
  fi
done < <(find ffi/poc/cpp \
           -path 'ffi/poc/cpp/build*' -prune -o \
           -path 'ffi/poc/cpp/core/target*' -prune -o \
           -name '__pycache__' -prune -o \
           -type f \( -name '*.py' -o -name '*.sh' -o -name '*.cpp' -o -name '*.cc' \
                      -o -name '*.h' -o -name '*.hpp' -o -name '*.inc' -o -name '*.rs' \
                      -o -name '*.toml' -o -name '*.md' -o -name '*.proto' -o -name '*.json' \
                      -o -name '*.bin' -o -name 'CMakeLists.txt' -o -name '.gitignore' \
                      -o -name '*.lock' \) -print)
# And the UPSTREAM inputs this slice's output is a function of. They are not this slice's
# to write, but a figure here cannot be re-derived without them, so a missing one is the
# same failure one level further out.
for f in ffi/poc/rust/gen/ir.py ffi/poc/rust/gen/rust_abi.py ffi/poc/rust/gen/rust_core.py \
         ffi/poc/rust/gen/rustnames.py ffi/poc/rust/crates/ak-abi/src/lib.rs \
         ffi/poc/rust/crates/ak-abi/src/generated/abi.rs ffi/poc/rust/crates/ak-rt/src/lib.rs \
         ffi/schema/emit/shapes.py ffi/schema/shapes.json \
         ffi/schema/generated/manifest.json ffi/schema/generated/shapes.proto; do
  if ! git ls-files --error-unmatch "$f" >/dev/null 2>&1; then
    echo "UPSTREAM-MISSING $f"
    bad=$((bad + 1))
  fi
done

echo "audit: $bad untracked, ignored or missing files that a figure could depend on"
exit $((bad ? 1 : 0))
