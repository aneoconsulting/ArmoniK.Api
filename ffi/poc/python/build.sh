#!/usr/bin/env bash
# Build the python slice's COMPOSED arm: the shared core plus the generated CPython shim.
#
#   ./build.sh <python-exe> [<python-exe> ...]
#
# Work unit 1's microbenchmark arms are built by `mech/build.sh` and are separate; this
# builds the thing that is actually the design.
#
# Four steps, and three of them are checks rather than compiles:
#   1. R0   -- the core is the ONE core at poc/codec, reached by path
#   2. R1   -- the generated tree is current with shapes.json
#   3.         the core, as a shared library
#   4. R5   -- the shim's imports of ak_* are unresolved, so the boundary is real
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
POC="$(dirname "$HERE")"
# AK_UPSTREAM: an `ffi/` tree to take the SHARED inputs from -- the core source and the
# generator inputs `gen/generate.py` imports. Default: this checkout, so the core is the one
# core at poc/codec (R0). It may point at a `git archive <commit> ffi/poc/codec
# ffi/poc/cpp/gen ffi/schema` extraction, which is how this slice builds and gates while
# other agents are editing those directories: the build then names a commit instead of a
# working tree caught mid-edit. Never a fork -- nothing in this slice writes to it.
UPSTREAM="${AK_UPSTREAM:-$(dirname "$POC")}"
CORE="$UPSTREAM/poc/codec"
export AK_UPSTREAM="$UPSTREAM"   # so every generate.py --check below reads the same inputs
# AK_CARGO_TARGET_BASE: where the three core builds go. Default is inside this slice's
# own build/ (git-ignored), so this slice never writes into poc/codec/target* and never
# races another slice's cargo lock or artifacts there.
TBASE="${AK_CARGO_TARGET_BASE:-$HERE/build/cargo}"
cd "$HERE"
echo "   core source: $CORE"
echo "   core commit: $(cat "$UPSTREAM/../COMMIT" 2>/dev/null || echo "working tree at $(git -C "$CORE" rev-parse --short HEAD)$(git -C "$CORE" diff --quiet HEAD -- . || echo ' + UNCOMMITTED CHANGES')")"
echo "   core tree:   $( (cd "$CORE" && find crates gen -type f \( -name '*.rs' -o -name '*.toml' -o -name '*.py' \) | sort | xargs cat | sha256sum | cut -c1-16) )"
echo "   targets:     $TBASE/{plain,count,rpc}"

echo "== 1. R0: one core, at poc/codec, reached by path =="
"$POC/codec/gen/one_core.sh" | tail -3

echo
echo "== 2. R1: the generated tree is current with shapes.json =="
python3 gen/generate.py --check

echo
echo "== 3. the shared core =="
cargo_q() {  # cargo's warnings go to a file beside the build, the count to the log
  local name=$1; shift
  mkdir -p "$TBASE"
  (cd "$CORE" && CARGO_TARGET_DIR="$TBASE/$name" cargo build --release -q -p ak-core "$@") \
      2> "$TBASE/$name.stderr" || { cat "$TBASE/$name.stderr"; exit 1; }
  echo "   cargo ($name$([ $# -gt 0 ] && echo " $*")): $(grep -c '^warning' "$TBASE/$name.stderr" || true) warnings, in $TBASE/$name.stderr"
}
cargo_q plain
CORELIB="$TBASE/plain/release"
ls -la "$CORELIB/libak_core.so"
echo "   exported ak_* entry points: $(nm -D --defined-only "$CORELIB/libak_core.so" | grep -cE ' T ak_')"

# The counting core, into its own target directory so a timing never comes from a binary
# carrying counters (README R5).
cargo_q count --features count
COUNTLIB="$TBASE/count/release"

# The RPC core, into a THIRD target directory. ABI v1 section 9 lives behind the `rpc`
# feature and pulls tonic and tokio in, so building it into the core the codec arms link
# against would change the library every timing in `bench.py` is taken over. Separate
# build, separate shim, separate process -- the same rule the counting build follows, and
# for the same reason.
cargo_q rpc --features rpc
RPCLIB="$TBASE/rpc/release"
echo "   the rpc core: $(nm -D --defined-only "$RPCLIB/libak_core.so" | grep -cE ' T ak_(call|queue|client|runtime)') section 9 entry points"

OUT=build
mkdir -p "$OUT"
CFLAGS_COMMON="-O2 -fPIC -Wall -Wextra -Werror -Wno-unused-parameter -fvisibility=hidden"

