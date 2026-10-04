#!/usr/bin/env bash
# D19 build impact: the core now compiles C++ (simdutf, through the `simdutf` crate's cc
# build) and libak_core.so links libstdc++. Every way a slice builds or links the core, run
# against the committed core (git archive HEAD ffi/poc/codec), nothing timed:
#
#   1  each slice's cargo feature sets for ak-core (union of poc/rust gen, poc/cpp CMakeLists,
#      poc/csharp gen/build_core.sh, poc/java gen/build.sh, poc/python build.sh), in one
#      scratch target directory; per set: built, the 7 D19 exports present, libstdc++ in DT_NEEDED
#   2  the h2-batch cores: poc/codec/h2-batch/build.sh, a --config patch build (C#/Python/C++
#      "by hand" form), and poc/rust/gen/h2batch_core.sh
#   3  the C hosts' link forms against the cdylib, with gcc: the Java JNI shim (java/native,
#      JDK headers), the Python shim (python/native/binding.c, CPython 3.12 headers); and a
#      small C program that calls ak_tc_utf16 and the exports through the .so and checks bytes
#   4  the staticlib: rustc's native-static-libs; a C program linked with gcc against
#      libak_core.a WITHOUT and WITH -lstdc++; the same with g++
#   5  the 1.88.0 floor toolchain builds the core
#
# The C++ slice's CMake targets and the C# slice's gen/build_core.sh are run separately
# (they write their own build trees); see the log index in STATE.md.
#
#   gen/d19_build_matrix.sh <scratch dir>
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SLICE="$(cd "$HERE/.." && pwd)"
POC="$(cd "$SLICE/.." && pwd)"
REPO="$(git -C "$SLICE" rev-parse --show-toplevel)"
S="${1:?usage: gen/d19_build_matrix.sh <scratch dir>}"
mkdir -p "$S"; S="$(cd "$S" && pwd)"
SNAP="$S/snap"; rm -rf "$SNAP"; mkdir -p "$SNAP"
git -C "$REPO" archive HEAD ffi/poc/codec | tar -x -C "$SNAP"
CORE="$SNAP/ffi/poc/codec"
TD="$S/target"
fails=0
ok()  { echo "  ok    $*"; }
bad() { echo "  FAIL  $*"; fails=$((fails+1)); }
D19='ak_(utf16_to_utf8|utf16_utf8_len|utf8_to_utf16|utf8_utf16_len|utf8_validate|utf16_validate|tc_utf16_scalar)$'

echo "# commit $(git -C "$REPO" rev-parse --short HEAD)$(git -C "$REPO" diff --quiet HEAD -- ffi/poc/codec || echo ' (poc/codec differs from HEAD: NOT what is built)'); rustc $(rustc --version | awk '{print $2}'); $(cc --version | head -1); $(g++ --version | head -1)"

echo "===== 1. ak-core feature sets (cargo build --release -p ak-core) ====="
build() {  # build <label> <cargo args...>
  local label=$1; shift
  if ( cd "$CORE" && CARGO_TARGET_DIR="$TD" cargo build --release -q -p ak-core "$@" ) > "$S/cargo.err" 2>&1; then
    local so="$TD/release/libak_core.so"
    local nd; nd=$(nm -D --defined-only "$so" | grep -cE " T $D19")
    local cxx; cxx=$(readelf -d "$so" | grep -c 'libstdc++')
    [ "$nd" = 7 ] && [ "$cxx" = 1 ] && ok "$label: $* -> $(nm -D --defined-only "$so" | grep -c ' T ak_') ak_* exports, 7 D19 exports, NEEDED libstdc++" \
      || bad "$label: $* -> D19 exports $nd, libstdc++ NEEDED $cxx"
  else
    bad "$label: $* did not build"; tail -20 "$S/cargo.err" | sed 's/^/        /'
  fi
}
build "default"
build "java/python/cpp plain"   --features init-guard
build "count"                   --features count,init-guard
build "corpus"                  --features corpus,init-guard
build "rpc"                     --features rpc,init-guard
build "rpc count"               --features rpc,count,init-guard
build "cpp AK_RPC"              --features rpc
build "cpp AK_RPC count"        --features rpc,count
build "cpp planted pad"         --features pad-widths,init-guard
build "cpp planted global"      --features global-widths,init-guard
build "cpp planted both"        --features pad-widths,global-widths,init-guard
build "nounk"                   --no-default-features --features init-guard
build "nounk count"             --no-default-features --features count,init-guard
build "nounk corpus"            --no-default-features --features corpus,init-guard
build "nounk rpc"               --no-default-features --features rpc,init-guard
build "nounk rpc count"         --no-default-features --features rpc,count,init-guard
build "dec-reject-simd"         --features dec-reject-simd,init-guard

