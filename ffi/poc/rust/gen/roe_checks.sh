#!/usr/bin/env bash
# reset-on-entry (owner, 2026-10-10): the checks of the measurement experiment. Nothing timed.
# The core feature `reset-on-entry` is default OFF; the variant is the core built with it and
# the bindings rendered for it (binding_roe*.rs, harness / campaign / corpus feature
# `reset-on-entry`). Stops at the first failure.
#
#   0   configuration
#   1   generators current (this slice, the core, the C# slice's default output)
#   2   the DEFAULT build unchanged: the core's shared object byte-identical to the base's in
#       four feature sets (logs/rust/opt/reset-on-entry/so-base.txt); the default bindings and
#       C# output identical to BASE; the default crossing counts identical to gen/crossings*.txt
#   3   core and runtime unit tests with the feature (debug)
#   4   variant: the FSM differential (FSM == pull records, FSM graph == push), full and no-unknown
#   5   variant: the codec suite's pre-check, full grid (push == FSM == pull == native, every
#       input and mode; encode variants), both builds
#   6   variant: byte identity and the shape vectors (harness bins)
#   7   roe_check (ABANDON, ERR, ENC; variant also HOSTERR and REARM with its control), default
#       and variant, both builds; the lines both builds print are identical
#   8   variant corpus: six arms (three no-unknown), the FSM corpus differential both builds,
#       decision 11's controls and their plant; the controls' output equal to the default build's
#   9   variant crossing counts vs gen/crossings-roe.txt / -nounk, and the change against the
#       default files (forward = default forward - default resets on every row)
#   10  variant RPC: the upload byte check and the section 9 semantics (cells C, D encode on
#       the variant core)
#
#   gen/roe_checks.sh BASE_COMMIT
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
BASE=${1:?base commit}
export AK_NO_TIMING=1
LD="$(cd "$HERE/../../.." && pwd)/logs/rust/opt/reset-on-entry"
step() { echo; echo "===== $* ====="; }
exe_of() {
  python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1
}
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT

step "0. configuration"
rustc --version
echo "commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/rust or poc/codec'); base $BASE"
nproc; uptime

step "1. generators current"
python3 gen/generate.py --check >/dev/null 2>&1 && echo "  this slice: current ($(python3 gen/generate.py --check 2>/dev/null | grep -c '^ok') files)" || { echo "  STALE"; exit 1; }
python3 ../codec/gen/generate.py --check --core-only >/dev/null 2>&1 && echo "  the core: current" || { echo "  core STALE"; exit 1; }
( cd ../csharp && python3 gen/generate.py --check >/dev/null 2>&1 ) && echo "  the C# slice's generated output: current (cs_host.py's option off = its committed text)" || { echo "  C# output STALE"; exit 1; }

