#!/usr/bin/env bash
# Snapshot the timed binaries of build-campaign WITH their cores, for a narrowed A/B run
# (gen/opt_ab.sh). Each binary's core is found through RUNPATH, which LD_LIBRARY_PATH
# overrides, so a snapshot runs against the core it was built with.
#   gen/opt_snap.sh DIR
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
B=${BUILD:-$HERE/build-campaign}
D=$1; rm -rf "$D"; mkdir -p "$D/full" "$D/nounk"
for t in campaign_codec campaign_rpc campaign_codec_nounk campaign_rpc_nounk; do
  v=full; case $t in *nounk) v=nounk ;; esac
  cp "$B/$t" "$D/$v/"
  so=$(ldd "$B/$t" | grep -o '/[^ ]*libak_core\.so' | head -1)
  cp "$so" "$D/$v/libak_core.so"
done
( cd "$HERE" && git rev-parse --short HEAD; git status --porcelain -- . ../codec | head -5 ) > "$D/REV"
echo "snapshot $D: $(head -1 "$D/REV")"