echo "===== 2. h2-batch cores ====="
if AK_CARGO_PREFIX="" bash "$CORE/h2-batch/build.sh" h2-batch "$S/h2b" "rpc,init-guard" > "$S/h2b.out" 2>&1; then
  so="$S/h2b/release/libak_core.so"
  ok "codec/h2-batch/build.sh h2-batch rpc,init-guard -> $(nm -D --defined-only "$so" | grep -cE " T $D19") D19 exports, h2: $(strings "$so" | grep -o '[^/]*/src/codec/framed_write\.rs' | sort -u | tr '\n' ' ')"
  H2SRC="$S/h2b/h2-batch-src"
  cp "$CORE/Cargo.lock" "$S/Cargo.lock.saved"
  if ( cd "$CORE" && CARGO_TARGET_DIR="$S/h2b-hand" cargo build --release -q -p ak-core --no-default-features --features rpc,count,init-guard \
         --config "patch.crates-io.h2.path=\"$H2SRC\"" ) > "$S/cargo.err" 2>&1; then
    ok "by hand (--config patch), --no-default-features rpc,count,init-guard -> $(nm -D --defined-only "$S/h2b-hand/release/libak_core.so" | grep -cE " T $D19") D19 exports"
  else bad "h2-batch by hand"; tail -20 "$S/cargo.err"; fi
  cp "$S/Cargo.lock.saved" "$CORE/Cargo.lock"
else
  bad "codec/h2-batch/build.sh"; tail -20 "$S/h2b.out"
fi
if bash "$SLICE/gen/h2batch_core.sh" "$S/h2b-rust" > "$S/h2b-rust.out" 2>&1; then
  ok "poc/rust/gen/h2batch_core.sh (the working tree's poc/codec): $(sed -n '2,3p' "$S/h2b-rust.out" | tr '\n' ' ')"
else bad "poc/rust/gen/h2batch_core.sh"; tail -20 "$S/h2b-rust.out"; fi

echo "===== 3. C hosts linking the cdylib with gcc ====="
( cd "$CORE" && CARGO_TARGET_DIR="$TD" cargo build --release -q -p ak-core --features init-guard ) > /dev/null 2>&1
LIB="$TD/release"
JDK=$(dirname "$(dirname "$(readlink -f "$(command -v javac)")")")
if gcc -O2 -fPIC -shared -std=c11 -Wall -Wextra -Wno-unused-parameter -I"$JDK/include" -I"$JDK/include/linux" \
     -I"$POC/java/native/generated" -o "$S/libakjni.so" "$POC/java/native/generated/shim.c" "$POC/java/native/tax.c" \
     -L"$LIB" -lak_core -Wl,-rpath,"$LIB" 2> "$S/jni.err"; then
  ok "java JNI shim (gcc -shared, $JDK headers; the slice uses JDK 17's): links; ldd resolves $(ldd "$S/libakjni.so" | grep -c 'libstdc++\|libak_core') of libak_core + libstdc++"
