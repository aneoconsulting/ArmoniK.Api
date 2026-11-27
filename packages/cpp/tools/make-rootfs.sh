#!/bin/sh
# Build a minimal musl root filesystem for a distroless runtime image.
#
# Usage: make-rootfs.sh <rootfs> <path>...
#
# Must run in an Alpine image. Scans the ELF files under each <path> for the
# shared libraries they need, installs into <rootfs> only the Alpine packages
# providing them (plus the CA bundle), then removes every executable and the
# apk database so the result has no shell and no package manager.
# Libraries found under <path> itself (the ArmoniK libraries) are skipped.
set -eu

rootfs="$1"
shift

apk add --no-cache scanelf >/dev/null

own="$(find "$@" -name '*.so*' -exec basename {} \; | sort -u)"
deps=""
for so in $(scanelf -RBF '%n#F' "$@" | tr ',' '\n' | sort -u); do
  if ! printf '%s\n' "$own" | grep -qxF "$so"; then
    deps="$deps so:$so"
  fi
done
echo "Runtime dependencies:$deps"

mkdir -p "$rootfs"
# --no-scripts: package scripts need /bin/sh, which the rootfs does not have
# shellcheck disable=SC2086
apk add --no-cache --initdb --no-scripts --root "$rootfs" \
  --keys-dir /etc/apk/keys --repositories-file /etc/apk/repositories \
  ca-certificates-bundle $deps

# Some dependencies (e.g. ca-certificates) pull busybox for their scripts:
# drop all executables and the package database.
rm -rf "$rootfs/bin" "$rootfs/sbin" "$rootfs/usr/bin" "$rootfs/usr/sbin" \
  "$rootfs/lib/apk" "$rootfs/etc/apk" "$rootfs/var/cache" "$rootfs/usr/share/apk"

# Non-root user, shared by the ArmoniK images (UID/GID 5000)
mkdir -p "$rootfs/home/armonikuser" "$rootfs/cache" "$rootfs/tmp"
chmod 1777 "$rootfs/tmp"
printf 'root:x:0:0:root:/root:/sbin/nologin\narmonikuser:x:5000:5000::/home/armonikuser:/sbin/nologin\n' > "$rootfs/etc/passwd"
printf 'root:x:0:\narmonikuser:x:5000:\n' > "$rootfs/etc/group"
chown 5000:5000 "$rootfs/home/armonikuser" "$rootfs/cache"
