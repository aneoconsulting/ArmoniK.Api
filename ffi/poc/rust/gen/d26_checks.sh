#!/usr/bin/env bash
# FIX-PLAN D26 (owner, 2026-10-10, as amended): no thread-local storage on a timed path; the
# binding's host state lives in a host context per binding context. The fast checks of that
# change (ffi/CLAUDE.md "Checks, not gates, per change"); nothing timed. Stops at the first
# failure.
#
#   T1  gen/no_tls.py --plant: no thread-local outside the allowed set (counting tallies,
#       the FSM differential's test arena, shapes-values' content-set selection), and the
#       planted control is found
#   T2  generators current (this slice), and the shared core
#   T3  byte identity (conformance), the shape vectors (shapes), content sets (contentall),
#       full build; conformance and shapes on the no-unknown build
#   T4  the counting build's harness counts (counts, pullbench) EQUAL those of the committed
#       tree (HEAD, built in a shadow copy): the change moved no crossing
#   T5  the campaign's crossing counts vs gen/crossings.txt and gen/crossings-nounk.txt
#   T6  the campaign pre-check (every timed arm, every input), full and no-unknown builds
#   T7  the FSM differential and the corpus differential (gen/fsm_checks.sh, F1-F6)
#   T8  reset on entry (gen/d27_checks.sh), roe_check both builds and its plants
#   T9  the corpus, both builds, with its controls and decision 11's (gen/corpus.sh)
#   T10 the concurrency suite, four builds (gen/concur.sh)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
export AK_NO_TIMING=1
T="${TMPDIR:-/tmp}/ak-d26-checks.$$"
mkdir -p "$T"
trap 'rm -rf "$T"' EXIT
step() { echo; echo "===== $* ====="; }

step "0. configuration"
rustc --version
echo "commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/rust or poc/codec')"

step "T1. no thread-local on a timed path (gen/no_tls.py --plant)"
python3 gen/no_tls.py --plant

step "T2. generators current"
python3 gen/generate.py --check 2>/dev/null && echo "  rust slice --check: exit 0"
python3 ../codec/gen/generate.py --check --core-only 2>/dev/null && echo "  shared core --check --core-only: exit 0"

step "T3. byte identity, shape vectors, content sets"
cargo run --release -q -p harness --bin conformance 2>/dev/null | grep -E "^VERDICT"
cargo run --release -q -p harness --bin shapes 2>/dev/null | grep -E "^VERDICT"
cargo run --release -q -p harness --bin contentall 2>/dev/null | tail -3
for bin in conformance shapes; do
  CARGO_TARGET_DIR="$PWD/target-nounk" cargo run --release -q -p harness --no-default-features --features guard,init-guard --bin $bin 2>/dev/null \
    | grep -E "^VERDICT" | sed "s/^/  no-unknown $bin: /"
done

step "T4. harness crossing counts, this tree vs the committed tree (HEAD) in a shadow copy"
R=$(git rev-parse --show-toplevel)
SH="$PWD/target-d26-head/tree"
rm -rf "$SH"; mkdir -p "$SH"
git -C "$R" archive HEAD ffi/poc/codec ffi/poc/rust | tar -C "$SH" -xf -
for d in ffi/schema ffi/corpus packages; do ln -sfn "$R/$d" "$SH/$d"; done
for bin in counts pullbench; do
  CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p harness --features count --bin $bin > "$T/$bin.now" 2>/dev/null
  ( cd "$SH/ffi/poc/rust" && CARGO_TARGET_DIR="$HERE/../target-d26-head/build" cargo run --release -q -p harness --features count --bin $bin ) > "$T/$bin.head" 2>/dev/null
  if diff -q "$T/$bin.head" "$T/$bin.now" >/dev/null; then
    echo "  $bin: $(wc -l < "$T/$bin.now") lines, identical to HEAD's"
  else
    diff "$T/$bin.head" "$T/$bin.now" | head -30; echo "  $bin DIFFERS from HEAD's"; exit 1
  fi
done

step "T5. campaign crossing counts vs the committed tables"
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p campaign --features count --bin crossings 2>/dev/null \
  | diff -q gen/crossings.txt - >/dev/null && echo "  full: $(wc -l < gen/crossings.txt) lines identical to gen/crossings.txt" \
  || { echo "  crossing counts DIFFER from gen/crossings.txt"; exit 1; }
CARGO_TARGET_DIR="$PWD/target-count-nounk" cargo run --release -q -p campaign --no-default-features --features count,init-guard --bin crossings 2>/dev/null \
  | diff -q gen/crossings-nounk.txt - >/dev/null && echo "  no-unknown: $(wc -l < gen/crossings-nounk.txt) lines identical to gen/crossings-nounk.txt" \
  || { echo "  no-unknown crossing counts DIFFER from gen/crossings-nounk.txt"; exit 1; }

step "T6. the campaign pre-check, full and no-unknown builds"
precheck() {  # target dir, cargo flags...
  local dir=$1; shift
  local B
  B=$(CARGO_TARGET_DIR="$dir" cargo bench -q -p campaign "$@" --bench codec_suite --no-run --message-format=json 2>/dev/null \
      | python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1)
  CRITERION_HOME="$(mktemp -d)" AK_PRECHECK_ONLY=1 "$B" 2>&1 | grep -E "^# precheck|PRECHECK"
}
precheck "$PWD/target"
precheck "$PWD/target-nounk" --no-default-features --features init-guard

step "T7. the FSM decode family (gen/fsm_checks.sh)"
gen/fsm_checks.sh

step "T8. reset on entry (gen/d27_checks.sh)"
gen/d27_checks.sh

step "T9. the corpus, both builds (gen/corpus.sh)"
gen/corpus.sh

step "T10. the concurrency suite, four builds (gen/concur.sh)"
gen/concur.sh 2>/dev/null | grep -E ">>>|loaded:|behaved as required|did not|PASS|FAIL"

echo
echo "D26 CHECKS PASSED"
