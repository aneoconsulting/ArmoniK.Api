#!/usr/bin/env bash
# FIX-PLAN WP5 step 3's corpus gate for the java slice (the rust slice's gen/corpus.sh is
# the model): ffi/corpus, every row, against
#   R, R-retain                       arm R, unknown fields dropped and retained
#   ffi, ffi-pull, ffi-pull-walk      the core through JNI (push; pull drained; pull in place)
#   ffi-borrow                        decision 13's borrowed facade
#   ffi-retain, ffi-pull-retain, ffi-pull-walk-retain, ffi-borrow-retain
#                                     the same, every decision 11 position armed, u-group encode
# on the target (java17 tree, JDK 17) and the floor (java8 tree, JDK 8), every row of every
# arm under a per-row timeout, against a core built with `corpus,init-guard`; then planted
# controls, each of which MUST FAIL, and the rule gaps the corpus does not reach directly.
#
#   gen/build.sh first (it builds build/jnicorpus and both class trees).
#
# AK_VARIANT=nounk runs it on the NO-UNKNOWN build (FIX-PLAN WP5 step 10): the class trees
# generated from the plan relowered with unknown="drop" (build/cls{17,8}-nounk), the corpus
# core built without `unknown-fields` (build/jnicorpus-nounk), the arms that exist there
# (R, ffi, ffi-pull, ffi-pull-walk, ffi-borrow: every unknown row read and re-encoded in the
# DROPPED form), the same planted controls and rule gaps, and instead of the retain
# controls (3b, 3c) section 3d: the variant's layout facts and rule 6 on its contexts.
set -u
cd "$(dirname "$0")/.."
J17=${J17:-/usr/lib/jvm/java-17-openjdk-amd64}
J8=${J8:-/usr/lib/jvm/java-8-openjdk-amd64}
unset JAVA_TOOL_OPTIONS || true
# The generator state the build used: the core snapshot's poc/codec/gen (gen/build.sh).
[ -z "${AK_CODECGEN:-}" ] && [ -d build/snap/ffi/poc/codec/gen ] && export AK_CODECGEN=$PWD/build/snap/ffi/poc/codec/gen
CP=$(cat deps/cp.txt)
VARIANT=${AK_VARIANT:-full}
if [ "$VARIANT" = nounk ]; then
  SX=-nounk
  ARMS=R,ffi,ffi-pull,ffi-pull-walk,ffi-borrow
else
  SX=
  ARMS=R,R-retain,ffi,ffi-pull,ffi-pull-walk,ffi-borrow,ffi-retain,ffi-pull-retain,ffi-pull-walk-retain,ffi-borrow-retain
fi
SHIM=$PWD/build/jnicorpus$SX/libakjni.so
CLS17=build/cls17$SX
CLS8=build/cls8$SX
export AK_CORPUS_TIMEOUT_MS=${AK_CORPUS_TIMEOUT_MS:-5000}
echo "== java slice corpus gate ($VARIANT build); tree ${AK_COMMIT:-$(git rev-parse --short HEAD)}$(git diff --quiet HEAD -- . ../codec/gen || echo ' + uncommitted changes')"
echo "   $(cat build/core-rev.txt 2>/dev/null)"
echo "   corpus core built with: $(grep -ho ',"features":"\[[^]]*\]' core-build/current/target-corpus$SX/release/.fingerprint/ak-core-*/lib-ak_core.json | sort -u | tr -d '\\' | tr '\n' ' ')"
echo "   shim $SHIM -> $(ldd "$SHIM" | awk '/libak_core/ {print $3}')"
echo "   the loaded core is the corpus-schema build: $(nm -D --defined-only "$(ldd "$SHIM" | awk '/libak_core/ {print $3}')" | grep -c ' T ak_decode_WireZoo') ak_decode_WireZoo export(s)"
echo "   per-row timeout ${AK_CORPUS_TIMEOUT_MS} ms"
fail=0

echo
echo "===== 1. generators current ====="
python3 -S gen/generate.py --check | grep -vE "^ok " | tail -6
python3 -S gen/generate.py --check >/dev/null 2>&1 || { echo "STALE generated files"; fail=1; }

for lv in "target $J17 $CLS17" "floor $J8 $CLS8"; do
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
  if AK_CORPUS_ONLY=$SUB AK_CORPUS_PLANT=$p python3 -S gen/corpus.py "$J17/bin" $CLS17 "$ARMS" "$SHIM" \
       > build/corpus-ctl.txt 2>&1; then
    echo "  control $p: PASSED -- the harness is blind to it"; bad=$((bad+1))
  else
    echo "  control $p: failed as required: $(grep '^RESULT' build/corpus-ctl.txt)"
  fi
done
if AK_CORPUS_ONLY=$SUB AK_SKIP_INIT=1 python3 -S gen/corpus.py "$J17/bin" $CLS17 ffi,ffi-pull,ffi-pull-walk,ffi-borrow "$SHIM" \
     > build/corpus-ctl.txt 2>&1; then
  echo "  control noinit: PASSED -- init-guard is not in the build"; bad=$((bad+1))