else bad "java JNI shim"; cat "$S/jni.err"; fi
PY=python3.12
INC=$($PY -c 'import sysconfig;print(sysconfig.get_paths()["include"])'); PINC=$($PY -c 'import sysconfig;print(sysconfig.get_paths()["platinclude"])')
if cc -O2 -fPIC -Wall -Wextra -Werror -Wno-unused-parameter -fvisibility=hidden -shared -I"$INC" -I"$PINC" -I"$(dirname "$INC")" \
     -I"$POC/python/gen/out" -DAK_MODNAME_STR='"_akffi"' -DAK_INITFUNC=PyInit__akffi -o "$S/_akffi.so" "$POC/python/native/binding.c" \
     -L"$LIB" -lak_core -Wl,-rpath,"$LIB" 2> "$S/py.err"; then
  ok "python shim (cc -shared -Werror, CPython $($PY -c 'import sys;print(sys.version.split()[0])') headers): links"
  if ( cd "$S" && $PY -c 'import _akffi' ) > "$S/py-import.err" 2>&1; then ok "python: import _akffi loads the core (libstdc++ resolved)"; else bad "python import"; cat "$S/py-import.err"; fi
else bad "python shim"; cat "$S/py.err"; fi
cat > "$S/d19c.c" <<'EOF'
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
typedef int32_t (*ak_grow_fn)(void *, int32_t, uint8_t **, int32_t *);
typedef int32_t (*ak_transcode_fn)(const void *, size_t, uint8_t *, int32_t, ak_grow_fn, void *);
ak_transcode_fn ak_tc_utf16(void);
int32_t ak_utf16_to_utf8(const uint16_t *, size_t, uint8_t *, size_t);
int32_t ak_utf16_utf8_len(const uint16_t *, size_t);
int32_t ak_utf8_to_utf16(const uint8_t *, size_t, uint16_t *, size_t);
int32_t ak_utf8_utf16_len(const uint8_t *, size_t);
int32_t ak_utf8_validate(const uint8_t *, size_t);
int32_t ak_utf16_validate(const uint16_t *, size_t);
static int32_t no_grow(void *s, int32_t w, uint8_t **d, int32_t *c) { (void)s; (void)w; (void)d; (void)c; return -7; }
int main(void) {
  /* "aé€😀" then a lone high surrogate */
  const uint16_t u[] = {0x61, 0xE9, 0x20AC, 0xD83D, 0xDE00, 0xD800};
  const uint8_t want[] = {0x61, 0xC3, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9F, 0x98, 0x80, 0xEF, 0xBF, 0xBD};
  uint8_t out[32]; uint16_t back[16]; int bad = 0;
  int32_t n = ak_tc_utf16()(u, 6, out, sizeof out, no_grow, NULL);
  bad |= n != (int32_t)sizeof want || memcmp(out, want, sizeof want);
  n = ak_utf16_to_utf8(u, 6, out, sizeof out);
  bad |= n != (int32_t)sizeof want || memcmp(out, want, sizeof want);
  bad |= ak_utf16_utf8_len(u, 6) != 13 || ak_utf16_validate(u, 6) != -6 || ak_utf16_validate(u, 5) != 0;
  bad |= ak_utf16_to_utf8(u, 6, out, 12) != -7;
  n = ak_utf8_to_utf16(want, 10, back, 16);
  bad |= n != 5 || memcmp(back, u, 10) || ak_utf8_utf16_len(want, 10) != 5 || ak_utf8_validate(want, 13) != 0;
  bad |= ak_utf8_to_utf16((const uint8_t *)"\xED\xA0\x80", 3, back, 16) != -6 || ak_utf8_validate((const uint8_t *)"\xC0\x80", 2) != -6;
  printf("%s\n", bad ? "D19 C CHECK FAILED" : "D19 C CHECK PASSED");
  return bad;
}
EOF
if gcc -O2 -std=c11 -Wall -o "$S/d19c" "$S/d19c.c" -L"$LIB" -lak_core -Wl,-rpath,"$LIB" 2> "$S/c.err"; then
  r=$("$S/d19c"); [ "$r" = "D19 C CHECK PASSED" ] && ok "C program (gcc, dynamic): ak_tc_utf16 and the six exports, bytes checked: $r" || bad "C program: $r"
