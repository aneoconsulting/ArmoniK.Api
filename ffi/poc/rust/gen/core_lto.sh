#!/usr/bin/env bash
# The core cdylib with fat LTO, built ON ITS OWN (owner: "compiling the lib itself with lto is
# fine, but compiling the host with lto wouldn't be"). Cargo applies a profile's lto to a
# whole build graph, so the core is built in poc/codec's workspace -- where the other
# slices build it -- with CARGO_PROFILE_RELEASE_LTO=fat set for that invocation only
# (codegen-units stays the default), into this slice's own target directory, with the
# features this slice's host build enables on ak-core:
#   full   rpc,init-guard,unknown-fields   -> target-core-lto/release/libak_core.so
#   nounk  rpc,init-guard                  -> target-core-lto-nounk/release/libak_core.so
# The host is then built with AK_CORE_LIB_DIR set to that directory: harness/build.rs and
# campaign/build.rs put it FIRST in the runpath, so the loader takes it; the non-LTO copy
# cargo still builds in the host's deps/ (the path dependency, for feature plumbing) is
# never loaded. The host itself keeps the default release profile (no LTO).
#   gen/core_lto.sh full|nounk   prints the directory
set -euo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd)
V=${1:?full|nounk}
case "$V" in
  full)  T="$HERE/target-core-lto";       F=(--no-default-features --features rpc,init-guard,unknown-fields) ;;
  nounk) T="$HERE/target-core-lto-nounk"; F=(--no-default-features --features rpc,init-guard) ;;
  *) echo "full|nounk" >&2; exit 2 ;;
esac
mkdir -p "$T"
CARGO_TARGET_DIR="$T" CARGO_PROFILE_RELEASE_LTO=fat cargo build --release -q \
  --manifest-path "$HERE/../codec/Cargo.toml" -p ak-core "${F[@]}" 2> "$T/core-lto-build.log" \
  || { cat "$T/core-lto-build.log" >&2; exit 1; }
echo "$T/release"
