#!/usr/bin/env bash
# D20 (owner, 2026-10-04): the checks the UTF-8 skip-bits unit asks for, NOT the full gate.
# Each numbered step is the gate's own command (gen/gate.sh), run on the D20 core:
#
#   1   generators current (every slice), one core            (gate step 1, all slices)
#   2   ak-core / ak-rt unit tests, debug build               (gate step 2; includes d20_utf8_skip_tests)
#   D20 the core's D20 tests on all four plans (shapes, shapes no-unknown, corpus, corpus
#       no-unknown) and three PLANTED defects in the generated codec that they must catch
#   3,4 byte identity, presence / oneof / unknown vectors     (gate steps 3, 4)
#   5   crossing counts, push and pull, record==reverse       (gate step 5)
#   9,10 the length-wrap reproductions and the sticky error slot (gate steps 9, 10: their
#       hand-written vtables gained the mask)
#   11  the corpus, full and no-unknown builds, with controls (gate step 11, gen/corpus.sh)
#   11b the codec harness pre-check, full build               (gate step 11b)
#   11c crossing counts vs gen/crossings.txt                  (gate step 11c)
#   12  no-unknown build: pre-check, conformance, shapes, crossing counts vs gen/crossings-nounk.txt
#
# Not run (the full gate's remaining steps): 6, 7 (concurrency), 8, 11d-11f, 12's RPC checks
# and C header pair, 12b. No step here is timed. Stops at the first failure.
#
#   gen/d20_checks.sh
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
export AK_NO_TIMING=1
step() { echo; echo "===== $* ====="; }
exe_of() {  # the codec_suite bench executable from cargo's JSON on stdin
  python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1
}

step "0. configuration"
rustc --version
echo "commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/rust or poc/codec')"
nproc; grep -m1 'model name' /proc/cpuinfo || true

step "1. generators current (every slice), one core"
python3 ../codec/gen/generate.py --check 2>/dev/null | grep -vE "^ok " | sed 's/^/  /'
../codec/gen/one_core.sh | tail -2

step "2. core unit tests (debug build: std's precondition checks on)"
( cd ../codec && CARGO_TARGET_DIR="$HERE/../target-codec-test" cargo test -q -p ak-core -p ak-rt 2>&1 \
    | grep -E "test result|FAILED|panicked" )

step "D20. the core's D20 tests on every plan, and three planted defects they must catch"
TT="$HERE/../target-codec-test"
for v in "" "--no-default-features" "--features corpus" "--no-default-features --features corpus"; do
  r=$( cd ../codec && CARGO_TARGET_DIR="$TT" cargo test -q -p ak-core --lib $v d20 -- --nocapture 2>&1 \
       | grep -E "d20 every|test result" | tr '\n' ' ' )
  echo "  ak-core ${v:-(default)}: $r"
  echo "$r" | grep -q "2 passed; 0 failed" || { echo "  D20 TESTS FAILED"; exit 1; }
done
CODEC=../codec/crates/ak-core/src/generated/codec.rs
SAVE=$(mktemp)
cp "$CODEC" "$SAVE"
restore() { cp "$SAVE" "$CODEC"; rm -f "$SAVE"; }
trap restore EXIT
plant() {  # name, python expression over s (the generated codec's text)
  cp "$SAVE" "$CODEC"
  python3 - "$CODEC" "$2" <<'PY'
import sys
f, expr = sys.argv[1], sys.argv[2]
s = open(f).read()
t = eval(expr)
assert t != s, "the plant changed nothing"
open(f, "w").write(t)
PY
  r=$( cd ../codec && CARGO_TARGET_DIR="$TT" cargo test -q -p ak-core --lib d20 2>&1 | grep -E "test result" || true )
  if echo "$r" | grep -q " 0 failed"; then echo "  PLANT NOT CAUGHT: $1 ($r)"; exit 1; fi
  echo "  planted $1: caught ($r)"
}
plant "one string field tests its neighbour's bit (bit 13 -> 14)" "s.replace('sk & (1u64 << 13) == 0', 'sk & (1u64 << 14) == 0', 1)"
plant "the first pull entry ignores the context's pvt" "s.replace('let sk: u64 = (*dcx).pvt_utf8_skip;', 'let sk: u64 = 0;', 1)"
plant "a child's numbering shifted by one (sk >> 18 -> 17)" "s.replace('sk >> 18', 'sk >> 17', 1)"
restore; trap - EXIT
python3 ../codec/gen/generate.py --check --core-only 2>/dev/null | grep -c "^ok" | sed 's/^/  generated core restored, files ok: /'

