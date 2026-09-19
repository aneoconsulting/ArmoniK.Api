#!/usr/bin/env bash
# Build the shared core (README R0), for a slice that does not build it itself.
#
# The rust, cpp and java slices each build this crate through their own build, into their
# own CARGO_TARGET_DIR, with their own features -- so they do not call this. The csharp
# slice loads `libak_core.so` as a file and never built it (it used to load the rust
# slice's); this is what produces the file it loads. A python core arm, when there is one,
# is the other caller.
#
# Default features: no `rpc`, so the shared object carries the codec and nothing else.
# Pass --features rpc for ABI v1 section 9.
set -eu
cd "$(dirname "$0")/.."
cargo build --release -p ak-core "$@"
echo
ls -la target/release/libak_core.so target/release/libak_core.a
echo
echo "exported ak_* entry points: $(nm -D --defined-only target/release/libak_core.so | grep -cE ' T ak_')"
