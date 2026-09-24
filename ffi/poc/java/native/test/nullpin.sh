#!/usr/bin/env bash
# Build and run the R-D9 fault injection against a given shim source.
#   native/test/nullpin.sh [shim.c]   (default: native/generated/shim.c)
set -eu
cd "$(dirname "$0")/../.."
J17=${J17:-/usr/lib/jvm/java-17-openjdk-amd64}
SHIM=${1:-native/generated/shim.c}
mkdir -p build/test
gcc -O1 -std=c11 -Wall -Wno-unused-parameter -Wno-unused-function \
    -I"$J17/include" -I"$J17/include/linux" -Inative/generated \
    -Wl,--wrap=ak_encode_UploadResultDataMessage \
    -o build/test/nullpin native/test/nullpin.c "$SHIM" \
    -Lcore-build/target/release -lak_core -Wl,-rpath,"$PWD/core-build/target/release"
echo "shim under test: $SHIM ($(sha256sum "$SHIM" | cut -c1-16))"
./build/test/nullpin
