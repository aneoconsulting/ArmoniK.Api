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

# ---- 2. the core, behind the C ABI. THE shared core (R0), not a copy.
# Built from a SNAPSHOT of the committed `ffi/poc/codec` (git archive of $AK_CORE_REV,
# default HEAD) rather than in place, because other slices regenerate the core in the same
# working tree concurrently; `AK_CODEC=<dir>` builds another tree instead. The snapshot's
# commit is printed and belongs in every log.
# Every codec build carries `init-guard` (R-G7): ABI v1 section 3 as specified, so a binding
# that skipped `ak_init` fails here instead of passing silently.
#
# D39: the cargo target directories are KEYED on what is built. `git archive` stamps every
# file with the commit's time, so over a reused target dir cargo could see an older mtime
# than the artifacts it already had and keep a stale core -- it did once. Each snapshot now
# builds into core-build/<key>/, key = the git TREE hash of ffi/poc/codec at $REV (or, for
# AK_CODEC, a hash of that directory's sources), and core-build/current points at the one
# this build made. A key that already exists was built from exactly those sources.
REV=${AK_CORE_REV:-HEAD}
if [ -n "${AK_CODEC:-}" ]; then
  CODEC=$AK_CODEC
  KEY=dir-$(cd "$CODEC" && find . -type f ! -path './target/*' ! -path '*/__pycache__/*' \
              | LC_ALL=C sort | xargs sha256sum | sha256sum | cut -c1-16)
  echo "   core: AK_CODEC=$CODEC, source key $KEY" | tee build/core-rev.txt
else
  # The snapshot keeps the ffi/ layout (poc/codec beside schema/ and corpus/), because the
  # generator in poc/codec/gen reads the descriptions relative to itself.
  SNAP=$HERE/build/snap
  rm -rf "$SNAP" && mkdir -p "$SNAP"
  TOP=$(git rev-parse --show-toplevel)
  # Every slice's gen/*.py too: poc/codec/gen/generate.py --check guards every
  # poc/<slice>/gen/ module (R-H13), and a snapshot without them fails that guard.
  ( cd "$TOP" && git archive "$REV" ffi/poc/codec ffi/schema ffi/corpus \
      $(git ls-tree -r --name-only "$REV" ffi/poc | grep -E '^ffi/poc/[^/]+/gen/[^/]+\.py$' | grep -v '^ffi/poc/codec/') ) \
    | tar -x -C "$SNAP"
  CODEC=$SNAP/ffi/poc/codec
  export AK_CODECGEN=${AK_CODECGEN:-$CODEC/gen}   # a preset one (development) wins
  KEY=tree-$(cd "$TOP" && git rev-parse "$REV:ffi/poc/codec" | cut -c1-16)
  echo "   core snapshot: ffi/poc/codec at $(git rev-parse --short "$REV"), tree key $KEY" | tee build/core-rev.txt
fi
# ---- 1. the generator: glue over poc/codec/gen's Java backend (FIX-PLAN WP5 step 3), from
# the SAME snapshot as the core, so the binding and the core come from one generator state
# even while another agent has poc/codec mid-change in the working tree.
say "generate (poc/codec/gen from ${AK_CODECGEN:-the working tree})"
python3 gen/generate.py

CB=core-build/$KEY
mkdir -p "$CB"
ln -sfn "$KEY" core-build/current
CORE=$CODEC/crates/ak-core/Cargo.toml
say "core (timed, init-guard)"
CARGO_TARGET_DIR=$HERE/$CB/target cargo build --release --features init-guard \
  --manifest-path $CORE >/dev/null
say "core (counting, init-guard)"
CARGO_TARGET_DIR=$HERE/$CB/target-count cargo build --release --features count,init-guard \
  --manifest-path $CORE >/dev/null
