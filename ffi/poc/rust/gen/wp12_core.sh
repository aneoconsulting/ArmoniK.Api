#!/usr/bin/env bash
# WP12: build ONE core (ak-core's cdylib) of one h2 variant with one exact feature set, and copy it
# to OUT/libak_core.so with OUT/info.txt (sha256, features, the h2 source compiled in).
#
#   gen/wp12_core.sh stock|h2-batch FEATURES OUT
#
# FEATURES is the exact, comma-separated set of ak-core features a harness build compiled the core
# with (gen/wp12-shim/cargo reads it from cargo's own artifact message), without "default".
#   - with unknown-fields in the set (a default feature): poc/codec/h2-batch/build.sh VARIANT TD
#     FEATURES, as the shared core's README says;
#   - without it (the no-unknown family), or without rpc (the corpus core: no h2 in it, and
#     build.sh's last line fails on a core without h2): build.sh cannot turn a default feature off (it passes
#     --features only), so the same cargo command by hand with --no-default-features, and for
#     h2-batch the same --config patch.crates-io.h2.path on the source build.sh patched into TD
#     (build.sh is run once first if TD holds none). The Cargo.lock cargo rewrites is restored.
# Then a second, no-op build with --message-format=json checks that the core is FRESH and was
# compiled with exactly FEATURES (+ "default" when unknown-fields is on).
# One target directory per variant, under poc/rust: target-wp12-stock, target-wp12-h2-batch.
# The whole script holds the shared core's build lock (/tmp/claude-0/ak-codec-build.lock).
set -Eeuo pipefail
LOCK=${AK_WP12_LOCK:-/tmp/claude-0/ak-codec-build.lock}
if [ -z "${AK_WP12_CORE_LOCKED:-}" ]; then
  mkdir -p "$(dirname "$LOCK")"
  AK_WP12_CORE_LOCKED=1 exec flock "$LOCK" "$0" "$@"
fi
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; RUST=$(cd "$HERE/.." && pwd); CODEC=$(cd "$RUST/../codec" && pwd)
VARIANT=${1:?usage: wp12_core.sh stock|h2-batch FEATURES OUT}; FEATURES=${2:?features}; OUT=${3:?out dir}
case $VARIANT in stock|h2-batch) ;; *) echo "wp12_core.sh: variant must be stock or h2-batch" >&2; exit 2 ;; esac
TD="$RUST/target-wp12-$VARIANT"; mkdir -p "$TD" "$OUT"
# the real cargo, never the WP12 shim
PATH=$(printf '%s' "$PATH" | tr ':' '\n' | grep -v '/gen/wp12-shim$' | paste -sd: -); export PATH
unset LD_DEBUG LD_DEBUG_OUTPUT LD_LIBRARY_PATH
has_unk=0; case ",$FEATURES," in *,unknown-fields,*) has_unk=1 ;; esac
has_rpc=0; case ",$FEATURES," in *,rpc,*) has_rpc=1 ;; esac
# build.sh is used where it can build the set: unknown-fields on (it cannot turn a default off)
# and rpc on (its last line, `strings | grep framed_write.rs`, fails the script when the core
# holds no h2: the corpus core). Otherwise the same cargo command by hand.
use_bs=$(( has_unk && has_rpc ))
cfg=(); nodef=()
[ "$has_unk" = 1 ] || nodef=(--no-default-features)
if [ "$use_bs" = 1 ]; then
  "$CODEC/h2-batch/build.sh" "$VARIANT" "$TD" "$FEATURES" | sed 's/^/  build.sh: /'
else
  if [ "$VARIANT" = h2-batch ] && [ ! -f "$TD/h2-batch-src/src/codec/framed_write.rs" ]; then
    "$CODEC/h2-batch/build.sh" h2-batch "$TD" | sed 's/^/  build.sh (to materialise h2-batch-src): /'
  fi
fi
[ "$VARIANT" = h2-batch ] && cfg=(--config "patch.crates-io.h2.path=\"$TD/h2-batch-src\"")
cp "$CODEC/Cargo.lock" "$TD/Cargo.lock.wp12-saved"
trap 'cp "$TD/Cargo.lock.wp12-saved" "$CODEC/Cargo.lock"' EXIT
if [ "$use_bs" = 0 ]; then
  echo "  by hand: cargo build --release --manifest-path poc/codec/crates/ak-core/Cargo.toml ${nodef[*]} --features $FEATURES ${cfg[*]}"
  CARGO_TARGET_DIR="$TD" CARGO_BUILD_BUILD_DIR="$TD" cargo build -q --release \
    --manifest-path "$CODEC/crates/ak-core/Cargo.toml" "${nodef[@]}" --features "$FEATURES" "${cfg[@]}"
fi
# the check: a no-op rebuild reports the core fresh, with exactly the wanted features
CARGO_TARGET_DIR="$TD" CARGO_BUILD_BUILD_DIR="$TD" cargo build -q --release --message-format=json \
    --manifest-path "$CODEC/crates/ak-core/Cargo.toml" "${nodef[@]}" --features "$FEATURES" "${cfg[@]}" 2>/dev/null \
  | python3 -S -c '
import sys, json
want = sorted(f for f in sys.argv[1].split(",") if f)
for l in sys.stdin:
    try: m = json.loads(l)
    except Exception: continue
    if m.get("reason") == "compiler-artifact" and "cdylib" in m["target"]["kind"] and m["target"]["name"] == "ak_core":
        got = sorted(f for f in m["features"] if f != "default")
        if got != want or not m["fresh"]:
            sys.exit("wp12_core.sh: the core was built with %s (fresh %s), wanted %s" % (got, m["fresh"], want))
        print("  checked: no-op rebuild fresh, ak-core features %s" % ",".join(got)); break
else:
    sys.exit("wp12_core.sh: no ak_core cdylib artifact")' "$FEATURES"
SO="$TD/release/libak_core.so"
cp "$SO" "$OUT/libak_core.so.tmp" && mv "$OUT/libak_core.so.tmp" "$OUT/libak_core.so"
{ echo "variant $VARIANT"
  echo "features $FEATURES"
  echo "built $(date -u +%FT%TZ) in $TD ($([ "$use_bs" = 1 ] && echo "poc/codec/h2-batch/build.sh" || echo "by hand, the same cargo command${nodef:+ with --no-default-features}"))"
  echo "sha256 $(sha256sum "$OUT/libak_core.so" | cut -c1-64)"
  h2=$(strings "$OUT/libak_core.so" | grep -o '[^/]*/src/codec/framed_write\.rs' | sort -u | paste -sd' ' -) || true
  echo "h2 compiled in: ${h2:-none (no h2 in this core)}"
  echo "AK_H2_COALESCE read by the core: $(grep -c AK_H2_COALESCE <(strings "$OUT/libak_core.so")) string(s)"
} > "$OUT/info.txt"
sed 's/^/  /' "$OUT/info.txt"