else
  echo "  control noinit: failed as required: $(grep '^RESULT' build/corpus-ctl.txt)"
  grep -m2 "could not be constructed\|UNINITIALIZED\|returned -10" build/corpus-ctl.txt | cut -c1-200 | sed 's/^/    /'
  grep -m3 "^      C" build/corpus-ctl.txt | cut -c1-160 | sed 's/^/    /'
fi
[ $bad -eq 0 ] || { echo "CONTROLS FAILED: $bad"; fail=1; }

if [ "$VARIANT" = nounk ]; then
echo
echo "===== 3d. the no-unknown build: its core, its layout facts, rule 6 (target, then floor) ====="
so=$(ldd "$SHIM" | awk '/libak_core/ {print $3}')
echo "  core $so: $(nm -D --defined-only "$so" | grep -cE ' T ak_(uencode|uelem|uelemu|dec_reset)_') u-family/reset exports, $(nm -D --defined-only "$so" | grep -c ' T ak_dec_ctx_new_') ak_dec_ctx_new_<Root>"
for lv in "$J17 $CLS17" "$J8 $CLS8"; do
  set -- $lv
  "$1/bin/java" -cp "$2:$CP" -Dak.lib="$SHIM" ak.RunVariant corpus 2>&1 | grep -v "^Picked up"
  [ "${PIPESTATUS[0]}" -eq 0 ] || fail=1
done
else
echo
echo "===== 3b. decision 11 (WP5 step 9): per-position discard, pull == push, wrong root (target) ====="
"$J17/bin/java" -cp "build/cls17:$CP" -Dak.lib="$SHIM" ak.RunUnkControls 2>&1 | grep -v "^Picked up"
[ "${PIPESTATUS[0]}" -eq 0 ] || fail=1
if "$J17/bin/java" -cp "build/cls17:$CP" -Dak.lib="$SHIM" -Dak.unk.plant=1 ak.RunUnkControls > build/unk-plant.txt 2>&1; then
  echo "  control unk-plant (mask ignored): PASSED -- the discard check is blind"; fail=1
else
  echo "  control unk-plant (mask ignored): failed as required: $(grep 'discard mismatches' build/unk-plant.txt)"
fi

echo
echo "===== 3b'. decision 11 rule 4: unknowns inside oneof members (the C++ slice's three sequences; target, then floor) ====="
for lv in "$J17 build/cls17" "$J8 build/cls8"; do
  set -- $lv
  "$1/bin/java" -cp "$2:$CP" -Dak.lib="$PWD/build/jni/libakjni.so" ak.RunUnkOneof 2>&1 | grep -v "^Picked up"
  [ "${PIPESTATUS[0]}" -eq 0 ] || fail=1
done
if "$J17/bin/java" -cp "build/cls17:$CP" -Dak.lib="$PWD/build/jni/libakjni.so" -Dak.unk.oneofplant=1 ak.RunUnkOneof > build/unk-oneofplant.txt 2>&1; then
  echo "  control unk-oneofplant (not armed): PASSED -- the oneof check is blind"; fail=1
else
  echo "  control unk-oneofplant (not armed): failed as required: $(grep -c FAIL build/unk-oneofplant.txt) failing (sequence, arm) pair(s)"
fi

echo
echo "===== 3c. decision 11 rule 3: buffers of a failed retain decode are reclaimed (target, then floor) ====="
for lv in "$J17 build/cls17" "$J8 build/cls8"; do
  set -- $lv
  "$1/bin/java" -cp "$2:$CP" -Dak.lib="$SHIM" ak.RunUnkLeak 2>&1 | grep -v "^Picked up"
  [ "${PIPESTATUS[0]}" -eq 0 ] || fail=1
done
if "$J17/bin/java" -cp "build/cls17:$CP" -Dak.lib="$SHIM" -Dak.unk.leakplant=1 ak.RunUnkLeak > build/unk-leakplant.txt 2>&1; then
  echo "  control unk-leakplant (no reclaim): PASSED -- the leak check is blind"; fail=1
else
  echo "  control unk-leakplant (no reclaim): failed as required: $(grep -m1 'LEFT BEHIND' build/unk-leakplant.txt | sed 's/^ *//')"
fi

fi

echo
echo "===== 4. rule gaps the corpus does not reach directly (target) ====="
mkdir -p build/rulegaps
"$J17/bin/java" -cp "$CLS17:$CP" -Dak.lib="$SHIM" ak.RunRuleGaps build/rulegaps 2>&1 | grep -v "^Picked up"
PROTOC=build/tools/protoc-3.19.0
C=../../corpus/generated
echo "-- protobuf C++ ($($PROTOC --version)) reading the two merge vectors, corpus.proto:"
echo "merge-singular (ResultRaw):"
$PROTOC -I $C --decode=armonik.ffi.corpus.v1.ResultRaw $C/corpus.proto < build/rulegaps/merge-singular.bin | sed 's/^/   /'
echo "merge-oneof (Probe):"
$PROTOC -I $C --decode=armonik.ffi.corpus.v1.Probe $C/corpus.proto < build/rulegaps/merge-oneof.bin | sed 's/^/   /'

echo
[ $fail -eq 0 ] && echo "CORPUS GATE PASSED" || { echo "CORPUS GATE FAILED"; exit 1; }