step "3. byte identity against the validated manifest"
cargo run --release -q -p harness --bin conformance 2>/dev/null | tail -25

step "4. presence, oneof, unknown-field vectors"
cargo run --release -q -p harness --bin shapes 2>/dev/null | tail -25

step "5. crossing counts (counting build)"
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p harness --features count --bin counts 2>/dev/null
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p harness --features count --bin pullbench 2>/dev/null

step "9. R-D1 length-wrap reproductions"
gen/rd1_repro.sh gate

step "10. R-D6 sticky error slot"
cargo run --release -q -p harness --bin stickyerr 2>/dev/null
cargo run --release -q -p harness --bin fresh_enc 2>/dev/null

if [ -x gen/corpus.sh ]; then
  step "11. the conformance corpus, C ABI and core-native, drop and retain"
  gen/corpus.sh
fi

step "11b. the campaign harness's in-process pre-check (CAMPAIGN.md 26): every timed arm, every input"
( B=$(cargo bench -q -p campaign --bench codec_suite --no-run --message-format=json 2>/dev/null | exe_of)
  CRITERION_HOME="$(mktemp -d)" AK_PRECHECK_ONLY=1 "$B" 2>&1 | grep -E "^# precheck|PRECHECK" )

step "11c. crossing counts of every timed core-ffi case vs gen/crossings.txt (CAMPAIGN.md 19)"
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p campaign --features count --bin crossings 2>/dev/null \
  | diff -q gen/crossings.txt - >/dev/null && echo "  $(wc -l < gen/crossings.txt) lines identical" \
  || { echo "  crossing counts DIFFER from gen/crossings.txt"; exit 1; }

step "12. the NO-UNKNOWN variant: pre-check, byte identity, shape vectors, crossing counts"
( export CARGO_TARGET_DIR="$PWD/target-nounk"
  B=$(cargo bench -q -p campaign --no-default-features --features init-guard --bench codec_suite --no-run --message-format=json 2>/dev/null | exe_of)
  L=$(ldd "$B" | grep -o '/[^ ]*libak_core.so')
  echo "  core: $L -- ak_uencode_* exports: $(nm -D --defined-only "$L" | grep -c ' T ak_uencode_') (must be 0); D20 pvt setters: $(nm -D --defined-only "$L" | grep -c ' T ak_dec_set_pvt_') (7)"
  [ "$(nm -D --defined-only "$L" | grep -cE ' T ak_(uencode|uelem|dec_reset)_')" = 0 ] || { echo "  the no-unknown core exports the u-family"; exit 1; }
  CRITERION_HOME="$(mktemp -d)" AK_PRECHECK_ONLY=1 "$B" 2>&1 | grep -E "^# precheck|PRECHECK" )
for bin in conformance shapes; do
  CARGO_TARGET_DIR="$PWD/target-nounk" cargo run --release -q -p harness --no-default-features --features guard,init-guard --bin $bin 2>/dev/null \
    | grep -E "^VERDICT" | sed "s/^/    $bin: /"
done
CARGO_TARGET_DIR="$PWD/target-count-nounk" cargo run --release -q -p campaign --no-default-features --features count,init-guard --bin crossings 2>/dev/null \
  | diff -q gen/crossings-nounk.txt - >/dev/null && echo "  no-unknown crossing counts: $(wc -l < gen/crossings-nounk.txt) lines identical to gen/crossings-nounk.txt" \
  || { echo "  no-unknown crossing counts DIFFER from gen/crossings-nounk.txt"; exit 1; }

echo
echo "D20 CHECKS PASSED"
