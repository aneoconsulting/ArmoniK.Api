#!/usr/bin/env bash
# Build the python slice's COMPOSED arm: the shared core plus the generated CPython shim.
#
#   ./build.sh <python-exe> [<python-exe> ...]
#
# Steps, and three of them are checks rather than compiles:
#   1. R0   -- the core is the ONE core at poc/codec, reached by path
#   2. R1   -- the generated tree is current with the plans (and the backend guard holds)
#   3.         four builds of the shared core, EVERY one WITH `init-guard` (R-G7): plain,
#              counting, rpc, and the corpus-schema core (`corpus`, test-only)
#   4.         per interpreter: five shims, and the must-fail `noinit` control
#   5. R5   -- the shims' imports of ak_* are unresolved, so the boundary is real
#
# FIX-PLAN WP5 step 5: the generated sources are rendered by poc/codec/gen (py_pure,
# py_capi, and the one C header backend c_abi) from plans; the AK_UPSTREAM
# snapshot mechanism is retired, the
# core and the generator are this checkout's, and the log says which commit and whether
# poc/codec carried uncommitted changes.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
POC="$(dirname "$HERE")"
CORE="$POC/codec"
TBASE="${AK_CARGO_TARGET_BASE:-$HERE/build/cargo}"
cd "$HERE"
# AK_SNAPSHOT=<commit>: build the core and render the shim from a `git archive` of that
# commit's poc/codec, schema and corpus (build/snap/<sha>), not from the working tree. The
# campaign runner uses it so a run names a commit even while other agents edit poc/codec;
# cargo targets are keyed on the snapshot (build/cargo-<sha>).
if [ -n "${AK_SNAPSHOT:-}" ]; then
  SHA=$(git -C "$HERE" rev-parse --short "$AK_SNAPSHOT")
  SNAP="$HERE/build/snap/$SHA"
  if [ ! -d "$SNAP/ffi/poc/codec" ] || [ ! -d "$SNAP/ffi/poc/rust" ]; then
    # poc/rust too: the calib suite builds the rust slice's crossing benchmark from it.
    mkdir -p "$SNAP"
    (cd "$(git -C "$HERE" rev-parse --show-toplevel)" && git archive "$SHA" ffi/poc/codec ffi/poc/rust ffi/schema ffi/corpus) | tar -x -C "$SNAP"
  fi
  CORE="$SNAP/ffi/poc/codec"
  export AK_CODECGEN="$CORE/gen"
  TBASE="${AK_CARGO_TARGET_BASE:-$HERE/build/cargo-$SHA}"
  echo "   core source: $CORE (snapshot of $SHA)"
  echo "   commit:      $SHA (snapshot)$(git -C "$HERE" diff --quiet HEAD -- . || echo ' + uncommitted changes in poc/python')"
else
  echo "   core source: $CORE"
  echo "   commit:      $(git -C "$CORE" rev-parse --short HEAD)$(git -C "$CORE" diff --quiet HEAD -- . || echo ' + UNCOMMITTED CHANGES in poc/codec')$(git -C "$HERE" diff --quiet HEAD -- . || echo ' + uncommitted changes in poc/python')"
fi
echo "   core tree:   $( (cd "$CORE" && find crates gen -type f \( -name '*.rs' -o -name '*.toml' -o -name '*.py' \) | sort | xargs cat | sha256sum | cut -c1-16) )"
echo "   targets:     $TBASE/{plain,count,rpc,corpus}"

echo "== 1. R0: one core, at poc/codec, reached by path =="
if [ -n "${AK_SNAPSHOT:-}" ]; then
  echo "   snapshot mode: the core is the archived poc/codec of $SHA, reached by path; one_core.sh"
  echo "   checks the working tree (other slices), which is not what this build reads"
else
  "$POC/codec/gen/one_core.sh" | tail -3
fi

echo
echo "== 2. R1: the generated tree is current with the plans =="
python3.12 gen/generate.py --check

