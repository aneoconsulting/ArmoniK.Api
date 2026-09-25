#!/usr/bin/env bash
# FIX-PLAN WP5 item 6.1: the conformance corpus through the C ABI and core-native, unknown
# fields dropped AND retained (four arms), every row in its own process under a timeout.
#
#   1  generators current (the corpus core, facade, native, binding, dispatch)
#   2  build poc/rust/corpus/ (its own workspace: ak-core --features corpus,init-guard)
#   3  the whole corpus, four arms: must PASS
#   4  controls, each of which MUST FAIL, so a pass above means something:
#        proj    a planted key in every projection        -> C2 must fail
#        reenc   a byte appended to every re-encoding     -> C3 must fail
#        accept  every refusal turned into an acceptance  -> C4 must fail
#        noinit  ak_init skipped (the core checks)        -> every C ABI arm must fail
#   5  decision 11's controls (corpus --unk-controls) and their plant, which must fail
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
export CARGO_TARGET_DIR="$PWD/target-corpus"
echo "# commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/rust or poc/codec'); rustc $(rustc --version | awk '{print $2}')"

echo "===== 1. generators current ====="
python3 gen/generate.py --check 2>/dev/null | grep -E "corpus|STALE" || true
python3 gen/generate.py --check >/dev/null 2>&1 || { echo "STALE generated files"; exit 1; }

echo "===== 2. build ====="
( cd corpus && cargo build --release -q 2>/dev/null )
BIN="$CARGO_TARGET_DIR/release/corpus"
echo "# $(ldd "$BIN" | grep -o 'libak_core.so => [^ ]*')"
echo "# the loaded core is the corpus-schema build: $(nm -D --defined-only "$(ldd "$BIN" | grep -o '/[^ ]*libak_core.so')" | grep -c ' T ak_decode_WireZoo') ak_decode_WireZoo export(s)"

echo "===== 3. the corpus, four arms ====="
"$BIN"

echo "===== 4. controls (each MUST FAIL) ====="
SUB="S-Probe,U-root,X-lenwrap-lrr,E-map,T-dec-root"
bad=0
for p in proj reenc accept; do
  if AK_CORPUS_PLANT=$p "$BIN" --only "$SUB" > /tmp/corpus-ctl.$$ 2>&1; then
    echo "  control $p: PASSED -- the harness is blind to it"; bad=$((bad+1))
  else
    echo "  control $p: failed as required ($(grep -c 'FAIL ' /tmp/corpus-ctl.$$) arm-row failures)"
  fi
done
if AK_CORPUS_SKIP_INIT=1 "$BIN" --only "$SUB" > /tmp/corpus-ctl.$$ 2>&1; then
  echo "  control noinit: PASSED -- init-guard is not in the build"; bad=$((bad+1))
else
  echo "  control noinit: failed as required: $(grep -c 'FAIL .*\[ffi-' /tmp/corpus-ctl.$$) C ABI arm-row failures, $(grep -c 'FAIL .*\[native-' /tmp/corpus-ctl.$$) native (native has no ak_init to skip)"
  grep -m2 'FAIL .*\[ffi-' /tmp/corpus-ctl.$$ | cut -c1-160 | sed 's/^/    /'
fi
rm -f /tmp/corpus-ctl.$$

echo "===== 5. decision 11 (WP5 step 7): per-position discard, pull == push, placement ====="
if "$BIN" --unk-controls > /tmp/corpus-unk.$$ 2>&1; then
  grep -E "^rows|^discard|placement|U-map-entry|U-leaf-all|U-deep-all" /tmp/corpus-unk.$$
else
  cat /tmp/corpus-unk.$$; echo "  decision 11 controls FAILED"; bad=$((bad+1))
fi
if "$BIN" --unk-controls --plant > /tmp/corpus-unk.$$ 2>&1; then
  echo "  control unk-plant: PASSED -- the discard comparison is blind"; bad=$((bad+1))
else
  echo "  control unk-plant (bags not cleared): failed as required: $(grep '^discard' /tmp/corpus-unk.$$)"
fi
rm -f /tmp/corpus-unk.$$
[ $bad -eq 0 ] || { echo "CONTROLS FAILED: $bad"; exit 1; }
echo "CORPUS GATE PASSED"
