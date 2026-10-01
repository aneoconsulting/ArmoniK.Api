#!/usr/bin/env bash
# Build core variants that differ ONLY in the h2 the core transport compiles, in the private
# worktree, on the OS CPU set, under the bench lock (owner, 2026-10-01: the h2 PR #903 comparison).
#
#   gen/h2_variants_build.sh WORKTREE STACK_PATCH OUT_CORES LOG NAME=H2_DIR|crates ...
#
# The worktree is at this tree's HEAD with STACK_PATCH applied once (checked: the same diff for every
# variant, printed as the sha256 of `git diff` without ffi/poc/codec/Cargo.toml and Cargo.lock). Per
# variant: ffi/poc/codec/Cargo.toml gets `[patch.crates-io] h2 = { path = H2_DIR }` (none for
# `crates`), the slice's campaign targets are rebuilt (cores rpc,init-guard and its no-unknown twin,
# campaign_rpc(_nounk), campaign_codec(_nounk), conformance_a17_shared, conformance_nounk_a17), and
# OUT_CORES/NAME/{libak_core.so,nounk/libak_core.so} are copied out. Recorded per variant: the
# resolved h2 (cargo tree), the sha256 of the h2 sources compiled, the cores' sha256, and whether the
# AK_H2_COALESCE knob string is in the core. The LAST variant stays built in the worktree (its
# binaries serve the checks); the caller reverts the worktree.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
W=${1:?WORKTREE}; SP=${2:?STACK_PATCH}; OUT=${3:?OUT_CORES}; LOG=${4:?LOG}; shift 4
if [ "${HV_LOCKED:-}" != 1 ]; then HV_LOCKED=1 exec flock /tmp/ak-physical-bench.lock taskset -c 0,9,10,19 bash "$0" "$W" "$SP" "$OUT" "$LOG" "$@"; fi
C=$W/ffi/poc/codec; WC=$W/ffi/poc/cpp; mkdir -p "$OUT" "$(dirname "$LOG")"
exec > >(tee -a "$LOG") 2>&1
echo "# h2_variants_build $(date -u +%FT%TZ): worktree $W at $(git -C "$W" rev-parse --short HEAD), stack $SP sha256 $(sha256sum "$SP" | cut -c1-16)"
if git -C "$W" diff --quiet; then git -C "$W" apply "$SP" || exit 1; fi
cp "$C/Cargo.toml" "$OUT/Cargo.toml.base"
for spec in "$@"; do
  name=${spec%%=*}; h2=${spec#*=}
  cp "$OUT/Cargo.toml.base" "$C/Cargo.toml"
  if [ "$h2" != crates ]; then printf '\n[patch.crates-io]\nh2 = { path = "%s" }\n' "$h2" >> "$C/Cargo.toml"; fi
  echo "===== $name: h2 = $h2"
  echo "  stack diff (without the workspace Cargo.toml / Cargo.lock) sha256 $(git -C "$W" diff -- . ':!ffi/poc/codec/Cargo.toml' ':!ffi/poc/codec/Cargo.lock' | sha256sum | cut -c1-16)"
  t=$(date +%s)
  (cd "$WC" && nix-shell gen/shell.nix --run 'cmake --build build-campaign -j4 --target campaign_rpc campaign_rpc_nounk campaign_codec campaign_codec_nounk conformance_a17_shared conformance_nounk_a17' > "$OUT/build-$name.log" 2>&1) \
    || { tail -20 "$OUT/build-$name.log"; echo "BUILD FAILED $name"; exit 1; }
  echo "  built in $(( $(date +%s) - t )) s"
  (cd "$C" && cargo tree -p ak-core --features rpc,init-guard -i h2 -e normal 2>/dev/null | head -1 | sed 's/^/  cargo tree: /')
  src=$h2; [ "$h2" = crates ] && src=$(ls -d "$HOME"/.cargo/registry/src/*/h2-0.4.19 | head -1)
  echo "  h2 sources $src: src/ sha256 $(cd "$src" && find src -type f -name '*.rs' | sort | xargs sha256sum | sha256sum | cut -c1-16)"
  mkdir -p "$OUT/$name/nounk"
  cp "$WC/core-build/target-camp/release/libak_core.so" "$OUT/$name/libak_core.so"
  cp "$WC/core-build/target-camp-nounk/release/libak_core.so" "$OUT/$name/nounk/libak_core.so"
  echo "  core $(sha256sum "$OUT/$name/libak_core.so" | cut -c1-16), no-unknown $(sha256sum "$OUT/$name/nounk/libak_core.so" | cut -c1-16); AK_H2_COALESCE string in core: $(grep -c AK_H2_COALESCE "$OUT/$name/libak_core.so")"
done
echo "# done $(date -u +%FT%TZ)"
