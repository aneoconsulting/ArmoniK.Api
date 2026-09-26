#!/usr/bin/env bash
# ABI v1 decision 11 under AddressSanitizer and LeakSanitizer, BOTH builds: the full build
# (WP5 step 9: the binding's unknown-field buffers) and the no-unknown build (WP5 step 10:
# unknown fields compiled out; WP6 step 1 asked for it under ASan). Every buffer the binding hands the core is malloc'd and is freed on
# delivery (unk_take / unk_drop / unk_drop_entry) or reclaimed after the decode; a double
# free, a use after free or a leak aborts the child, which corpus_all.py reports as a crash
# row. Correctness only; nothing is timed.
#
#   gen/d11_asan.sh [BUILD_DIR]     default: a scratch directory, deleted afterwards
# Log: ffi/logs/cpp/asan.log (wp5s9-asan.log is the step 9 run, full build only)
set -u
cd "$(dirname "$0")/.." || exit 2
L=../../logs/cpp
KEEP=${1:-}
A=${KEEP:-$(mktemp -d)}
FAILS=0
mkdir -p "$A"
{
  echo "# cpp slice: decision 11 under ASan + LSan, full and no-unknown builds"
  echo "#   date       $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "#   compiler   $(g++ --version | head -1)"
  echo "#   commit     $(git rev-parse --short HEAD 2>/dev/null || echo "${AK_COMMIT:-unknown}")"
  echo "#   flags      -fsanitize=address -fno-omit-frame-pointer, ASAN_OPTIONS=detect_leaks=1"
  echo
  cmake -S . -B "$A" -DCMAKE_CXX_FLAGS="-fsanitize=address -fno-omit-frame-pointer" \
        -DCMAKE_EXE_LINKER_FLAGS="-fsanitize=address" > "$A/asan-cfg.log" 2>&1 \
    && cmake --build "$A" -j"$(nproc)" --target corpus_all_a17 conformance_a17_shared \
         corpus_nounk_a17 conformance_nounk_a17 \
         > "$A/asan-build.log" 2>&1 \
    || { echo ">>> FAIL: the ASan build failed"; tail -5 "$A/asan-build.log"; exit 1; }
  echo "===== conformance_a17_shared (payload identity, decision 11 section) ====="
  (cd ../../schema/generated && ASAN_OPTIONS=detect_leaks=1 "$A/conformance_a17_shared" payloads \
     > "$A/conf.log" 2>&1); rc=$?
  grep -A20 'decision 11' "$A/conf.log"; grep -E 'ERROR: (Leak|Address)Sanitizer' "$A/conf.log"
  [ $rc -eq 0 ] && echo ">>> ok: conformance under ASan (exit 0)" || { echo ">>> FAIL: exit $rc"; FAILS=$((FAILS+1)); }
  echo
  echo "===== corpus_all_a17 --unk-controls under ASan (a sanitizer abort is a crash row) ====="
  ASAN_OPTIONS=detect_leaks=1 python3 gen/corpus_all.py "$A/corpus_all_a17" --unk-controls --timeout 60
  [ $? -eq 0 ] && echo ">>> ok: controls under ASan" || { echo ">>> FAIL: controls under ASan"; FAILS=$((FAILS+1)); }
  echo
  echo "===== corpus_all_a17, four arms, under ASan ====="
  ASAN_OPTIONS=detect_leaks=1 python3 gen/corpus_all.py "$A/corpus_all_a17" --timeout 60 \
    --max-retain-gap U-map-entry | grep -E '^## |^   pass|retention|^# rows|^CORPUS|FAIL|!!'
  [ ${PIPESTATUS[0]} -eq 0 ] && echo ">>> ok: corpus under ASan" || { echo ">>> FAIL: corpus under ASan"; FAILS=$((FAILS+1)); }
  echo
  echo "===== NO-UNKNOWN build: conformance_nounk_a17 (byte identity, 240 layout facts, rule 6) ====="
  (cd ../../schema/generated && ASAN_OPTIONS=detect_leaks=1 "$A/conformance_nounk_a17" payloads \
     > "$A/confn.log" 2>&1); rc=$?
  grep -E 'layout facts|no-unknown build|checks,' "$A/confn.log"; grep -E 'ERROR: (Leak|Address)Sanitizer' "$A/confn.log"
  [ $rc -eq 0 ] && echo ">>> ok: no-unknown conformance under ASan (exit 0)" || { echo ">>> FAIL: exit $rc"; FAILS=$((FAILS+1)); }
  echo
  echo "===== NO-UNKNOWN build: corpus_nounk_a17, four arms, every unknown row dropped, under ASan ====="
  ASAN_OPTIONS=detect_leaks=1 python3 gen/corpus_all.py "$A/corpus_nounk_a17" --timeout 60 \
    --expect-dropped ffi-drop | grep -E '^## |^   pass|dropped form|^# rows|^CORPUS|FAIL|!!'
  [ ${PIPESTATUS[0]} -eq 0 ] && echo ">>> ok: no-unknown corpus under ASan" || { echo ">>> FAIL: no-unknown corpus under ASan"; FAILS=$((FAILS+1)); }
  echo "d11_asan: $FAILS failure(s)"
} > "$L/asan.log" 2>&1
[ -z "$KEEP" ] && rm -rf "$A"
grep -q ">>> FAIL" "$L/asan.log" && exit 1 || exit 0