echo
echo "== 3. the shared core, every build WITH init-guard (R-G7) =="
cargo_q() {  # cargo's warnings go to a file beside the build, the count to the log
  local name=$1; shift
  mkdir -p "$TBASE"
  (cd "$CORE" && CARGO_TARGET_DIR="$TBASE/$name" cargo build --release -q -p ak-core "$@") \
      2> "$TBASE/$name.stderr" || { cat "$TBASE/$name.stderr"; exit 1; }
  echo "   cargo ($name $*): $(grep -c '^warning' "$TBASE/$name.stderr" || true) warnings, in $TBASE/$name.stderr"
}
cargo_q plain --features init-guard
CORELIB="$TBASE/plain/release"
echo "   exported ak_* entry points: $(nm -D --defined-only "$CORELIB/libak_core.so" | grep -cE ' T ak_')"
# The counting core, into its own target directory so a timing never comes from a binary
# carrying counters (README R5).
cargo_q count --features count,init-guard
COUNTLIB="$TBASE/count/release"
# The RPC core: section 9 pulls tonic and tokio in, so it is a third build and a third shim.
cargo_q rpc --features rpc,init-guard
RPCLIB="$TBASE/rpc/release"
echo "   the rpc core: $(nm -D --defined-only "$RPCLIB/libak_core.so" | grep -cE ' T ak_(call|queue|client|runtime)') section 9 entry points"
# The corpus-schema core (WP5 item 6.1): the ABI generated for ffi/corpus's READER schema.
cargo_q corpus --features corpus,init-guard
CORPUSLIB="$TBASE/corpus/release"
echo "   the corpus core: $(nm -D --defined-only "$CORPUSLIB/libak_core.so" | grep -c ' T ak_decode_WireZoo') ak_decode_WireZoo export(s)"
for lib in "$CORELIB" "$COUNTLIB" "$RPCLIB" "$CORPUSLIB"; do
  # grep -c, not grep -q: -q exits at the first match and pipefail then reports nm's SIGPIPE.
  [ "$(nm -D --defined-only "$lib/libak_core.so" | grep -c ' T ak_init$' || true)" -ge 1 ] \
    || { echo "   FAIL: $lib exports no ak_init"; exit 1; }
done

OUT=build
mkdir -p "$OUT"
CFLAGS_COMMON="-O2 -fPIC -Wall -Wextra -Werror -Wno-unused-parameter -fvisibility=hidden"

