#!/usr/bin/env bash
# Build every artifact this slice measures, in dependency order.
#
# Not a Maven lifecycle: the measured artifacts are plain class files and one .so, and a
# build tool between them and the harness is one more thing a figure would have to be
# traced through. `deps/pom.xml` resolves the incumbent's classpath and nothing else.
set -eu
cd "$(dirname "$0")/.."
HERE=$PWD
J17=${J17:-/usr/lib/jvm/java-17-openjdk-amd64}
J8=${J8:-/usr/lib/jvm/java-8-openjdk-amd64}
unset JAVA_TOOL_OPTIONS || true
CP=$(cat deps/cp.txt)
mkdir -p build

say() { echo "== $*"; }

# ---- 1. the generator, from one description (R1)
say "generate"
python3 gen/generate.py

# ---- 2. the core, behind the C ABI. Two builds: timed, and counting (R5).
# THE shared core (R0): ../codec/crates/ak-core, the same crate the rust, cpp and
# csharp slices build, not a copy. This slice contributed `ak_tc_utf16` and
# `ak_tc_latin1` to it -- the two converting transcoders ABI v1 section 4 specifies for
# a host that holds UTF-16 -- and gets the cpp slice's `ak_enc_count_reverse` back.
# CARGO_TARGET_DIR keeps this slice's two builds out of the shared workspace's target
# directory, which every other slice is also building into.
CORE=../codec/crates/ak-core/Cargo.toml
say "core (timed)"
CARGO_TARGET_DIR=$HERE/core-build/target cargo build --release --manifest-path $CORE \
  >/dev/null
say "core (counting)"
CARGO_TARGET_DIR=$HERE/core-build/target-count cargo build --release --features count \
  --manifest-path $CORE >/dev/null

# ---- 3. the JNI shim, one per core build, plus a no-guard and a tax build
say "shim"
shim() {   # $1 = output dir, $2 = core target dir, $3... = extra cflags
  local out=$1 core=$2; shift 2
  mkdir -p "build/$out"
  gcc -O2 -fPIC -shared -std=c11 -Wall -Wextra -Wno-unused-parameter \
      -I"$J17/include" -I"$J17/include/linux" -Inative/generated \
      "$@" -o "build/$out/libakjni.so" native/generated/shim.c native/tax.c \
      -L"$core/release" -lak_core -Wl,-rpath,"$HERE/$core/release"
}
shim jni     core-build/target
shim jnicnt  core-build/target-count
shim jnong   core-build/target       -DAK_NO_GUARD
shim jnitax  core-build/target       -DAK_CROSSING_TAX

# ---- 4. the incumbent's generated Java
say "protoc"
mkdir -p build/pbjava
./build/tools/protoc-3.19.0 --java_out=build/pbjava -I proto proto/shapes.proto

# ---- 5. arm a: the JDK 17 implementation on the JDK 17 runtime (README 5.2)
say "classes: arm a (java17 on JDK 17)"
mkdir -p build/cls17
"$J17/bin/javac" -nowarn -encoding UTF-8 -d build/cls17 -cp "$CP" \
  -sourcepath "src/java:src/generated/java17:src/generated/shared:build/pbjava" \
  $(find src/java src/generated/java17 src/generated/shared -name '*.java') \
  $(find build/pbjava -name '*.java')

# ---- 6. arm b and c: the Java 8 implementation. Same sources, compiled at release 8.
# Compiled by the JDK 8 compiler, not by `--release 8` on JDK 17. `--release` builds
# against ct.sym, which does not carry `sun.misc.Unsafe` -- an undocumented API that the
# floor and the target both use and that protobuf-java uses too. A floor that cannot see
# it is not the floor a Java 8 consumer has. This is itself the first packaging finding:
# see STATE.md.
say "classes: arms b and c (java8 source tree, JDK 8 javac)"
mkdir -p build/cls8
"$J8/bin/javac" -nowarn -encoding UTF-8 -source 8 -target 8 -d build/cls8 -cp "$CP" \
  -sourcepath "src/java:src/generated/java8:src/generated/shared:build/pbjava" \
  $(find src/java src/generated/java8 src/generated/shared -name '*.java' \
       ! -name 'Ffm*.java') \
  $(find build/pbjava -name '*.java')

say "done"