else bad "C program (gcc, dynamic) did not link"; cat "$S/c.err"; fi

echo "===== 4. the staticlib ====="
NSL=$( cd "$CORE" && CARGO_TARGET_DIR="$TD" cargo rustc --release -q -p ak-core --features init-guard --crate-type staticlib -- --print native-static-libs 2>&1 | grep -o 'native-static-libs: .*' | head -1 )
echo "  rustc $NSL"
if gcc -O2 -std=c11 -o "$S/d19c-static-nocxx" "$S/d19c.c" "$LIB/libak_core.a" -lpthread -ldl -lm 2> "$S/static.err"; then
  ok "gcc static WITHOUT -lstdc++ links (unexpected)"
else
  echo "  info  gcc static WITHOUT -lstdc++: does not link ($(grep -c 'undefined reference' "$S/static.err") undefined references, e.g. $(grep -o "undefined reference to \`[^']*'" "$S/static.err" | head -2 | tr '\n' ' '))"
fi
if gcc -O2 -std=c11 -o "$S/d19c-static" "$S/d19c.c" "$LIB/libak_core.a" -lstdc++ -lpthread -ldl -lm 2> "$S/static.err"; then
  r=$("$S/d19c-static"); [ "$r" = "D19 C CHECK PASSED" ] && ok "gcc static WITH -lstdc++: $r" || bad "gcc static: $r"
else bad "gcc static WITH -lstdc++"; cat "$S/static.err"; fi
cp "$S/d19c.c" "$S/d19cxx.cpp"
if g++ -O2 -std=c++11 -x c++ -o "$S/d19cxx-static" "$S/d19cxx.cpp" "$LIB/libak_core.a" -lpthread -ldl -lm 2> "$S/static.err"; then
  r=$("$S/d19cxx-static"); [ "$r" = "D19 C CHECK PASSED" ] && ok "g++ static (the C++ slice's driver): $r" || bad "g++ static: $r"
else bad "g++ static"; head -20 "$S/static.err"; fi

echo "===== 5. the floor toolchain ====="
if ( cd "$CORE" && RUSTUP_TOOLCHAIN=1.88.0 CARGO_TARGET_DIR="$S/target-floor" cargo build --release -q -p ak-core --features rpc,init-guard ) > "$S/cargo.err" 2>&1; then
  ok "rustc $(RUSTUP_TOOLCHAIN=1.88.0 rustc --version | awk '{print $2}'): --features rpc,init-guard builds, $(nm -D --defined-only "$S/target-floor/release/libak_core.so" | grep -cE " T $D19") D19 exports"
else bad "1.88.0"; tail -20 "$S/cargo.err"; fi
if ( cd "$CORE" && RUSTUP_TOOLCHAIN=1.88.0 CARGO_TARGET_DIR="$S/target-floor-test" cargo test -q -p ak-core -p ak-rt ) > "$S/floor-test.out" 2>&1; then
  ok "rustc 1.88.0: ak-core / ak-rt unit tests: $(grep -E 'test result' "$S/floor-test.out" | awk '{s+=$4; f+=$6} END {print s" passed, "f" failed"}')"
else bad "1.88.0 unit tests"; tail -20 "$S/floor-test.out"; fi

echo
echo "sizes: libak_core.so (init-guard) $(stat -c %s "$LIB/libak_core.so") B, libak_core.a $(stat -c %s "$LIB/libak_core.a") B"
echo "failures: $fails"
[ "$fails" = 0 ] && echo "D19 BUILD MATRIX PASSED" || echo "D19 BUILD MATRIX FAILED"