# The core generated for ffi/corpus's reader schema (test-only `corpus` feature: it changes
# the ABI, so it is its own build and its own shim, never linked with a shapes host).
say "core (corpus schema, init-guard)"
CARGO_TARGET_DIR=$HERE/$CB/target-corpus cargo build --release --features corpus,init-guard \
  --manifest-path $CORE >/dev/null

# The NO-UNKNOWN build (FIX-PLAN WP5 step 10): ak-core without its default `unknown-fields`
# feature (plan.py, THE NO-UNKNOWN VARIANT), each in a target dir of its own (a variant
# build over a shared dir overwrites libak_core.so for the other variant's shims).
NOUNK=(--no-default-features)
say "core (no-unknown: timed, counting, corpus schema; init-guard)"
CARGO_TARGET_DIR=$HERE/$CB/target-nounk cargo build --release "${NOUNK[@]}" --features init-guard \
  --manifest-path $CORE >/dev/null
CARGO_TARGET_DIR=$HERE/$CB/target-count-nounk cargo build --release "${NOUNK[@]}" --features count,init-guard \
  --manifest-path $CORE >/dev/null
CARGO_TARGET_DIR=$HERE/$CB/target-corpus-nounk cargo build --release "${NOUNK[@]}" --features corpus,init-guard \
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
shim jni       $CB/target        native/generated
shim jnicnt    $CB/target-count  native/generated
shim jnong     $CB/target        native/generated         -DAK_NO_GUARD
shim jnitax    $CB/target        native/generated         -DAK_CROSSING_TAX
shim jnicorpus $CB/target-corpus native/generated_corpus
# The no-unknown build's shims, over c_abi's second header (native/generated*_nounk).
shim jni-nounk       $CB/target-nounk        native/generated_nounk
shim jnicnt-nounk    $CB/target-count-nounk  native/generated_nounk
shim jnicorpus-nounk $CB/target-corpus-nounk native/generated_corpus_nounk
# Each shim must resolve to THIS build's core, never another key's -- and to the core of ITS
# variant: a full core exports the u-family (ak_uencode_*), a no-unknown core none.
for s in jni jnicnt jnong jnitax jnicorpus jni-nounk jnicnt-nounk jnicorpus-nounk; do
  ldd build/$s/libakjni.so | grep -q "$HERE/$CB/" \
    || { echo "shim $s does not link $CB: $(ldd build/$s/libakjni.so | grep ak_core)"; exit 1; }
  so=$(ldd build/$s/libakjni.so | awk '/libak_core/ {print $3}')
  nu=$(nm -D --defined-only "$so" | grep -c ' T ak_uencode_' || true)
  case $s in *-nounk) [ "$nu" = 0 ] || { echo "shim $s links a core WITH the u-family ($nu)"; exit 1; } ;;
             *)       [ "$nu" -gt 0 ] || { echo "shim $s links a core WITHOUT the u-family"; exit 1; } ;; esac
done

# ---- 3b. ABI v1 section 9, the RPC half. A SEPARATE core build (the `rpc` feature links
# tonic and tokio, which a codec arm must not carry). Not in the correctness gate.
say "core (rpc feature) and its shim"
CARGO_TARGET_DIR=$HERE/$CB/target-rpc cargo build --release --features rpc,init-guard \
  --manifest-path $CORE >/dev/null
mkdir -p build/jnirpc
gcc -O2 -fPIC -shared -std=c11 -Wall -Wextra -Wno-unused-parameter \
    -I"$J17/include" -I"$J17/include/linux" -Inative/generated \
    -o build/jnirpc/libakjni.so native/generated/shim.c native/tax.c native/rpc.c \
    -L"$CB/target-rpc/release" -lak_core \
    -Wl,-rpath,"$HERE/$CB/target-rpc/release"
CARGO_TARGET_DIR=$HERE/$CB/target-rpc-nounk cargo build --release "${NOUNK[@]}" --features rpc,init-guard \
  --manifest-path $CORE >/dev/null
