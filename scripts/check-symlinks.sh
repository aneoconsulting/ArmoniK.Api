#!/bin/sh
#
# Verify that the symlinks this repository relies on are still symlinks.
#
# packages/rust/armonik reaches the shared Protos, README.md and LICENSE through symlinks, which is what
# lets `cargo package` vendor them into the published crate while keeping a single source in the repository.
#
# A checkout without symlink support (Git for Windows without core.symlinks) materialises each of them as an
# ordinary file containing its target. Committing such a file, or a copy of the tree it pointed at, would
# detach the crate from the shared source and let the two drift, so this refuses it.
#
# Run from the repository root. Diagnostics go to stderr; the exit status is the verdict.

set -u

status=0

check() {
  path=$1
  want=$2

  entry=$(git ls-files -s -- "$path")

  if [ -z "$entry" ]; then
    echo "${path}: not in the index. A symlink was expected; it may have been replaced by a directory." >&2
    return 1
  fi

  if [ "$(printf '%s\n' "$entry" | wc -l)" -ne 1 ]; then
    echo "${path}: expands to several index entries. A single symlink was expected, not a copy of the tree it points at." >&2
    return 1
  fi

  mode=$(printf '%s' "$entry" | awk '{ print $1 }')
  if [ "$mode" != "120000" ]; then
    echo "${path}: expected a symlink (mode 120000) but found mode ${mode}. On Windows without symlink support you may work with a local copy, but it must not be committed. See the Windows section of CONTRIBUTING.md." >&2
    return 1
  fi

  blob=$(git cat-file blob ":${path}")
  if [ "$blob" != "$want" ]; then
    echo "${path}: symlink points at '${blob}', expected '${want}'." >&2
    return 1
  fi

  printf '  ok  %-44s -> %s\n' "$path" "$blob"
}

# path                                            expected target
check packages/rust/armonik/protos                ../../../Protos    || status=1
check packages/rust/armonik/README.md             ../../../README.md || status=1
check packages/rust/armonik/LICENSE               ../../../LICENSE   || status=1
check packages/rust/armonik-macros/LICENSE        ../../../LICENSE   || status=1
check packages/rust/armonik-transport/LICENSE     ../../../LICENSE   || status=1

if [ "$status" -ne 0 ]; then
  echo "One or more symlinks are no longer symlinks. See the Windows section of CONTRIBUTING.md." >&2
fi

exit "$status"
