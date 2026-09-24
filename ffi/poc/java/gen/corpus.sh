#!/usr/bin/env bash
# FIX-PLAN WP5 step 3's corpus gate for the java slice (the rust slice's gen/corpus.sh is
# the model): ffi/corpus, every row, against
#   R, R-retain                       arm R, unknown fields dropped and retained
#   ffi, ffi-pull, ffi-pull-walk      the core through JNI (push; pull drained; pull in place)
#   ffi-borrow                        decision 13's borrowed facade
# on the target (java17 tree, JDK 17) and the floor (java8 tree, JDK 8), every row of every
# arm under a per-row timeout, against a core built with `corpus,init-guard`; then planted
# controls, each of which MUST FAIL, and the rule gaps the corpus does not reach directly.
#
#   gen/build.sh first (it builds build/jnicorpus and both class trees).
set -u
cd "$(dirname "$0")/.."
J17=${J17:-/usr/lib/jvm/java-17-openjdk-amd64}
J8=${J8:-/usr/lib/jvm/java-8-openjdk-amd64}
unset JAVA_TOOL_OPTIONS || true
CP=$(cat deps/cp.txt)
SHIM=$PWD/build/jnicorpus/libakjni.so
ARMS=R,R-retain,ffi,ffi-pull,ffi-pull-walk,ffi-borrow
export AK_CORPUS_TIMEOUT_MS=${AK_CORPUS_TIMEOUT_MS:-5000}
echo "== java slice corpus gate; tree ${AK_COMMIT:-$(git rev-parse --short HEAD)}$(git diff --quiet HEAD -- . ../codec/gen || echo ' + uncommitted changes')"
echo "   $(cat build/core-rev.txt 2>/dev/null)"
echo "   corpus core built with: $(grep -ho ',"features":"\[[^]]*\]' core-build/current/target-corpus/release/.fingerprint/ak-core-*/lib-ak_core.json | sort -u | tr -d '\\' | tr '\n' ' ')"
echo "   shim $SHIM -> $(ldd "$SHIM" | awk '/libak_core/ {print $3}')"
echo "   the loaded core is the corpus-schema build: $(nm -D --defined-only "$(ldd "$SHIM" | awk '/libak_core/ {print $3}')" | grep -c ' T ak_decode_WireZoo') ak_decode_WireZoo export(s)"
echo "   per-row timeout ${AK_CORPUS_TIMEOUT_MS} ms"
fail=0

echo
echo "===== 1. generators current ====="
python3 -S gen/generate.py --check | grep -vE "^ok " | tail -6
python3 -S gen/generate.py --check >/dev/null 2>&1 || { echo "STALE generated files"; fail=1; }

for lv in "target $J17 build/cls17" "floor $J8 build/cls8"; do
  set -- $lv
  echo
  echo "===== 2. the corpus, $1 ($2, $3), arms $ARMS ====="
  timeout 3600 python3 -S gen/corpus.py "$2/bin" "$3" "$ARMS" "$SHIM" || fail=1
done

echo
echo "===== 3. controls (target), each MUST FAIL ====="
SUB="S-Probe,U-root,X-lenwrap,E-map,T-dec-,X-tag-zero-ListResults"
bad=0
for p in proj reenc accept; do
  if AK_CORPUS_ONLY=$SUB AK_CORPUS_PLANT=$p python3 -S gen/corpus.py "$J17/bin" build/cls17 "$ARMS" "$SHIM" \
       > build/corpus-ctl.txt 2>&1; then
    echo "  control $p: PASSED -- the harness is blind to it"; bad=$((bad+1))
  else
    echo "  control $p: failed as required: $(grep '^RESULT' build/corpus-ctl.txt)"
  fi
done
if AK_CORPUS_ONLY=$SUB AK_SKIP_INIT=1 python3 -S gen/corpus.py "$J17/bin" build/cls17 ffi,ffi-pull,ffi-pull-walk,ffi-borrow "$SHIM" \
     > build/corpus-ctl.txt 2>&1; then
  echo "  control noinit: PASSED -- init-guard is not in the build"; bad=$((bad+1))
else
  echo "  control noinit: failed as required: $(grep '^RESULT' build/corpus-ctl.txt)"
  grep -m2 "could not be constructed\|UNINITIALIZED\|returned -10" build/corpus-ctl.txt | cut -c1-200 | sed 's/^/    /'
  grep -m3 "^      C" build/corpus-ctl.txt | cut -c1-160 | sed 's/^/    /'
fi
[ $bad -eq 0 ] || { echo "CONTROLS FAILED: $bad"; fail=1; }

echo
echo "===== 4. rule gaps the corpus does not reach directly (target) ====="
mkdir -p build/rulegaps
"$J17/bin/java" -cp "build/cls17:$CP" -Dak.lib="$SHIM" ak.RunRuleGaps build/rulegaps 2>&1 | grep -v "^Picked up"
PROTOC=build/tools/protoc-3.19.0
C=../../corpus/generated
echo "-- protobuf C++ ($($PROTOC --version)) reading the two merge vectors, corpus.proto:"
echo "merge-singular (ResultRaw):"
$PROTOC -I $C --decode=armonik.ffi.corpus.v1.ResultRaw $C/corpus.proto < build/rulegaps/merge-singular.bin | sed 's/^/   /'
echo "merge-oneof (Probe):"
$PROTOC -I $C --decode=armonik.ffi.corpus.v1.Probe $C/corpus.proto < build/rulegaps/merge-oneof.bin | sed 's/^/   /'

echo
[ $fail -eq 0 ] && echo "CORPUS GATE PASSED" || { echo "CORPUS GATE FAILED"; exit 1; }