for PY in "$@"; do
  PYABS=$(command -v "$PY" || true); case "$PYABS" in /*) ;; *) PYABS="$(cd "$(dirname "$PY")" && pwd)/$(basename "$PY")";; esac
  TAG=$("$PY" -c 'import sys;print("%d.%d"%sys.version_info[:2])')
  SOABI=$("$PY" -c 'import sysconfig;print(sysconfig.get_config_var("EXT_SUFFIX"))')
  INC=$("$PY" -c 'import sysconfig;print(sysconfig.get_paths()["include"])')
  PINC=$("$PY" -c 'import sysconfig;print(sysconfig.get_paths()["platinclude"])')
  D="$OUT/py$TAG"
  mkdir -p "$D" "$D/ctl"
  echo
  echo "== python $TAG ($("$PY" -c 'import sys;print(sys.version.split()[0])'), sys.hexversion $("$PY" -c 'import sys;print(hex(sys.hexversion))')) =="
  CI="-I$INC -I$PINC -I$(dirname "$INC")"
  shim() {  # shim <module> <gen dir> <core lib dir> <outdir> [extra cc flags...]
    local mod=$1 gdir=$2 lib=$3 odir=$4; shift 4
    cc $CFLAGS_COMMON -shared $CI -I"$gdir" "$@" \
       -DAK_MODNAME_STR="\"$mod\"" -DAK_INITFUNC="PyInit_$mod" \
       -o "$odir/$mod$SOABI" native/binding.c \
       -L"$lib" -lak_core -Wl,-rpath,"$lib"
  }
  shim _akffi gen/out "$CORELIB" "$D"
  shim _akffi_count gen/out "$COUNTLIB" "$D" -DAK_COUNT
  shim _akffi_rpc gen/out "$RPCLIB" "$D" -DAK_RPC
  shim _akffi_corpus gen/out/corpus "$CORPUSLIB" "$D" -DAK_CORPUS
  # The planted control: the same corpus shim with ak_init SKIPPED. Against a guarded core
  # every codec call must then fail AK_ERR_UNINITIALIZED; corpus.py --control noinit runs it.
  shim _akffi_corpus_noinit gen/out/corpus "$CORPUSLIB" "$D/ctl" -DAK_CORPUS -DAK_SKIP_INIT
  # A test arm: the same corpus shim with 256-byte element chunks and 3-value packed runs,
  # so every element run and packed run in the corpus crosses in many chunks (CONTRACT.md
  # `chunking`: the default 32 KB puts C-elemu-512 in ONE chunk of 32-byte groups).
  shim _akffi_corpus_chunk gen/out/corpus "$CORPUSLIB" "$D" -DAK_CORPUS -DAK_CHUNK_BYTES=256 -DAK_CHUNK_PACKED=3
  echo "   built _akffi, _akffi_count, _akffi_rpc, _akffi_corpus, _akffi_corpus_chunk in $D; the noinit control in $D/ctl"
  python3.12 write_buildinfo.py "$D/buildinfo.json" "$CORE" "$CFLAGS_COMMON"

  # README R5: the boundary is proved from the built artifact, not claimed in a log.
  for so in "$D/_akffi$SOABI" "$D/_akffi_count$SOABI" "$D/_akffi_rpc$SOABI" "$D/_akffi_corpus$SOABI" "$D/_akffi_corpus_chunk$SOABI"; do
    u=$(nm -D --undefined-only "$so" | grep -cE ' ak_' || true)
    [ "$u" -ge 8 ] || { echo "   FAIL: $so imports only $u ak_* symbols"; exit 1; }
    n=$(readelf -d "$so" | grep -c 'libak_core' || true)
    [ "$n" -ge 1 ] || { echo "   FAIL: $so does not depend on libak_core.so"; exit 1; }
    [ "$(nm -D --undefined-only "$so" | grep -cE ' ak_init$' || true)" -ge 1 ] || { echo "   FAIL: $so does not import ak_init (R-G7)"; exit 1; }
    [ "$(nm -D --defined-only "$so" | grep -cE ' T PyInit_' || true)" -eq 1 ] || { echo "   FAIL: $so exports no PyInit_ (3.7-3.8: PyMODINIT_FUNC has no default visibility)"; exit 1; }
    echo "   $(basename "$so"): $u undefined ak_* imports incl. ak_init, PyInit_ exported, libak_core.so NEEDED"
  done
  if [ "$(nm -D --undefined-only "$D/ctl/_akffi_corpus_noinit$SOABI" | grep -cE ' ak_init$' || true)" -ge 1 ]; then
    echo "   FAIL: the noinit control still imports ak_init"; exit 1
  fi
  echo "   control: _akffi_corpus_noinit imports no ak_init (AK_SKIP_INIT is in that build only)"
  # The import-time layout check must be able to fail: the same shim with one fact planted
  # off by one must refuse to import, naming the fact.
  neg="$D/ctl/_akffi_layoutplant$SOABI"
  shim _akffi_layoutplant gen/out "$CORELIB" "$D/ctl" -DAK_PLANT_LAYOUT_MISMATCH=5
  # The failure must be THE layout refusal, not any failure to run (a relative interpreter
  # path once made this control "pass" by not starting Python at all).
  if msg=$( (cd "$D/ctl" && "$PYABS" -c 'import _akffi_layoutplant') 2>&1); then
    echo "   FAIL: a shim with a planted layout mismatch imported"; exit 1
  fi
  case "$msg" in *"ImportError: layout:"*) ;; *) echo "   FAIL: the layout control failed for another reason: $msg"; exit 1;; esac
  echo "   must-fail control: a planted layout mismatch refuses to import: $(echo "$msg" | tail -1 | cut -c1-110)"
  rm -f "$neg"
  so="$D/_akffi_rpc$SOABI"
  r=$(nm -D --undefined-only "$so" | grep -cE ' ak_(call_unary|call_unary_q|call_unary_cb|queue_next|client_new|runtime_new)$' || true)
  [ "$r" -eq 6 ] || { echo "   FAIL: $so imports $r of the 6 section 9 entry points it binds"; exit 1; }
  rl=$(ldd "$so" | awk '/libak_core/ {print $3}')
  [ "$(readlink -f "$rl")" = "$(readlink -f "$RPCLIB/libak_core.so")" ] \
    || { echo "   FAIL: $so resolves libak_core.so to $rl, not the rpc build"; exit 1; }
  echo "   $(basename "$so"): all 6 section 9 imports, libak_core.so resolves to the rpc build"
  # The check above must be able to fail: the same source WITHOUT -DAK_RPC.
  neg="$TBASE/neg_akffi_rpc$SOABI"
  cc $CFLAGS_COMMON -shared $CI -Igen/out \
     -DAK_MODNAME_STR='"_akffi_rpc"' -DAK_INITFUNC=PyInit__akffi_rpc \
     -o "$neg" native/binding.c -L"$RPCLIB" -lak_core -Wl,-rpath,"$RPCLIB"
  r=$(nm -D --undefined-only "$neg" | grep -cE ' ak_(call_unary|call_unary_q|call_unary_cb|queue_next|client_new|runtime_new)$' || true)
  [ "$r" -ne 6 ] || { echo "   FAIL: the section 9 check accepts a shim built without AK_RPC"; exit 1; }
  rm -f "$neg"
  echo "   must-fail control: a shim built WITHOUT -DAK_RPC imports $r of 6 and is refused"
done

echo
echo 'the composed arm is built. conformance.py gates it before anything is timed.'
