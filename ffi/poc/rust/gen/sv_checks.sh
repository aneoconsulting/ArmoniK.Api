#!/usr/bin/env bash
# Static decode vtables (owner, 2026-10-04): the gate's own commands on the regenerated
# bindings (one `static VT` per root in the Rust binding, one `static const k_dvt_<Root>` in the
# C++ one), NOT the full gate. Steps as gen/d20_checks.sh without its D20 plant section (the core
# is unchanged), plus:
#   SV  every regenerated C++ binding assigns each vtable member the function the per-call code
#       assigned (gen/sv_vtmap.py); the Rust bindings hold 7 `static VT` and no per-call vtable;
#       the codec_suite executables of both builds carry the 7 statics (nm); bin d20_toggle on
#       both builds (the mask the static passes is 0: a malformed session_id is rejected)
# Not run: the full gate's steps 6, 7, 8, 11d-11f, 12's RPC checks and C header pair, 12b.
# Nothing here is timed. Stops at the first failure.
#
#   gen/sv_checks.sh
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

step "SV. the static vtables are what the build runs"
python3 gen/sv_vtmap.py | tail -1
for f in crates/harness/src/generated/binding.rs crates/harness/src/generated/binding_nounk.rs \
         corpus/crates/harness/src/generated/binding.rs corpus/crates/harness/src/generated/binding_nounk.rs; do
  echo "  $f: static VT $(grep -c '^    static VT: ak_dvt_' "$f"), per-call 'let vt = ak_dvt_' $(grep -c 'let vt = ak_dvt_' "$f")"
  [ "$(grep -c 'let vt = ak_dvt_' "$f")" = 0 ] || { echo "  a per-call vtable is left"; exit 1; }
done
for td in target target-nounk; do
  fl=(); [ $td = target-nounk ] && fl=(--no-default-features --features init-guard)
  B=$(CARGO_TARGET_DIR="$PWD/$td" cargo bench -q -p campaign "${fl[@]}" --bench codec_suite --no-run --message-format=json 2>/dev/null | exe_of)
  echo "  $td codec_suite: $(nm -C "$B" | grep -cE ' [dDrR] .*decode_with_[a-z_]+(_armed)?::VT$') decode-vtable statics in a data section (nm -C, 7 roots)"
done
CARGO_TARGET_DIR="$PWD/target" AK_D20_EXPECT=reject cargo run --release -q -p harness --bin d20_toggle 2>/dev/null | sed 's/^/  full: /'
CARGO_TARGET_DIR="$PWD/target-nounk" AK_D20_EXPECT=reject cargo run --release -q -p harness --no-default-features --features guard,init-guard --bin d20_toggle 2>/dev/null | sed 's/^/  no-unknown: /'

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
echo "STATIC-VTABLE CHECKS PASSED"
