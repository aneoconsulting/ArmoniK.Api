#!/usr/bin/env bash
# The no-unknown twin of the h2-batch core (CMake target core_camp_nounk_h2batch).
#   gen/h2batch_nounk.sh CORE_ROOT H2_SRC TARGET_DIR FEATURES
# Builds ak-core with --no-default-features --features FEATURES against H2_SRC (the h2-batch source
# poc/codec/h2-batch/build.sh materialised) through cargo's --config patch table; restores
# CORE_ROOT/Cargo.lock, which cargo rewrites for a path patch.
set -Eeuo pipefail
ROOT=${1:?}; SRC=${2:?}; TD=${3:?}; FEAT=${4:?}
[ -f "$SRC/src/codec/framed_write.rs" ] || { echo "h2batch_nounk.sh: no h2 source at $SRC (build core_camp_h2batch first)" >&2; exit 2; }
mkdir -p "$TD"; cp "$ROOT/Cargo.lock" "$TD/Cargo.lock.saved"
trap 'cp "$TD/Cargo.lock.saved" "$ROOT/Cargo.lock"' EXIT
CARGO_TARGET_DIR="$TD" CARGO_BUILD_BUILD_DIR="$TD" cargo build -q --release --no-default-features --features "$FEAT" \
  --manifest-path "$ROOT/crates/ak-core/Cargo.toml" --config "patch.crates-io.h2.path=\"$SRC\""
SO="$TD/release/libak_core.so"
echo "$(sha256sum "$SO" | cut -c1-64)  $SO"
strings "$SO" | grep -o '/[^ ]*/src/codec/framed_write\.rs' | grep -o '[^/]*/src/codec/framed_write\.rs$' | sort -u | sed 's/^/    h2 compiled in: /'
