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

# ---- 1. the generator: glue over poc/codec/gen's Java backend (FIX-PLAN WP5 step 3).
# It writes under poc/java only; the core's generated files are poc/codec/gen's.
say "generate"
python3 gen/generate.py

# ---- 2. the core, behind the C ABI. THE shared core (R0), not a copy.
# Built from a SNAPSHOT of the committed `ffi/poc/codec` (git archive of $AK_CORE_REV,
# default HEAD) rather than in place, because other slices regenerate the core in the same
# working tree concurrently; `AK_CODEC=<dir>` builds another tree instead. The snapshot's
# commit is printed and belongs in every log.
# Every codec build carries `init-guard` (R-G7): ABI v1 section 3 as specified, so a binding
# that skipped `ak_init` fails here instead of passing silently.
REV=${AK_CORE_REV:-HEAD}
if [ -n "${AK_CODEC:-}" ]; then
  CODEC=$AK_CODEC
else
  CODEC=$HERE/build/codec-snap
  rm -rf "$CODEC" && mkdir -p "$CODEC"
  ( cd "$(git rev-parse --show-toplevel)" && git archive "$REV" ffi/poc/codec ) \
    | tar -x -C "$CODEC" --strip-components=3
  echo "   core snapshot: ffi/poc/codec at $(git rev-parse --short "$REV")" | tee build/core-rev.txt
fi
CORE=$CODEC/crates/ak-core/Cargo.toml
say "core (timed, init-guard)"
CARGO_TARGET_DIR=$HERE/core-build/target cargo build --release --features init-guard \
  --manifest-path $CORE >/dev/null
say "core (counting, init-guard)"
CARGO_TARGET_DIR=$HERE/core-build/target-count cargo build --release --features count,init-guard \
  --manifest-path $CORE >/dev/null
# The core generated for ffi/corpus's reader schema (test-only `corpus` feature: it changes
# the ABI, so it is its own build and its own shim, never linked with a shapes host).
say "core (corpus schema, init-guard)"
CARGO_TARGET_DIR=$HERE/core-build/target-corpus cargo build --release --features corpus,init-guard \
  --manifest-path $CORE >/dev/null

# ---- 3. the JNI shim, one per core build, plus a no-guard and a tax build
say "shim"
shim() {   # $1 = output dir, $2 = core target dir, $3 = generated native dir, $4... = cflags
  local out=$1 core=$2 gen=$3; shift 3
  mkdir -p "build/$out"
  gcc -O2 -fPIC -shared -std=c11 -Wall -Wextra -Wno-unused-parameter \
      -I"$J17/include" -I"$J17/include/linux" -I"$gen" \
      "$@" -o "build/$out/libakjni.so" "$gen/shim.c" native/tax.c \
      -L"$core/release" -lak_core -Wl,-rpath,"$HERE/$core/release"
}
shim jni       core-build/target        native/generated
shim jnicnt    core-build/target-count  native/generated
shim jnong     core-build/target        native/generated         -DAK_NO_GUARD
shim jnitax    core-build/target        native/generated         -DAK_CROSSING_TAX
shim jnicorpus core-build/target-corpus native/generated_corpus

# ---- 3b. ABI v1 section 9, the RPC half. A SEPARATE core build (the `rpc` feature links
# tonic and tokio, which a codec arm must not carry). Not in the correctness gate.
say "core (rpc feature) and its shim"
CARGO_TARGET_DIR=$HERE/core-build/target-rpc cargo build --release --features rpc,init-guard \
  --manifest-path $CORE >/dev/null
mkdir -p build/jnirpc
gcc -O2 -fPIC -shared -std=c11 -Wall -Wextra -Wno-unused-parameter \
    -I"$J17/include" -I"$J17/include/linux" -Inative/generated \
    -o build/jnirpc/libakjni.so native/generated/shim.c native/tax.c native/rpc.c \
    -L"core-build/target-rpc/release" -lak_core \
    -Wl,-rpath,"$HERE/core-build/target-rpc/release"