step "2. the default build unchanged"
SO="$PWD/target-roe-so"
( cd ../codec
  for fs in "full:--features rpc,init-guard" "nounk:--no-default-features --features rpc,init-guard" "corpus:--features corpus,init-guard" "count:--features count,rpc,init-guard"; do
    n=${fs%%:*}; f=${fs#*:}
    CARGO_TARGET_DIR="$SO/$n" cargo build --release -q -p ak-core $f 2>/dev/null
    h=$(sha256sum "$SO/$n/release/libak_core.so" | cut -c1-64); b=$(awk -v n="$n" '$2 == n { print $3 }' "$LD/so-base.txt")
    [ "$h" = "$b" ] && echo "  libak_core.so ($n): byte-identical to the base ($(echo $h | cut -c1-16))" || { echo "  libak_core.so ($n) DIFFERS from the base: $h vs $b"; exit 1; }
  done )
R=$(git rev-parse --show-toplevel)
for p in ffi/poc/rust/crates/harness/src/generated/binding.rs ffi/poc/rust/crates/harness/src/generated/binding_nounk.rs \
         ffi/poc/rust/corpus/crates/harness/src/generated/binding.rs ffi/poc/rust/corpus/crates/harness/src/generated/binding_nounk.rs \
         ffi/poc/rust/crates/campaign/src/generated/roots.rs ffi/poc/rust/corpus/crates/harness/src/generated/dispatch.rs; do
  ( cd "$R" && git diff --quiet "$BASE" -- "$p" ) && echo "  $p: identical to $BASE" || { echo "  $p CHANGED since $BASE"; exit 1; }
done
( cd "$R" && git diff --quiet "$BASE" -- ffi/poc/csharp/src ) && echo "  ffi/poc/csharp/src: identical to $BASE" || echo "  ffi/poc/csharp/src changed since $BASE (by the C# slice; its --check above passed)"
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p campaign --features count --bin crossings 2>/dev/null > "$T/c"
diff -q gen/crossings.txt "$T/c" >/dev/null && echo "  default crossing counts: identical to gen/crossings.txt ($(wc -l < "$T/c") lines)" || { echo "  default crossing counts DIFFER"; exit 1; }
CARGO_TARGET_DIR="$PWD/target-count-nounk" cargo run --release -q -p campaign --no-default-features --features count,init-guard --bin crossings 2>/dev/null > "$T/cn"
diff -q gen/crossings-nounk.txt "$T/cn" >/dev/null && echo "  default crossing counts (no-unknown): identical to gen/crossings-nounk.txt ($(wc -l < "$T/cn") lines)" || { echo "  DIFFER"; exit 1; }

step "3. core and runtime unit tests with reset-on-entry (debug build)"
( cd ../codec && CARGO_TARGET_DIR="$HERE/../target-codec-test-roe" cargo test -q -p ak-core -p ak-rt --features ak-core/reset-on-entry 2>&1 | grep -E "test result|FAILED|panicked" )

step "4. variant: the FSM differential, full and no-unknown"
for t in "target-roe:" "target-roe-nounk:--no-default-features --features init-guard"; do
  d=${t%%:*}; f=${t#*:}
  CARGO_TARGET_DIR="$PWD/$d" cargo build --release -q -p campaign --features reset-on-entry $f --bin fsm_diff 2>/dev/null
  echo "  $d: loads $(ldd "$d/release/fsm_diff" | grep -o '/[^ ]*libak_core.so') ($(nm -D --defined-only "$(ldd "$d/release/fsm_diff" | grep -o '/[^ ]*libak_core.so')" | grep -c ' T ak_measure_reset_on_entry') reset-on-entry marker)"
  "$d/release/fsm_diff" > "$T/f" 2>&1 || { tail -30 "$T/f"; echo "  FSM DIFFERENTIAL FAILED"; exit 1; }
  grep -E "checks|PASSED" "$T/f" | tail -2 | sed 's/^/  /'
done

step "5. variant: the codec suite's pre-check (full grid), both builds"
for t in "target-roe:" "target-roe-nounk:--no-default-features --features init-guard"; do
  d=${t%%:*}; f=${t#*:}
  B=$(CARGO_TARGET_DIR="$PWD/$d" cargo bench -q -p campaign --features reset-on-entry $f --bench codec_suite --no-run --message-format=json 2>/dev/null | exe_of)
  CRITERION_HOME="$(mktemp -d)" AK_CAMPAIGN_GRID=full AK_PRECHECK_ONLY=1 "$B" 2>&1 | grep -E "^# precheck|PRECHECK" | sed "s/^/  $d: /"
done

step "6. variant: byte identity and the shape vectors"
for bin in conformance shapes; do
  CARGO_TARGET_DIR="$PWD/target-roe" cargo run --release -q -p harness --features reset-on-entry --bin $bin 2>/dev/null | grep -E "^VERDICT" | sed "s/^/  $bin: /"
done

step "7. roe_check: default and variant, both builds"
for t in "target::" "target-nounk::--no-default-features --features init-guard" "target-roe::--features reset-on-entry" "target-roe-nounk::--no-default-features --features init-guard,reset-on-entry"; do
  d=${t%%::*}; f=${t#*::}
  CARGO_TARGET_DIR="$PWD/$d" cargo build --release -q -p campaign $f --bin roe_check 2>/dev/null
  "$d/release/roe_check" > "$T/r-$d" 2>&1 || { grep -E "^FAIL|^#" "$T/r-$d" | head -20; echo "  ROE CHECK FAILED ($d)"; exit 1; }
  cp "$T/r-$d" "$LD/checks/roe_check-$d.txt"
  grep -E "^#" "$T/r-$d" | sed "s/^/  $d: /"
done
for p in "target target-roe" "target-nounk target-roe-nounk"; do
  set -- $p
  diff <(grep -E "^(ABANDON|ERR|ENC) " "$T/r-$1") <(grep -E "^(ABANDON|ERR|ENC) " "$T/r-$2") >/dev/null \
    && echo "  $1 vs $2: the $(grep -cE '^(ABANDON|ERR|ENC) ' "$T/r-$1") ABANDON / ERR / ENC lines identical (today's behaviour = the variant's)" \
    || { echo "  $1 vs $2: the lines DIFFER"; exit 1; }
done

step "8. variant corpus: six arms, the FSM corpus differential, decision 11's controls"
( cd corpus && CARGO_TARGET_DIR="$HERE/../target-corpus-roe" cargo build --release -q --features reset-on-entry 2>/dev/null )
( cd corpus && CARGO_TARGET_DIR="$HERE/../target-corpus-roe-nounk" cargo build --release -q --no-default-features --features init-guard,reset-on-entry 2>/dev/null )
( cd corpus && CARGO_TARGET_DIR="$HERE/../target-corpus" cargo build --release -q 2>/dev/null )
CB="$PWD/target-corpus-roe/release/corpus"; CN="$PWD/target-corpus-roe-nounk/release/corpus"; CD="$PWD/target-corpus/release/corpus"
echo "  variant core: $(ldd "$CB" | grep -o '/[^ ]*libak_core.so') ($(nm -D --defined-only "$(ldd "$CB" | grep -o '/[^ ]*libak_core.so')" | grep -c ' T ak_measure_reset_on_entry') marker)"
"$CB" > "$T/cb" 2>&1 || { tail -30 "$T/cb"; echo "  VARIANT CORPUS FAILED"; exit 1; }
grep -E "^## |^   pass|CORPUS" "$T/cb" | sed 's/^/  /'
"$CN" > "$T/cn" 2>&1 || { tail -30 "$T/cn"; echo "  VARIANT NO-UNKNOWN CORPUS FAILED"; exit 1; }
grep -E "^## |^   pass|CORPUS" "$T/cn" | sed 's/^/  no-unknown: /'
timeout 900 "$CB" --fsm-diff | grep -E "checks|PASSED|FAIL" | sed 's/^/  /'
timeout 900 "$CN" --fsm-diff | grep -E "checks|PASSED|FAIL" | sed 's/^/  no-unknown: /'
"$CB" --unk-controls > "$T/u-roe" 2>&1 || { cat "$T/u-roe" | tail -20; echo "  decision 11 controls FAILED on the variant"; exit 1; }
"$CD" --unk-controls > "$T/u-def" 2>&1 || { echo "  decision 11 controls FAILED on the default build"; exit 1; }
grep -E "^rows|^discard|placement" "$T/u-roe" | sed 's/^/  /'
# The one case whose expectation the variant changes by definition: R-H20's "decided at arm
# time" (the retain decision made at the reset). The variant re-arms on every decode entry,
# so the decision is made there; unkctl states and checks the variant's expectation instead.
diff <(grep -v "decided at" "$T/u-def") <(grep -v "decided at" "$T/u-roe") >/dev/null && echo "  decision 11 controls: the variant's output identical to the default build's but for the one case below ($(wc -l < "$T/u-roe") lines: discard per position, pull == push, placement cases)" \
  || { diff "$T/u-def" "$T/u-roe" | head -20; echo "  decision 11 controls DIFFER from the default build"; exit 1; }
grep "decided at" "$T/u-def" | sed 's/^ */  default: /'
grep "decided at" "$T/u-roe" | sed 's/^ */  variant: /'
if "$CB" --unk-controls --plant > "$T/u-plant" 2>&1; then echo "  control unk-plant PASSED on the variant: the comparison is blind"; exit 1; else echo "  control unk-plant (bags not cleared): failed as required: $(grep '^discard' "$T/u-plant")"; fi

step "9. variant crossing counts"
CARGO_TARGET_DIR="$PWD/target-roe-count" cargo run --release -q -p campaign --features count,reset-on-entry --bin crossings 2>/dev/null > "$T/rc"
diff -q gen/crossings-roe.txt "$T/rc" >/dev/null && echo "  identical to gen/crossings-roe.txt ($(wc -l < "$T/rc") lines)" || { echo "  DIFFER from gen/crossings-roe.txt"; exit 1; }
CARGO_TARGET_DIR="$PWD/target-roe-count-nounk" cargo run --release -q -p campaign --no-default-features --features count,init-guard,reset-on-entry --bin crossings 2>/dev/null > "$T/rcn"
diff -q gen/crossings-roe-nounk.txt "$T/rcn" >/dev/null && echo "  no-unknown: identical to gen/crossings-roe-nounk.txt ($(wc -l < "$T/rcn") lines)" || { echo "  DIFFER"; exit 1; }
for n in "" "-nounk"; do python3 "$LD/crossings/roe_diff.py" gen/crossings$n.txt gen/crossings-roe$n.txt | grep -E "^rows" | sed "s/^/  crossings$n: /"; done

step "10. variant RPC: uploads and section 9's semantics"
CARGO_TARGET_DIR="$PWD/target-roe" cargo run --release -q -p campaign --features reset-on-entry --bin upload_check 2>/dev/null > "$T/up" || true
tail -1 "$T/up" | grep -q "UPLOAD CHECK PASSED" && echo "  the upload byte check passes on every cell (variant)" || { tail -5 "$T/up"; echo "  UPLOAD CHECK FAILED"; exit 1; }
CARGO_TARGET_DIR="$PWD/target-roe" cargo run --release -q -p campaign --features reset-on-entry --bin rpc_semantics 2>/dev/null > "$T/rs" || true
tail -1 "$T/rs" | grep -q "RPC SEMANTICS PASSED" && echo "  the RPC semantics cases pass (variant): $(grep -c '^PASS' "$T/rs") cases" || { grep "^FAIL" "$T/rs" | head; echo "  RPC SEMANTICS FAILED"; exit 1; }

echo
echo "ROE CHECKS PASSED"
