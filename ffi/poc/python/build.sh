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
  if [ ! -d "$SNAP/ffi/poc/codec" ] || [ ! -d "$SNAP/ffi/poc/rust" ] || [ ! -d "$SNAP/packages/rust" ]; then
    # poc/rust too: the calib suite builds the rust slice's crossing benchmark from it, and the
    # shared RPC server; packages/rust since the rust slice's campaign crate depends on it by path.
    mkdir -p "$SNAP"
    (cd "$(git -C "$HERE" rev-parse --show-toplevel)" && git archive "$SHA" ffi/poc/codec ffi/poc/rust ffi/schema ffi/corpus packages/rust) | tar -x -C "$SNAP"
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
echo "   targets:     $TBASE/{plain,count,rpc,corpus} and the no-unknown variant's $TBASE/{plain,count,rpc,corpus}-nounk"

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
# WP5 step 10, THE NO-UNKNOWN VARIANT: the same four cores with `unknown-fields` OFF
# (--no-default-features), each in its OWN target directory (a variant build in a shared
# target overwrites libak_core.so under the full binaries: found by the rust slice).
cargo_q plain-nounk --no-default-features --features init-guard
cargo_q count-nounk --no-default-features --features count,init-guard
cargo_q rpc-nounk --no-default-features --features rpc,init-guard
cargo_q corpus-nounk --no-default-features --features corpus,init-guard
# Req 19 (R-H31): the RPC cells are counted per call, so the rpc core has a counting build too,
# in both variants.
cargo_q rpc-count --features rpc,count,init-guard
cargo_q rpc-count-nounk --no-default-features --features rpc,count,init-guard
RPCCOUNTLIB="$TBASE/rpc-count/release"; NRPCCOUNTLIB="$TBASE/rpc-count-nounk/release"
NCORELIB="$TBASE/plain-nounk/release"; NCOUNTLIB="$TBASE/count-nounk/release"
NRPCLIB="$TBASE/rpc-nounk/release"; NCORPUSLIB="$TBASE/corpus-nounk/release"
# FIX-PLAN WP13 / D11 as amended: the h2-batch variant of every core that carries h2 (the rpc
# feature): rpc, rpc-nounk, rpc-count, rpc-count-nounk. The full rpc core through the shared
# poc/codec/h2-batch/build.sh (it materialises h2 0.4.19 from cargo's cache, checks its sha256
# against Cargo.lock and applies h2-batch.patch); the other three by hand with the same
# --config and the same patched source (README "By hand"), because the script takes features
# only and the no-unknown cores need --no-default-features. Each in its own target directory.
# The stock cores are the ones above (an ordinary build is stock: no [patch] anywhere).
H2B="$TBASE/rpc-h2batch"
bash "$CORE/h2-batch/build.sh" h2-batch "$H2B" rpc,init-guard > "$TBASE/rpc-h2batch.log" 2>&1 \
  || { tail -5 "$TBASE/rpc-h2batch.log"; echo "   FAIL: the h2-batch rpc core did not build"; exit 1; }