mkdir -p build/jnirpc-nounk
gcc -O2 -fPIC -shared -std=c11 -Wall -Wextra -Wno-unused-parameter \
    -I"$J17/include" -I"$J17/include/linux" -Inative/generated_nounk \
    -o build/jnirpc-nounk/libakjni.so native/generated_nounk/shim.c native/tax.c native/rpc.c \
    -L"$CB/target-rpc-nounk/release" -lak_core \
    -Wl,-rpath,"$HERE/$CB/target-rpc-nounk/release"

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

# ---- 5a. the no-unknown build's classes (WP5 step 10): the same hand-written sources over
# the tree generated from the plan relowered with unknown="drop", minus the two retain-only
# controls. Same package names as the full tree, so a class tree of its own.
say "classes: the no-unknown build (java17 on JDK 17)"
rm -rf build/cls17-nounk && mkdir -p build/cls17-nounk
"$J17/bin/javac" -nowarn -encoding UTF-8 -d build/cls17-nounk -cp "$CP" \
  $(find src/java src/generated_nounk/java17 src/generated_nounk/shared src/generated_corpus_nounk/java17 \
       src/generated_corpus_nounk/shared -name '*.java' ! -name 'Pin.java' ! -name 'RunUnkControls.java' ! -name 'RunUnkLeak.java' ! -name 'RunUnkOneof.java') \
  $(find build/pbjava -name '*.java')

# ---- 5b. the codec suite on JMH (CAMPAIGN.md req 22a), target only: the annotation
# processor generates the benchmark stubs and META-INF/BenchmarkList into build/jmh17.
if [ -f deps/jmh/cp.txt ]; then
  say "JMH codec suite (JDK 17)"
  JMHCP=$(cat deps/jmh/cp.txt)
  rm -rf build/jmh17 && mkdir -p build/jmh17
  "$J17/bin/javac" -nowarn -encoding UTF-8 -d build/jmh17 -cp "build/cls17:$CP:$JMHCP" \
    -processorpath "$JMHCP" src/jmh/ak/*.java
  rm -rf build/jmh17-nounk && mkdir -p build/jmh17-nounk
  "$J17/bin/javac" -nowarn -encoding UTF-8 -d build/jmh17-nounk -cp "build/cls17-nounk:$CP:$JMHCP" \
    -processorpath "$JMHCP" src/jmh/ak/*.java
else
  say "JMH: deps/jmh/cp.txt missing (cd deps/jmh && mvn dependency:build-classpath -Dmdep.outputFile=cp.txt)"
fi

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
       ! -name 'Ffm*.java' ! -name 'Pin.java' ! -name 'RunR14.java' \
       ! -name 'Campaign*.java') \
  $(find build/pbjava -name '*.java')
say "classes: the no-unknown build (java8 source tree, JDK 8 javac)"
rm -rf build/cls8-nounk && mkdir -p build/cls8-nounk
"$J8/bin/javac" -nowarn -encoding UTF-8 -source 8 -target 8 -d build/cls8-nounk -cp "$CP" \
  $(find src/java src/generated_nounk/java8 src/generated_nounk/shared src/generated_corpus_nounk/java8 \
       src/generated_corpus_nounk/shared -name '*.java' \
       ! -name 'Ffm*.java' ! -name 'Pin.java' ! -name 'RunR14.java' \
       ! -name 'Campaign*.java' ! -name 'RunUnkControls.java' ! -name 'RunUnkLeak.java' ! -name 'RunUnkOneof.java') \
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
"$J17/bin/javac" -nowarn -d build/probe probe/Probe.java probe/CampaignRev.java

# The shim-primitive probe: what a generated C shim would pay per JNI accessor, priced
# before the arm that would depend on it is built. Its own .so, so `crossing.log`'s
# artifact does not move.
gcc -O2 -fPIC -shared -I"$J17/include" -I"$J17/include/linux" \
  -o build/probe/libshimprobe.so probe/shimprobe.c
"$J17/bin/javac" -nowarn -d build/probe probe/ShimProbe.java

say "done"
