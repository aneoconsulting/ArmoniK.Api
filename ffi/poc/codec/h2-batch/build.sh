#!/usr/bin/env bash
# Build the shared core (ak-core: cdylib + staticlib) as one of its two h2 variants.
#
#   poc/codec/h2-batch/build.sh stock|h2-batch TARGET_DIR [FEATURES]
#
#   stock     crates.io h2 0.4.19, exactly the default build (no patch).
#   h2-batch  h2 0.4.19 + h2-batch.patch (README.md): the h2 source is materialised from the
#             crates.io .crate file in cargo's cache (its sha256 checked against Cargo.lock),
#             patched into TARGET_DIR/h2-batch-src, and given to cargo for this build only with
#             `--config patch.crates-io.h2.path=...`. Nothing lands in the default build: the
#             workspace manifest carries no [patch], and the Cargo.lock that cargo rewrites for a
#             path patch is restored on exit.
#
# FEATURES: ak-core's features, comma-separated (default `rpc,init-guard`; unknown-fields is a
# default feature). The library lands in TARGET_DIR/release/libak_core.{so,a}. The build
# directory is the target directory (CARGO_BUILD_BUILD_DIR), so a machine whose cargo
# configuration sets `build.build-dir` keeps one build per variant. Prints the library's sha256
# and the h2 source compiled into it (panic-location paths of h2's framed_write.rs).
# AK_CARGO_PREFIX (optional): a command prefix, e.g. "taskset -c 0,9,10,19".
set -Eeuo pipefail
HERE=$(cd "$(dirname "$0")" && pwd); CODEC=$(cd "$HERE/.." && pwd)
VARIANT=${1:?usage: build.sh stock|h2-batch TARGET_DIR [FEATURES]}
TD=${2:?usage: build.sh stock|h2-batch TARGET_DIR [FEATURES]}; mkdir -p "$TD"; TD=$(cd "$TD" && pwd)
FEATURES=${3:-rpc,init-guard}
read -r -a PREFIX <<< "${AK_CARGO_PREFIX:-}"
cfg=()
case $VARIANT in
  stock) ;;
  h2-batch)
    VER=0.4.19
    SUM=$(awk -v v="$VER" '$0=="name = \"h2\""{f=1;next} f&&$0=="version = \""v"\""{g=1;next} f&&g&&/^checksum/{gsub(/"/,"",$3);print $3;exit} /^$/{f=0;g=0}' "$CODEC/Cargo.lock")
    [ -n "$SUM" ] || { echo "build.sh: h2 $VER with a checksum not found in $CODEC/Cargo.lock" >&2; exit 2; }
    CRATE=$(ls "${CARGO_HOME:-$HOME/.cargo}"/registry/cache/*/h2-$VER.crate 2>/dev/null | head -1)
    if [ -z "$CRATE" ]; then
      "${PREFIX[@]}" cargo fetch -q --manifest-path "$CODEC/Cargo.toml"
      CRATE=$(ls "${CARGO_HOME:-$HOME/.cargo}"/registry/cache/*/h2-$VER.crate | head -1)
    fi
    [ "$(sha256sum "$CRATE" | cut -c1-64)" = "$SUM" ] || { echo "build.sh: $CRATE does not match Cargo.lock's checksum $SUM" >&2; exit 2; }
    SRC="$TD/h2-batch-src"
    rm -rf "${SRC:?}"; mkdir -p "$SRC"
    tar -xzf "$CRATE" -C "$SRC" --strip-components=1
    patch -s -p1 -d "$SRC" < "$HERE/h2-batch.patch"
    cfg=(--config "patch.crates-io.h2.path=\"$SRC\"")
    ;;
  *) echo "build.sh: variant must be stock or h2-batch" >&2; exit 2 ;;
esac
cp "$CODEC/Cargo.lock" "$TD/Cargo.lock.saved"
trap 'cp "$TD/Cargo.lock.saved" "$CODEC/Cargo.lock"' EXIT
CARGO_TARGET_DIR="$TD" CARGO_BUILD_BUILD_DIR="$TD" "${PREFIX[@]}" cargo build -q --release \
    --manifest-path "$CODEC/crates/ak-core/Cargo.toml" --features "$FEATURES" "${cfg[@]}"
SO="$TD/release/libak_core.so"
echo "# ak-core $VARIANT, features $FEATURES, $(date -u +%FT%TZ)"
echo "$(sha256sum "$SO" | cut -c1-64)  $SO"
strings "$SO" | grep -o '/[^ ]*/src/codec/framed_write\.rs' | grep -o '[^/]*/src/codec/framed_write\.rs$' | sort -u | sed 's/^/    h2 compiled in: /'