H2SRC="$H2B/h2-batch-src"
cargo_h2b() {  # cargo_h2b <name> <cargo args...>: an h2-batch core, by hand
  local name=$1; shift
  cp "$CORE/Cargo.lock" "$TBASE/$name.lock.saved"
  (cd "$CORE" && CARGO_TARGET_DIR="$TBASE/$name" CARGO_BUILD_BUILD_DIR="$TBASE/$name" cargo build --release -q -p ak-core "$@" \
     --config "patch.crates-io.h2.path=\"$H2SRC\"") 2> "$TBASE/$name.stderr" || { cp "$TBASE/$name.lock.saved" "$CORE/Cargo.lock"; cat "$TBASE/$name.stderr"; exit 1; }
  cp "$TBASE/$name.lock.saved" "$CORE/Cargo.lock"
}
cargo_h2b rpc-nounk-h2batch --no-default-features --features rpc,init-guard
cargo_h2b rpc-count-h2batch --features rpc,count,init-guard
cargo_h2b rpc-count-nounk-h2batch --no-default-features --features rpc,count,init-guard
HRPCLIB="$H2B/release"; HNRPCLIB="$TBASE/rpc-nounk-h2batch/release"
HRPCCOUNTLIB="$TBASE/rpc-count-h2batch/release"; HNRPCCOUNTLIB="$TBASE/rpc-count-nounk-h2batch/release"
# Which h2 each rpc core compiled in (panic-location paths of h2's framed_write.rs): the stock
# cores must name the crates.io source, the h2-batch cores the patched one (h2-batch-src).
for pair in "$RPCLIB:stock" "$NRPCLIB:stock" "$RPCCOUNTLIB:stock" "$NRPCCOUNTLIB:stock" \
            "$HRPCLIB:h2-batch" "$HNRPCLIB:h2-batch" "$HRPCCOUNTLIB:h2-batch" "$HNRPCCOUNTLIB:h2-batch"; do
  IFS=: read -r lib v <<< "$pair"
  src=$(strings "$lib/libak_core.so" | grep -o '/[^ ]*/src/codec/framed_write\.rs' | sort -u | head -1)
  case "$v:$src" in
    stock:*h2-batch-src*|stock:) echo "   FAIL: $lib (stock) compiled in h2 from '$src'"; exit 1;;
    h2-batch:*h2-batch-src*) ;;
    h2-batch:*) echo "   FAIL: $lib (h2-batch) compiled in h2 from '$src'"; exit 1;;
  esac
  echo "   $(basename "$(dirname "$lib")"): h2 $v ($(echo "$src" | grep -o '[^/]*/src/codec/framed_write\.rs$' | sed 's#/src/codec/framed_write.rs##')), sha256 $(sha256sum "$lib/libak_core.so" | cut -c1-16)"
done
UFAM=' T ak_(uencode_|uelem|dec_reset_)'
for lib in "$CORELIB" "$COUNTLIB" "$RPCLIB" "$CORPUSLIB"; do
  n=$(nm -D --defined-only "$lib/libak_core.so" | grep -cE "$UFAM" || true)
  [ "$n" -gt 0 ] || { echo "   FAIL: the full core $lib exports no u-family entry point"; exit 1; }
done
for lib in "$NCORELIB" "$NCOUNTLIB" "$NRPCLIB" "$NCORPUSLIB"; do
  n=$(nm -D --defined-only "$lib/libak_core.so" | grep -cE "$UFAM" || true)
  [ "$n" -eq 0 ] || { echo "   FAIL: the no-unknown core $lib exports $n u-family entry points"; exit 1; }
