#!/usr/bin/env bash
# D19 (owner, 2026-10-04): the checks the simdutf transcoder unit asks for, NOT the full gate.
# Each step is the gate's own command (gen/gate.sh), run on the D19 core:
#
#   1   generators current, one core                         (gate step 1)
#   2   ak-core / ak-rt unit tests, debug build               (gate step 2; includes d19_utf16_tests)
#   3,4 byte identity, presence / oneof / unknown vectors     (gate steps 3, 4)
#   11  the corpus, full and no-unknown builds, with controls (gate step 11, gen/corpus.sh)
#   11b the codec harness pre-check, full build               (gate step 11b)
#   11c crossing counts vs gen/crossings.txt                  (gate step 11c)
#   12  no-unknown build: pre-check, conformance, shapes, crossing counts vs gen/crossings-nounk.txt
#   D19 the differential (bin tc16_diff) against the pre-D19 core binary, AK_D19_SCALE (default 8)
#
# Not run (the full gate's remaining steps): 5, 6, 7 (concurrency), 8, 9, 10, 11d-11f, 12's
# RPC checks and C header pair, 12b. No step here is timed. Stops at the first failure.
#
#   gen/d19_checks.sh <pre-D19 libak_core.so>
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
BASE_CORE="${1:?usage: gen/d19_checks.sh <pre-D19 libak_core.so>}"
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
echo "pre-D19 core: $BASE_CORE sha256 $(sha256sum "$BASE_CORE" | cut -c1-16)"

step "1. generators current, one core"
python3 gen/generate.py --check 2>/dev/null
python3 ../codec/gen/generate.py --check --core-only 2>/dev/null
../codec/gen/one_core.sh | tail -2

step "2. core unit tests (debug build: std's precondition checks on)"
( cd ../codec && CARGO_TARGET_DIR="$HERE/../target-codec-test" cargo test -q -p ak-core -p ak-rt 2>&1 \
    | grep -E "test result|FAILED|panicked" )

step "3. byte identity against the validated manifest"
cargo run --release -q -p harness --bin conformance 2>/dev/null | tail -25

step "4. presence, oneof, unknown-field vectors"
cargo run --release -q -p harness --bin shapes 2>/dev/null | tail -25

step "D19. the differential: ak_tc_utf16 (simdutf) vs ak_tc_utf16_scalar vs the pre-D19 core vs std; the exports; an encode context vs prost"
cargo build --release -q -p harness --bin tc16_diff 2>/dev/null
L=$(ldd "$CARGO_TARGET_DIR/release/tc16_diff" | grep -o '/[^ ]*libak_core.so')
echo "  core loaded: $L ($(nm -D --defined-only "$L" | grep -cE ' T ak_(utf16_to_utf8|utf8_to_utf16|tc_utf16_scalar)$') of the 3 D19 marker exports)"
AK_D19_BASE_CORE="$BASE_CORE" AK_D19_SCALE="${AK_D19_SCALE:-8}" "$CARGO_TARGET_DIR/release/tc16_diff"

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
  echo "  core: $L -- ak_uencode_* exports: $(nm -D --defined-only "$L" | grep -c ' T ak_uencode_') (must be 0); D19 exports: $(nm -D --defined-only "$L" | grep -cE ' T ak_(utf16_to_utf8|utf16_utf8_len|utf8_to_utf16|utf8_utf16_len|utf8_validate|utf16_validate|tc_utf16_scalar)$') (7)"
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
echo "D19 CHECKS PASSED"