# ---- 4. the incumbent's generated Java
say "protoc"
# Fetched if absent, so a clean tree builds. `build/` is gitignored and deleting it is the
# right way to be sure a stale artifact is not being linked -- which is exactly what R5's
# hazard is about -- so the build has to be able to put it back.
PROTOC=build/tools/protoc-3.19.0
M2PROTOC=$HOME/.m2/repository/com/google/protobuf/protoc/3.19.0/protoc-3.19.0-linux-x86_64.exe
if [ ! -x "$PROTOC" ] && [ -f "$M2PROTOC" ]; then
  mkdir -p build/tools && cp "$M2PROTOC" "$PROTOC" && chmod +x "$PROTOC"
fi
if [ ! -x "$PROTOC" ]; then
  mkdir -p build/tools
  curl -sSf --max-time 180 -o "$PROTOC" \
    "https://repo1.maven.org/maven2/com/google/protobuf/protoc/3.19.0/protoc-3.19.0-linux-x86_64.exe"
  chmod +x "$PROTOC"
fi
mkdir -p build/pbjava
"$PROTOC" --java_out=build/pbjava -I proto proto/shapes.proto

# ---- 5. arm a: the JDK 17 implementation on the JDK 17 runtime (README 5.2)
say "classes: arm a (java17 on JDK 17)"
mkdir -p build/cls17
"$J17/bin/javac" -nowarn -encoding UTF-8 -d build/cls17 -cp "$CP" \
  -sourcepath "src/java:src/generated/java17:src/generated/shared:src/generated_corpus/java17:src/generated_corpus/shared:build/pbjava" \
  $(find src/java src/generated/java17 src/generated/shared src/generated_corpus/java17 \
       src/generated_corpus/shared -name '*.java' ! -name 'Pin.java') \
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
  -sourcepath "src/java:src/generated/java8:src/generated/shared:src/generated_corpus/java8:src/generated_corpus/shared:build/pbjava" \
  $(find src/java src/generated/java8 src/generated/shared src/generated_corpus/java8 \
       src/generated_corpus/shared -name '*.java' \
       ! -name 'Ffm*.java' ! -name 'Pin.java' ! -name 'RunR14.java') \
  $(find build/pbjava -name '*.java')

# ---- 7. the two secondary probes, both newer than the target and built separately
# `ak.Pin` uses virtual threads (JDK 21) and `FfmProbe` uses java.lang.foreign (JDK 22,
# preview on 21), so neither belongs in a JDK 17 or a Java 8 compilation. Both are
# secondary arms and README section 5 says so.
say "secondary probes (JDK 21)"
J21=${J21:-/usr/lib/jvm/java-21-openjdk-amd64}
mkdir -p build/ffm build/pin build/probe
gcc -O2 -fPIC -shared -I"$J21/include" -I"$J21/include/linux" \
  -o build/pin/libakpin.so native/pin.c -lpthread
gcc -O2 -fPIC -shared -I"$J17/include" -I"$J17/include/linux" \
  -o build/probe/libprobe.so probe/probe.c
"$J21/bin/javac" --release 21 --enable-preview -nowarn -d build/ffm probe/FfmProbe.java
"$J21/bin/javac" -nowarn -d build/pin src/java/ak/Pin.java
"$J17/bin/javac" -nowarn -d build/probe probe/Probe.java

# The shim-primitive probe: what a generated C shim would pay per JNI accessor, priced
# before the arm that would depend on it is built. Its own .so, so `crossing.log`'s
# artifact does not move.
gcc -O2 -fPIC -shared -I"$J17/include" -I"$J17/include/linux" \
  -o build/probe/libshimprobe.so probe/shimprobe.c
"$J17/bin/javac" -nowarn -d build/probe probe/ShimProbe.java

say "done"
