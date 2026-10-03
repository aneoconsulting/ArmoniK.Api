#!/usr/bin/env bash
# CAMPAIGN section 4.0 (D18): the h2-batch core for the rust slice's Cf-cb rows on c and d.
#
#   gen/h2batch_core.sh TARGET_DIR
#
# Builds ak-core (cdylib) from THIS workspace (poc/rust/Cargo.lock, so every dependency but h2
# is the version the stock core of target/ uses) with the full build's core features
# (as the campaign crate's default features resolve them: rpc, init-guard, unknown-fields), and h2 0.4.19 + poc/codec/h2-batch/h2-batch.patch, the
# patch the other slices build with poc/codec/h2-batch/build.sh. The h2 source is the crates.io
# .crate in cargo's cache (sha256 checked against Cargo.lock), patched into
# TARGET_DIR/h2-batch-src and given to cargo for this build only (--config patch...);
# Cargo.lock is restored on exit. The campaign's rpc_suite binary loads this core through
# LD_LIBRARY_PATH (ahead of its RUNPATH); rpc_suite checks the mapped core (AK_H2=h2-batch).
# Prints TARGET_DIR/release/libak_core.so's sha256 and the h2 source compiled into it.
set -Eeuo pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd)
PATCH="$HERE/../codec/h2-batch/h2-batch.patch"
TD=${1:?usage: gen/h2batch_core.sh TARGET_DIR}; mkdir -p "$TD"; TD=$(cd "$TD" && pwd)
VER=0.4.19
SUM=$(awk -v v="$VER" '$0=="name = \"h2\""{f=1;next} f&&$0=="version = \""v"\""{g=1;next} f&&g&&/^checksum/{gsub(/"/,"",$3);print $3;exit} /^$/{f=0;g=0}' "$HERE/Cargo.lock")
[ -n "$SUM" ] || { echo "h2batch_core.sh: h2 $VER not in $HERE/Cargo.lock" >&2; exit 2; }
CRATE=$(ls "${CARGO_HOME:-$HOME/.cargo}"/registry/cache/*/h2-$VER.crate 2>/dev/null | head -1)
[ -n "$CRATE" ] || { (cd "$HERE" && cargo fetch -q); CRATE=$(ls "${CARGO_HOME:-$HOME/.cargo}"/registry/cache/*/h2-$VER.crate | head -1); }
[ "$(sha256sum "$CRATE" | cut -c1-64)" = "$SUM" ] || { echo "h2batch_core.sh: $CRATE does not match Cargo.lock ($SUM)" >&2; exit 2; }
SRC="$TD/h2-batch-src"
rm -rf "${SRC:?}"; mkdir -p "$SRC"
tar -xzf "$CRATE" -C "$SRC" --strip-components=1
patch -s -p1 -d "$SRC" < "$PATCH"
cp "$HERE/Cargo.lock" "$TD/Cargo.lock.saved"
trap 'cp "$TD/Cargo.lock.saved" "$HERE/Cargo.lock"' EXIT
# ak-core is outside this workspace, so its features cannot be named here: the core is built
# as the stock campaign build resolves it, by building the same bench target (rpc_suite) with
# the patch; only its libak_core.so is used (the h2 of tonic in that binary is not).
(cd "$HERE" && CARGO_TARGET_DIR="$TD" cargo bench -q -p campaign --bench rpc_suite --no-run \
    --config "patch.crates-io.h2.path=\"$SRC\"" 2>/dev/null)
SO="$TD/release/deps/libak_core.so"
echo "# ak-core h2-batch (patch sha256 $(sha256sum "$PATCH" | cut -c1-16)), features as the campaign crate's default build, $(date -u +%FT%TZ)"
echo "$(sha256sum "$SO" | cut -c1-64)  $SO"
strings "$SO" | grep -o '/[^ ]*/src/codec/framed_write\.rs' | grep -o '[^/]*/src/codec/framed_write\.rs$' | sort -u | sed 's/^/    h2 compiled in: /'
