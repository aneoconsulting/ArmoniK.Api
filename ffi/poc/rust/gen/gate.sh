#!/usr/bin/env bash
# The rust slice's CORRECTNESS gate, and nothing timed (README section 1.1: a container
# timing is instrumentation). One command, one log. Every step is a check with a verdict;
# the script stops at the first step that fails.
#
#   1  the generators are current (this slice's and the shared core's), and one core
#   2  the core's unit tests (ak-rt reader, ak-core transcoders)
#   3  byte identity: every arm against ffi/schema's validated manifest, 16 payloads
#   4  explicit presence, the oneof, the unknown-field vectors
#   5  crossing COUNTS (counting build), push and pull, and the record==reverse control
#   6  the content sets on every payload (correctness section only)
#   7  obligation 12.5's concurrency suite: four builds, two must pass, two must fail
#   8  ABI v1 section 3: the lifecycle, init-guard off and on
#   9  R-D1: the length-wrap reproductions, each under `timeout`
#  10  R-D6: the sticky error slot after every upcall
#  11  the conformance corpus through the C ABI and core-native, BOTH unknown-field modes
#      (gen/corpus.sh), when it exists
#
# Own target dirs, all under poc/rust. `--tsan` also runs gen/tsan.sh (nightly).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
export AK_NO_TIMING=1
step() { echo; echo "===== $* ====="; }

step "0. configuration"
rustc --version
echo "commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/rust or poc/codec')"
nproc; grep -m1 'model name' /proc/cpuinfo || true

step "1. generators current, one core"
python3 gen/generate.py --check 2>/dev/null
# The shared core and its guard. `--core-only`: the other slices' generated trees are
# gated by their own gates (and by the one command, `generate.py --check`, run on its own).
python3 ../codec/gen/generate.py --check --core-only 2>/dev/null
../codec/gen/one_core.sh | tail -2

step "2. core unit tests (debug build: std's precondition checks on)"
( cd ../codec && CARGO_TARGET_DIR="$HERE/../target-codec-test" cargo test -q -p ak-core -p ak-rt 2>&1 \
    | grep -E "test result|FAILED|panicked" )

step "3. byte identity against the validated manifest"
cargo run --release -q -p harness --bin conformance 2>/dev/null | tail -25

step "4. presence, oneof, unknown-field vectors"
cargo run --release -q -p harness --bin shapes 2>/dev/null | tail -25

step "5. crossing counts (counting build)"
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p harness --features count --bin counts 2>/dev/null
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p harness --features count --bin pullbench 2>/dev/null

step "6. content sets, every payload (correctness section)"
cargo run --release -q -p harness --bin contentall 2>/dev/null

step "7. concurrency suite, four builds"
gen/concur.sh 2>/dev/null | grep -vE "^\s*$"

step "8. lifecycle"
gen/lifecycle.sh 2>/dev/null

step "9. R-D1 length-wrap reproductions"
gen/rd1_repro.sh gate

step "10. R-D6 sticky error slot"
cargo run --release -q -p harness --bin stickyerr 2>/dev/null

if [ -x gen/corpus.sh ]; then
  step "11. the conformance corpus, C ABI and core-native, drop and retain"
  gen/corpus.sh
fi

step "11b. the campaign harness's in-process pre-check (CAMPAIGN.md 26): every timed arm, every input"
( B=$(cargo bench -q -p campaign --bench codec_suite --no-run --message-format=json 2>/dev/null \
      | python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1)
  CRITERION_HOME="$(mktemp -d)" AK_PRECHECK_ONLY=1 "$B" 2>&1 | grep -E "^# precheck|PRECHECK" )

step "11c. crossing counts of every timed core-ffi case vs gen/crossings.txt (CAMPAIGN.md 19)"
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p campaign --features count --bin crossings 2>/dev/null \
  | diff -q gen/crossings.txt - >/dev/null && echo "  $(wc -l < gen/crossings.txt) lines identical" \
  || { echo "  crossing counts DIFFER from gen/crossings.txt"; exit 1; }

step "12. the NO-UNKNOWN variant (WP5 step 10; CAMPAIGN.md req 10): unknown-field support compiled out"
# Its own build and target directory (a shared target would overwrite libak_core.so).
( export CARGO_TARGET_DIR="$PWD/target-nounk"
  echo "  build: campaign --no-default-features --features init-guard (ak-core/ak-abi without unknown-fields)"
  B=$(cargo bench -q -p campaign --no-default-features --features init-guard --bench codec_suite --no-run --message-format=json 2>/dev/null \
      | python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1)
  L=$(ldd "$B" | grep -o '/[^ ]*libak_core.so')
  echo "  core: $L -- ak_uencode_* exports: $(nm -D --defined-only "$L" | grep -c ' T ak_uencode_'), ak_dec_reset_* exports: $(nm -D --defined-only "$L" | grep -c ' T ak_dec_reset_') (both must be 0)"
  [ "$(nm -D --defined-only "$L" | grep -cE ' T ak_(uencode|uelem|dec_reset)_')" = 0 ] || { echo "  the no-unknown core exports the u-family"; exit 1; }
  CRITERION_HOME="$(mktemp -d)" AK_PRECHECK_ONLY=1 "$B" 2>&1 | grep -E "^# precheck|PRECHECK" )
echo "  byte identity and the shape vectors (incl. absent-path and unknown-field) on the no-unknown build:"
for bin in conformance shapes; do
  CARGO_TARGET_DIR="$PWD/target-nounk" cargo run --release -q -p harness --no-default-features --features guard,init-guard --bin $bin 2>/dev/null \
    | grep -E "^VERDICT" | sed "s/^/    $bin: /"
done
CARGO_TARGET_DIR="$PWD/target-count-nounk" cargo run --release -q -p campaign --no-default-features --features count,init-guard --bin crossings 2>/dev/null \
  | diff -q gen/crossings-nounk.txt - >/dev/null && echo "  no-unknown crossing counts: $(wc -l < gen/crossings-nounk.txt) lines identical to gen/crossings-nounk.txt" \
  || { echo "  no-unknown crossing counts DIFFER from gen/crossings-nounk.txt"; exit 1; }

echo "  the C header's two variants (poc/codec/gen/c_abi.py), each against its core:"
cargo build --release -q -p campaign --bin crossings 2>/dev/null   # the full core, default target
gen/c_variant.sh target/release/deps/libak_core.so target-nounk/release/deps/libak_core.so

if [ "${1:-}" = "--tsan" ]; then
  step "13. ThreadSanitizer over the concurrency suite"
  gen/tsan.sh 2>/dev/null
fi

echo
echo "GATE PASSED"