done
echo "   no-unknown cores: 0 u-family exports (ak_uencode_*, ak_uelem*, ak_dec_reset_*) in each of 4; the full cores export $(nm -D --defined-only "$CORELIB/libak_core.so" | grep -cE "$UFAM")"
for lib in "$CORELIB" "$COUNTLIB" "$RPCLIB" "$CORPUSLIB" "$NCORELIB" "$NCOUNTLIB" "$NRPCLIB" "$NCORPUSLIB"; do
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
  # The per-thread AK_LAST_RECLAIMED check's must-fail twin: one process-wide slot.
  shim _akffi_corpus_globalreclaim gen/out/corpus "$CORPUSLIB" "$D/ctl" -DAK_CORPUS -DAK_THREAD_LOCAL=
  # The leak check's must-fail twin (R-H9): ak_py_release skipped, every delivered slot leaks.
  shim _akffi_corpus_skiprelease gen/out/corpus "$CORPUSLIB" "$D/ctl" -DAK_CORPUS -DAK_PLANT_SKIP_RELEASE
  # WP5 step 10: the no-unknown variant, separately built modules over the variant cores.
  shim _akffi_nounk gen/out/nounk "$NCORELIB" "$D" -DAK_NOUNK
  shim _akffi_count_nounk gen/out/nounk "$NCOUNTLIB" "$D" -DAK_NOUNK -DAK_COUNT
  shim _akffi_rpc_nounk gen/out/nounk "$NRPCLIB" "$D" -DAK_NOUNK -DAK_RPC
  shim _akffi_rpc_count gen/out "$RPCCOUNTLIB" "$D" -DAK_RPC -DAK_COUNT
  shim _akffi_rpc_count_nounk gen/out/nounk "$NRPCCOUNTLIB" "$D" -DAK_NOUNK -DAK_RPC -DAK_COUNT
  for m in _akffi_rpc_count _akffi_rpc_count_nounk; do
    f=$( (cd "$D" && "$PYABS" -c "import $m; print($m.counting(), $m.abi_counts() is not None, $m.nounk())") 2>&1) \
      || { echo "   FAIL: $m does not import: $f"; exit 1; }
    echo "   $m (the RPC counting build, req 19): counting, abi counts, variant: $f"
  done
  # WP13 / D11: the h2-batch variant's rpc shims, the same module names in $D/h2batch (AK_H2=h2-batch
  # puts it first on sys.path, arms.py), each linked to its h2-batch core
  mkdir -p "$D/h2batch"
  shim _akffi_rpc gen/out "$HRPCLIB" "$D/h2batch" -DAK_RPC
  shim _akffi_rpc_nounk gen/out/nounk "$HNRPCLIB" "$D/h2batch" -DAK_NOUNK -DAK_RPC
  shim _akffi_rpc_count gen/out "$HRPCCOUNTLIB" "$D/h2batch" -DAK_RPC -DAK_COUNT
  shim _akffi_rpc_count_nounk gen/out/nounk "$HNRPCCOUNTLIB" "$D/h2batch" -DAK_NOUNK -DAK_RPC -DAK_COUNT
  for pair in "_akffi_rpc:$HRPCLIB" "_akffi_rpc_nounk:$HNRPCLIB" "_akffi_rpc_count:$HRPCCOUNTLIB" "_akffi_rpc_count_nounk:$HNRPCCOUNTLIB"; do
    IFS=: read -r m lib <<< "$pair"
    rl=$(ldd "$D/h2batch/$m$SOABI" | awk '/libak_core/ {print $3}')
    [ "$(readlink -f "$rl")" = "$(readlink -f "$lib/libak_core.so")" ] || { echo "   FAIL: h2batch/$m resolves libak_core.so to $rl, not $lib"; exit 1; }
    f=$(cd "$HERE" && AK_H2=h2-batch AK_FFI_MODULE=$m "$PYABS" -c "import sys; sys.argv=['x']; import arms; m=arms._ffi; import os; print(m.__name__, os.path.relpath(m.__file__), m.nounk())" 2>&1 | tail -1) \
      || { echo "   FAIL: AK_H2=h2-batch does not load h2batch/$m: $f"; exit 1; }
    case "$f" in *h2batch/*) ;; *) echo "   FAIL: AK_H2=h2-batch loaded $f, not build/py$TAG/h2batch/$m"; exit 1;; esac
    echo "   h2batch/$m: libak_core.so is $(basename "$(dirname "$lib")"); AK_H2=h2-batch loads $f"
  done
  shim _akffi_corpus_nounk gen/out/corpus-nounk "$NCORPUSLIB" "$D" -DAK_NOUNK -DAK_CORPUS
  shim _akffi_corpus_chunk_nounk gen/out/corpus-nounk "$NCORPUSLIB" "$D" -DAK_NOUNK -DAK_CORPUS -DAK_CHUNK_BYTES=256 -DAK_CHUNK_PACKED=3
  shim _akffi_corpus_noinit_nounk gen/out/corpus-nounk "$NCORPUSLIB" "$D/ctl" -DAK_NOUNK -DAK_CORPUS -DAK_SKIP_INIT
  echo "   built the no-unknown variant: _akffi_nounk, _akffi_count_nounk, _akffi_rpc_nounk, _akffi_corpus_nounk, _akffi_corpus_chunk_nounk; its noinit control in $D/ctl"
  for pair in "_akffi:$CORELIB:0" "_akffi_count:$COUNTLIB:0" "_akffi_rpc:$RPCLIB:0" "_akffi_corpus:$CORPUSLIB:0" \
              "_akffi_nounk:$NCORELIB:1" "_akffi_count_nounk:$NCOUNTLIB:1" "_akffi_rpc_nounk:$NRPCLIB:1" \
              "_akffi_corpus_nounk:$NCORPUSLIB:1" "_akffi_corpus_chunk_nounk:$NCORPUSLIB:1"; do
    IFS=: read -r m lib v <<< "$pair"
    so="$D/$m$SOABI"
    rl=$(ldd "$so" | awk '/libak_core/ {print $3}')
    [ "$(readlink -f "$rl")" = "$(readlink -f "$lib/libak_core.so")" ] || { echo "   FAIL: $m resolves libak_core.so to $rl, not $lib"; exit 1; }
    u=$(nm -D --undefined-only "$so" | grep -cE ' U ak_(uencode_|uelem|dec_reset_)' || true)
    if [ "$v" = 1 ]; then [ "$u" -eq 0 ] || { echo "   FAIL: $m (no-unknown) imports $u u-family entry points"; exit 1; }
    else [ "$u" -gt 0 ] || { echo "   FAIL: $m (full) imports no u-family entry point"; exit 1; }; fi
    f=$( (cd "$D" && "$PYABS" -c "import $m; print(len($m.layout_facts()), $m.nounk())") 2>&1) \
      || { echo "   FAIL: $m does not import: $f"; exit 1; }
    echo "   $m: libak_core.so is $(basename "$(dirname "$lib")"), $u u-family imports, layout facts and variant: $f"
  done
  # Must-fail: the no-unknown shim against the FULL core refuses to import at the layout check.
  shim _akffi_nounk gen/out/nounk "$CORELIB" "$D/ctl" -DAK_NOUNK
  if msg=$( (cd "$D/ctl" && "$PYABS" -c 'import _akffi_nounk') 2>&1); then
    echo "   FAIL: the no-unknown shim imported against the full core"; exit 1
  fi
  case "$msg" in *"ImportError: layout:"*) ;; *) echo "   FAIL: the variant-mismatch control failed for another reason: $msg"; exit 1;; esac
  echo "   must-fail control: the no-unknown shim over the full core refuses to import: $(echo "$msg" | tail -1 | cut -c1-100)"
  rm -f "$D/ctl/_akffi_nounk$SOABI"
  python3.12 write_buildinfo.py "$D/buildinfo.json" "$CORE" "$CFLAGS_COMMON"

  # README R5: the boundary is proved from the built artifact, not claimed in a log.
  for so in "$D/_akffi$SOABI" "$D/_akffi_count$SOABI" "$D/_akffi_rpc$SOABI" "$D/_akffi_corpus$SOABI" "$D/_akffi_corpus_chunk$SOABI" \
            "$D/_akffi_nounk$SOABI" "$D/_akffi_count_nounk$SOABI" "$D/_akffi_rpc_nounk$SOABI" "$D/_akffi_corpus_nounk$SOABI" "$D/_akffi_corpus_chunk_nounk$SOABI"; do
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
echo "== the grpc-core CPU-count shim (WP13 / D14: grpc-core sizes itself from sysconf(_SC_NPROCESSORS_CONF)) =="
cc -O2 -shared -fPIC -Wall -Werror -o build/ncpus_shim.so native/ncpus_shim.c -ldl
echo "   build/ncpus_shim.so: LD_PRELOAD with AK_SHIM_NCPUS=N makes sysconf(_SC_NPROCESSORS_CONF/ONLN) report N"

echo
echo "== the shared campaign RPC server (FIX-PLAN WP10: the Rust rpc_server, poc/rust/SERVER.md) =="
# serve.sh of the tree this build reads: the snapshot's in snapshot mode, else this checkout's
SERVE="$POC/rust/serve.sh"; [ -n "${AK_SNAPSHOT:-}" ] && SERVE="$SNAP/ffi/poc/rust/serve.sh"
bash "$SERVE" build 2>&1 | tail -1 | sed 's/^/   /'
[ -x "$(dirname "$SERVE")/target-server/release/rpc_server" ] || { echo "   FAIL: serve.sh build left no rpc_server"; exit 1; }
# the incumbent's typed stub for it (cell A), beside the target's shapes_pb2 (the RPC grid runs
# at the target only; mech/build.sh writes the same pair)
SG="$POC/../schema/generated"; [ -n "${AK_SNAPSHOT:-}" ] && SG="$SNAP/ffi/schema/generated"
mkdir -p mech/build/pb2
python3.12 -m grpc_tools.protoc -I"$SG" -Iproto --python_out=mech/build/pb2 --grpc_python_out=mech/build/pb2 campaign_grid.proto
echo "   mech/build/pb2/campaign_grid_pb2{,_grpc}.py written from proto/campaign_grid.proto"

echo
echo 'the composed arm is built. conformance.py gates it before anything is timed.'
