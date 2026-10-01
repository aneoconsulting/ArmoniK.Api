#!/usr/bin/env bash
# Build the core cdylib and/or the harness against a chosen h2 (2026-10-01, h2 PR 903 task).
# Runs in a private worktree carrying the core stack; the core is never changed in the main tree.
# Run under the bench lock:
#   flock /tmp/ak-physical-bench.lock gen/h2_variant_build.sh core WT H2DIR|crates TARGET_DIR
#   flock /tmp/ak-physical-bench.lock gen/h2_variant_build.sh host WT H2DIR TARGET_DIR
# core: ffi/poc/codec (features rpc,init-guard; unknown-fields by default) -> TARGET_DIR/release/libak_core.so
# host: ffi/poc/rust campaign bins (stream_probe, upload_check, rpc_semantics) -> TARGET_DIR/release/
# The h2 source is set with cargo's --config patch table (H2DIR) or left on crates.io ("crates");
# the manifest's own [patch.crates-io] line, if any, is set aside for the build and restored.
# Prints the sha256 of each artifact and the h2 source each one embeds (panic-location paths of
# h2's src/codec/framed_write.rs in the binary: the proof of which h2 was compiled in).
set -Eeuo pipefail
what=$1 WT=$2 H2=$3 TD=$4
OSCPU=${AK_OS_CPUS:-0,9,10,19}
case $what in core) M=$WT/ffi/poc/codec/Cargo.toml;; host) M=$WT/ffi/poc/rust/Cargo.toml;; *) exit 2;; esac
cp "$M" "$M.h2vb"; trap 'mv "$M.h2vb" "$M"' EXIT
python3 - "$M" <<'PY'
import sys,re
p=sys.argv[1]; s=open(p).read()
s=re.sub(r'\n\[patch\.crates-io\]\n(#[^\n]*\n)*h2 = [^\n]*\n','\n',s); open(p,'w').write(s)
PY
cfg=(); [ "$H2" = crates ] || cfg=(--config "patch.crates-io.h2.path=\"$H2\"")
export PATH="$(cd "$(dirname "$0")" && pwd)/cargo-shim:$PATH" CARGO_TARGET_DIR="$TD"
if [ $what = core ]; then
  taskset -c "$OSCPU" cargo build -q --release --manifest-path "$M" -p ak-core --features rpc,init-guard "${cfg[@]}"
  arts=("$TD/release/libak_core.so")
else
  taskset -c "$OSCPU" cargo build -q --release --manifest-path "$M" -p campaign --bin stream_probe --bin upload_check --bin rpc_semantics "${cfg[@]}"
  arts=("$TD/release/stream_probe" "$TD/release/upload_check" "$TD/release/rpc_semantics" "$TD/release/deps/libak_core.so")
fi
echo "# $what build, manifest $M, h2 $H2, target $TD, $(date -u +%FT%TZ)"
tf=(); [ $what = core ] && tf=(--features rpc)
{ cargo tree -q --manifest-path "$M" -p "$([ $what = core ] && echo ak-core || echo campaign)" "${tf[@]}" -i h2 -e normal "${cfg[@]}" 2>&1 || true; } | head -1 | sed 's/^/# cargo tree: /'
for a in "${arts[@]}"; do
  echo "$(sha256sum "$a" | cut -c1-64)  $a"
  strings "$a" | grep -o '/[^ ]*/src/codec/framed_write\.rs' | grep -o '/[^/]*/src/codec/framed_write\.rs$' | sort -u | sed 's/^/    h2 compiled in: /'
done
