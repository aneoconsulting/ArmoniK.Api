#!/usr/bin/env bash
# README R5's second half: an arm is not what its name says until the ARTIFACT agrees.
#
# The rust slice's FFI arm turned out to be the no-boundary control with extra struct
# copies, because rustc inlined every `extern "C"` entry point -- and the counting build
# kept reporting the right counts, because the counting code inlined with the bodies. R5
# now requires a slice to show from the built artifact that the entry points are unresolved
# imports.
#
# The JVM is the one host this hazard cannot reach: a managed caller cannot inline across
# JNI. That is a reason to check cheaply rather than a reason not to check, because the
# shim is C and the core is a cdylib and THAT pair can be got exactly the way the rust
# slice's was, if anyone ever links them statically.
set -eu
cd "$(dirname "$0")/.."
echo "== R5: the boundary, from the built artifacts =="
echo
echo "# the shim's undefined imports: every ABI entry point it calls must be resolved by"
echo "# the core at load, not inlined into it"
nm -D --undefined-only build/jni/libakjni.so | grep -E ' U ak_' | sort | sed 's/^/  /'
echo
echo "# and the core defines them"
nm -D --defined-only core-build/current/target/release/libak_core.so | grep -cE ' T ak_' \
  | sed 's/^/  exported ak_* symbols: /'
echo
echo "# the shim carries no copy of the codec: no ak_encode_/ak_decode_ TEXT symbols"
if nm -D --defined-only build/jni/libakjni.so | grep -qE ' T ak_(encode|decode|elem)'; then
  echo "  FAIL: the shim defines a codec entry point, so it is not calling the core"
  exit 1
fi
echo "  ok"
echo
echo "# the JNI natives the JVM will bind, which is the other direction of the same check:"
echo "# a method the shim does not define is an UnsatisfiedLinkError at load, never a"
echo "# silently skipped arm"
nm -D --defined-only build/jni/libakjni.so | grep -c ' T Java_ak_' \
  | sed 's/^/  JNI entry points: /'
