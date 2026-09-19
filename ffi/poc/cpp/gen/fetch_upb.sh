#!/usr/bin/env bash
# Fetch and build upb from the PROTOBUF REPOSITORY at a pinned tag.
#
# Not apt's `libupb-dev`: that is 0.0.0~git200730, a July 2020 snapshot from before upb
# was merged into protobuf, with a different API and different codegen. Measuring it and
# calling it the fastest available C protobuf would be a statement about a library nobody
# ships.
#
# Not Bazel either. `protoc-gen-upb` is Bazel-only and a hand-written minitable would be a
# hand-written codec (R1). This arm needs neither: the minitables come from a
# `upb_DefPool` loaded from the descriptor set `protoc` emits, and `upb_Encode` and
# `upb_Decode` work off a minitable whatever produced it, so the CODEC being timed is
# upb's own.
#
# upb ships a `upb/cmake/CMakeLists.txt`, and at v25.3 it is a STUB: four INTERFACE
# libraries referring to targets (`mem`, `reflection_internal`, ...) it never defines, so
# `ninja` there reports "no work to do". So the runtime is compiled here from upstream's
# own sources -- a file list, not a reimplementation. Excluded: tests, conformance, the
# JSON and text printers, and `util`, none of which this arm calls.
set -eu
cd "$(dirname "$0")/.." || exit 2
TAG=${AK_UPB_TAG:-v25.3}
SRC=build/upb-src
# AK_UPB_VARIANT picks the build: "" (the default, UPB_FASTTABLE=0, gcc),
# "clang" (UPB_FASTTABLE=0, clang), or "ft" (UPB_FASTTABLE=1, clang -- clang is required
# because UPB_MUSTTAIL needs __attribute__((musttail)), which gcc 13 does not have).
VARIANT=${AK_UPB_VARIANT:-}
case "$VARIANT" in
  ft)    OUT=build/upb-build-ft;    CC_=${AK_UPB_CC:-clang}; EXTRA="-DUPB_ENABLE_FASTTABLE" ;;
  clang) OUT=build/upb-build-clang; CC_=${AK_UPB_CC:-clang}; EXTRA="" ;;
  *)     OUT=build/upb-build;       CC_=${AK_UPB_CC:-cc};    EXTRA="" ;;
esac
if [ ! -d "$SRC/.git" ]; then
  rm -rf "$SRC"
  git clone --quiet --depth 1 --branch "$TAG" \
      https://github.com/protocolbuffers/protobuf.git "$SRC"
fi
echo "upb source: protobuf $TAG at $(git -C "$SRC" rev-parse --short HEAD)"

mkdir -p "$OUT"
SRCS=$(cd "$SRC" && find upb third_party/utf8_range -name '*.c' \
        ! -name '*test*' ! -path '*conformance*' ! -path 'upb/json/*' \
        ! -path 'upb/util/*' ! -path 'upb/io/*' ! -path 'upb/lex/*' \
        ! -path 'upb/text/*' ! -path 'upb/cmake/*' \
        ! -path 'third_party/utf8_range/utf8_to_utf16/*' \
        ! -name 'main.c' ! -name 'lemire-*' ! -name 'lookup.c' \
        ! -name 'range-*' ! -name 'range2-neon.c' | sort)
# utf8_range ships several interchangeable implementations of the same symbol, one per
# instruction set, and Bazel picks one. `range2-sse.c` is the SSE one upb's own build
# selects on x86-64; the rest would be duplicate definitions.
echo "compiling $(echo "$SRCS" | wc -l) upstream upb .c files"
rm -f "$OUT"/*.o "$OUT/libupb.a"
for f in $SRCS; do
  o="$OUT/$(echo "$f" | tr '/' '_').o"
  # -mssse3 for utf8_range's SSE path (upstream builds it with the right -m flags via
  # Bazel); the include roots reproduce Bazel's, where `upb/...` resolves from the repo
  # root and the bootstrap descriptor lives under upb/cmake.
  # -msse4.1 for utf8_range's SSE path (upstream passes the -m flags through Bazel).
  # stage0 FIRST on the include path: it is the BOOTSTRAP descriptor accessor set upb
  # uses to build reflection without protoc-gen-upb, and `upb/cmake` carries a second,
  # stale copy of the same header whose symbol spelling does not match its own .c.
  $CC_ -std=gnu99 -O2 -DNDEBUG -fPIC -msse4.1 $EXTRA -c "$SRC/$f" -o "$o" \
     -I"$SRC/upb/reflection/stage0" -I"$SRC" -I"$PWD/build/upbinc" \
     -I"$SRC/third_party/utf8_range" 2>&1 | head -4
done
ar rcs "$OUT/libupb.a" "$OUT"/*.o
echo "built $OUT/libupb.a ($(stat -c%s "$OUT/libupb.a") bytes) with $CC_ ${EXTRA:-(no extra defines)}"
# R5's discipline applied to a #if: prove from the ARTIFACT that the fast decoder was
# compiled in, rather than trusting that the define reached it. The fast parser's
# functions exist only under `#if UPB_FASTTABLE`.
n=$(nm "$OUT/libupb.a" 2>/dev/null | grep -cE " [tT] upb_p[a-z]+_[0-9a-z]+bt")
echo "  fast-parse functions in the archive: $n  (0 means UPB_FASTTABLE compiled to 0)"