for PY in "$@"; do
  TAG=$("$PY" -c 'import sys;print("%d.%d"%sys.version_info[:2])')
  SOABI=$("$PY" -c 'import sysconfig;print(sysconfig.get_config_var("EXT_SUFFIX"))')
  INC=$("$PY" -c 'import sysconfig;print(sysconfig.get_paths()["include"])')
  D="$OUT/py$TAG"
  mkdir -p "$D"
  echo
  echo "== python $TAG =="

  cc $CFLAGS_COMMON -shared -I"$INC" -Igen/out \
     -DAK_MODNAME_STR='"_akffi"' -DAK_INITFUNC=PyInit__akffi \
     -o "$D/_akffi$SOABI" native/binding.c \
     -L"$CORELIB" -lak_core -Wl,-rpath,"$CORELIB"

  cc $CFLAGS_COMMON -shared -I"$INC" -Igen/out -DAK_COUNT \
     -DAK_MODNAME_STR='"_akffi_count"' -DAK_INITFUNC=PyInit__akffi_count \
     -o "$D/_akffi_count$SOABI" native/binding.c \
     -L"$COUNTLIB" -lak_core -Wl,-rpath,"$COUNTLIB"

  # The RPC shim, against the rpc-feature core. Only `rpc.py` imports it.
  cc $CFLAGS_COMMON -shared -I"$INC" -Igen/out -DAK_RPC \
     -DAK_MODNAME_STR='"_akffi_rpc"' -DAK_INITFUNC=PyInit__akffi_rpc \
     -o "$D/_akffi_rpc$SOABI" native/binding.c \
     -L"$RPCLIB" -lak_core -Wl,-rpath,"$RPCLIB"

  echo "   built $D/_akffi$SOABI and the counting build beside it"

  # README R5: the boundary is proved from the built artifact, not claimed in a log.
  # A shim whose ak_* calls were folded away would be the no-boundary control wearing
  # the composed arm's name, which is the defect that cost the rust slice a stage.
  # `_akffi_rpc` is in the loop (FIX-PLAN R-D3): it carries the codec too -- cell C of the
  # RPC grid decodes through it -- so it needs the same proof as the other two. It also
  # has to import section 9, or the grid's core cells are running over nothing.
  for so in "$D/_akffi$SOABI" "$D/_akffi_count$SOABI" "$D/_akffi_rpc$SOABI"; do
    u=$(nm -D --undefined-only "$so" | grep -cE ' ak_' || true)
    [ "$u" -ge 8 ] || { echo "   FAIL: $so imports only $u ak_* symbols"; exit 1; }
    n=$(readelf -d "$so" | grep -c 'libak_core' || true)
    [ "$n" -ge 1 ] || { echo "   FAIL: $so does not depend on libak_core.so"; exit 1; }
    echo "   $(basename "$so"): $u undefined ak_* imports, libak_core.so NEEDED"
  done
  so="$D/_akffi_rpc$SOABI"
  r=$(nm -D --undefined-only "$so" | grep -cE ' ak_(call_unary|call_unary_q|call_unary_cb|queue_next|client_new|runtime_new)$' || true)
  [ "$r" -eq 6 ] || { echo "   FAIL: $so imports $r of the 6 section 9 entry points it binds"; exit 1; }
  rl=$(ldd "$so" | awk '/libak_core/ {print $3}')
  [ "$rl" = "$(readlink -f "$RPCLIB/libak_core.so")" ] || [ "$(readlink -f "$rl")" = "$(readlink -f "$RPCLIB/libak_core.so")" ] \
    || { echo "   FAIL: $so resolves libak_core.so to $rl, not the rpc build"; exit 1; }
  echo "   $(basename "$so"): all 6 section 9 imports, libak_core.so resolves to the rpc build"
  # The check above must be able to fail (README R1: a refusal nobody has seen refuse is
  # not a check). The same source WITHOUT -DAK_RPC, under the rpc shim's name, is exactly
  # the build that would leave the grid's core cells running over nothing.
  neg="$TBASE/neg_akffi_rpc$SOABI"
  cc $CFLAGS_COMMON -shared -I"$INC" -Igen/out \
     -DAK_MODNAME_STR='"_akffi_rpc"' -DAK_INITFUNC=PyInit__akffi_rpc \
     -o "$neg" native/binding.c -L"$RPCLIB" -lak_core -Wl,-rpath,"$RPCLIB"
  r=$(nm -D --undefined-only "$neg" | grep -cE ' ak_(call_unary|call_unary_q|call_unary_cb|queue_next|client_new|runtime_new)$' || true)
  [ "$r" -ne 6 ] || { echo "   FAIL: the section 9 check accepts a shim built without AK_RPC"; exit 1; }
  rm -f "$neg"
  echo "   must-fail control: a shim built WITHOUT -DAK_RPC imports $r of 6 and is refused"
done

echo
echo 'the composed arm is built. conformance.py gates it before anything is timed.'
