#!/usr/bin/env bash
# FIX-PLAN D23 (owner, 2026-10-09): the FSM decode family's checks. NOT the full gate: the
# gate's own steps that prove push and pull unchanged, plus the FSM's differential, its
# planted defects and its C-header smoke. Nothing here is timed. Stops at the first failure.
#
#   0    configuration
#   1    generators current (every slice), one core
#   S    separation: the push/pull codec (codec.rs, four plans) byte-identical to BASE; every
#        Rust binding's pre-D23 text a prefix of the new one; every other generated file
#        changed by additions only
#   2    ak-core / ak-rt unit tests (debug build)
#   F1   the FSM differential on the codec suite's inputs (shapes, content sets, U-* rows),
#        full build (drop, retain), malformed variants under D20 masks 0 and all-ones
#   F2   the same, no-unknown build
#   F3   the same in the counting build: events, calls and crossings per payload
#   F4   the corpus differential, full and no-unknown builds
#   F5   planted defects in the generated FSM: each must FAIL the differential
#   F6   the C header: compiles as C11 and C++17 (full, no-unknown), and a C host decodes
#        through ak_fsm_begin/next against the core
#   F7   the codec suite's pre-check with the FSM arm (AK_FSM=1), both builds
#   3,4  byte identity and shape vectors (gate steps 3, 4)
#   5    crossing counts, push and pull (gate step 5), compared with BASE's
#   9,10 R-D1 and R-D6 (gate steps 9, 10)
#   11   the corpus, four arms and controls (gate step 11, gen/corpus.sh)
#   11b  the codec harness pre-check, default (no FSM arm): unchanged count
#   11c  crossing counts vs gen/crossings.txt
#   12   no-unknown build: pre-check, conformance, shapes, crossing counts
#
#   gen/d23_checks.sh BASE_COMMIT BASE_COUNTS_DIR
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
BASE=${1:?base commit}
BASEC=${2:?base counts dir}
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
export AK_NO_TIMING=1
step() { echo; echo "===== $* ====="; }
exe_of() {
  python3 -S -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("target",{}).get("name")=="codec_suite" and m.get("executable"): print(m["executable"])' | tail -1
}

step "0. configuration"
rustc --version
echo "commit $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/rust or poc/codec'); base $BASE"
nproc; grep -m1 'model name' /proc/cpuinfo || true; uptime

step "1. generators current (every slice), one core"
# D23_SKIP_SLICES (space-separated): slices whose generator is not checked here, named in the
# log. Used when another agent is editing that slice's generator in the same tree.
if [ -z "${D23_SKIP_SLICES:-}" ]; then
  python3 ../codec/gen/generate.py --check 2>/dev/null | grep -vE "^ok " | sed 's/^/  /'
  python3 ../codec/gen/generate.py --check >/dev/null 2>&1 || { echo "  STALE generated files"; exit 1; }
else
  echo "  NOT CHECKED here (D23_SKIP_SLICES): $D23_SKIP_SLICES"
  python3 ../codec/gen/generate.py --check --core-only 2>/dev/null | grep -vE "^ok " | sed 's/^/  /'
  python3 ../codec/gen/generate.py --check --core-only >/dev/null 2>&1 || { echo "  STALE core files"; exit 1; }
  for sl in rust cpp java csharp python; do
    case " $D23_SKIP_SLICES " in *" $sl "*) continue ;; esac
    if AK_GEN_CORE_ONLY=1 python3 ../$sl/gen/generate.py --check >/dev/null 2>&1; then echo "  slice $sl --check: current"; else echo "  slice $sl --check: STALE"; exit 1; fi
  done
fi
../codec/gen/one_core.sh | tail -2

step "S. separation: push and pull's generated code unchanged, everything else additive"
R=$(git rev-parse --show-toplevel)
cd "$R"
for f in generated generated_nounk generated_corpus generated_corpus_nounk; do
  p=ffi/poc/codec/crates/ak-core/src/$f/codec.rs
  if git diff --quiet "$BASE" -- "$p"; then echo "  $p: identical to $BASE ($(sha256sum "$p" | cut -c1-16))"; else echo "  $p DIFFERS"; exit 1; fi
done
python3 - "$BASE" <<'PY'
import subprocess, sys
base = sys.argv[1]
bad = 0
for p in ["ffi/poc/rust/crates/harness/src/generated/binding.rs",
          "ffi/poc/rust/crates/harness/src/generated/binding_nounk.rs",
          "ffi/poc/rust/corpus/crates/harness/src/generated/binding.rs",
          "ffi/poc/rust/corpus/crates/harness/src/generated/binding_nounk.rs"]:
    old = subprocess.run(["git", "show", "%s:%s" % (base, p)], capture_output=True, text=True, check=True).stdout
    new = open(p).read()
    ok = new.startswith(old)
    print("  %s: pre-D23 text (%d lines) is %s of the new (%d lines)" % (p, old.count("\n"), "a PREFIX" if ok else "NOT a prefix", new.count("\n")))
    bad += not ok
sys.exit(1 if bad else 0)
PY
echo "  generated or hand-written files with deleted or changed lines since $BASE (py sources, docs, logs excluded):"
git diff --numstat "$BASE" -- ffi/poc ':!*.py' ':!*.md' | awk '$2 > 0 { print "    " $0 }'
echo "  generated files changed by additions only: $(git diff --numstat "$BASE" -- ffi/poc ':!*.py' ':!*.md' | awk '$2 == 0' | wc -l)"
cd "$HERE/.."

step "2. core unit tests (debug build)"
( cd ../codec && CARGO_TARGET_DIR="$HERE/../target-codec-test" cargo test -q -p ak-core -p ak-rt 2>&1 \
    | grep -E "test result|FAILED|panicked" )

step "F1. FSM differential, shapes core, full build (drop, retain)"
cargo build --release -q -p campaign --bin fsm_diff 2>/dev/null
"$CARGO_TARGET_DIR/release/fsm_diff" > /tmp/d23-f1.$$ 2>&1 || { tail -40 /tmp/d23-f1.$$; exit 1; }
grep -E "^#|PASSED|FAIL" /tmp/d23-f1.$$ | head -20; rm -f /tmp/d23-f1.$$

step "F2. FSM differential, shapes core, no-unknown build"
CARGO_TARGET_DIR="$PWD/target-nounk" cargo build --release -q -p campaign --no-default-features --features init-guard --bin fsm_diff 2>/dev/null
"$PWD/target-nounk/release/fsm_diff" > /tmp/d23-f2.$$ 2>&1 || { tail -40 /tmp/d23-f2.$$; exit 1; }
grep -E "^#|PASSED|FAIL" /tmp/d23-f2.$$ | head -20; rm -f /tmp/d23-f2.$$

step "F3. FSM differential in the counting build: events, calls, crossings per payload (no malformed variants)"
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p campaign --features count --bin fsm_diff -- --no-malformed 2>/dev/null
CARGO_TARGET_DIR="$PWD/target-count-nounk" cargo run --release -q -p campaign --no-default-features --features count,init-guard --bin fsm_diff -- --no-malformed 2>/dev/null

step "F4. FSM differential, corpus core (every row the ABI carries), full and no-unknown builds"
( cd corpus && CARGO_TARGET_DIR="$HERE/../target-corpus" cargo build --release -q 2>/dev/null )
timeout 600 "$HERE/../target-corpus/release/corpus" --fsm-diff
( cd corpus && CARGO_TARGET_DIR="$HERE/../target-corpus-nounk" cargo build --release -q --no-default-features --features init-guard 2>/dev/null )
timeout 600 "$HERE/../target-corpus-nounk/release/corpus" --fsm-diff

step "F5. planted defects in the generated FSM (each must FAIL the differential)"
# D23_PLANT_POC: an ffi/poc of a separate worktree to plant in (so another agent building the
# core in this tree never picks up a planted core); default this tree.
PP=$(cd "${D23_PLANT_POC:-$HERE/../..}" && pwd)
echo "  planting in $PP"
FSM=$PP/codec/crates/ak-core/src/generated/fsm.rs
SAVE=$(mktemp)
cp "$FSM" "$SAVE"
restore() { cp "$SAVE" "$FSM"; rm -f "$SAVE"; }
trap restore EXIT
PT="$PWD/target-d23-plant"
plant() {  # name, python expression over s (the generated FSM's text)
  cp "$SAVE" "$FSM"
  python3 - "$FSM" "$2" <<'PY'
import sys
f, expr = sys.argv[1], sys.argv[2]
s = open(f).read()
t = eval(expr)
assert t != s, "the plant changed nothing"
open(f, "w").write(t)
PY
  ( cd "$PP/rust" && CARGO_TARGET_DIR="$PT" cargo build --release -q -p campaign --bin fsm_diff 2>/dev/null )
  if timeout 600 "$PT/release/fsm_diff" > /tmp/d23-plant.$$ 2>&1; then
    echo "  PLANT NOT CAUGHT: $1"; tail -3 /tmp/d23-plant.$$; rm -f /tmp/d23-plant.$$; exit 1
  fi
  echo "  planted $1: caught ($(grep -E '^# [0-9]+ checks' /tmp/d23-plant.$$); first: $(grep -m1 -E 'FAIL' /tmp/d23-plant.$$ | cut -c1-170))"
  rm -f /tmp/d23-plant.$$
}
plant "a minted token off by one" "s.replace('let tok = f.mint();', 'let tok = f.mint() + 1;', 1)"
plant "a lost run (the root's last run never flushed: ListResultsResponse)" "s.replace('if f.n > 0 { ev_run_root!(); return FSM_ADD; }', '', 1)"
plant "a run split one element early (ListResultsResponse.results arena - 1)" "s.replace('const FSM_N_LISTRESULTSRESPONSE_ROOT_RESULTS: usize = fsm_arena_n(::core::mem::size_of::<ak_dfix_ResultRaw>());', 'const FSM_N_LISTRESULTSRESPONSE_ROOT_RESULTS: usize = fsm_arena_n(::core::mem::size_of::<ak_dfix_ResultRaw>()) - 1;', 1)"
plant "the rewind after a flush lands one byte late (element scope)" "s.replace('if f.n > 0 { ev_run_e0!(); f.cur = 0; f.pos = \$s0; return FSM_ADD; }', 'if f.n > 0 { ev_run_e0!(); f.cur = 0; f.pos = \$s0 + 1; return FSM_ADD; }')"
plant "the owed error of a truncated element dropped" "s.replace('if f.pend != 0 && f.depth <= f.pend_depth { fail!(f.pend); }', '')"
plant "the FSM ignores its D20 mask" "s.replace('f.sk = f.utf8_skip;', 'f.sk = 0;')"
restore; trap - EXIT
python3 "$PP/codec/gen/generate.py" --check --core-only 2>/dev/null | grep -c "^ok" | sed 's/^/  generated core restored, files ok: /'
rm -rf "$PT"

step "F6. the C header: C11 / C++17, full and no-unknown; a C host through ak_fsm_* against the core"
for h in ../cpp/include ../cpp/nounk/include; do
  echo '#include "ak_abi.h"' | gcc -std=c11 -Wall -Werror -fsyntax-only -I"$h" -x c - && echo "  $h: C11 ok"
  echo '#include "ak_abi.h"' | g++ -std=c++17 -Wall -Werror -fsyntax-only -I"$h" -x c++ - && echo "  $h: C++17 ok"
  echo "  $h: $(grep -c 'ak_fsm_\(begin\|next\|set_pvt\)_' "$h/ak_abi.h") FSM declarations"
done
SO=$(ldd "$CARGO_TARGET_DIR/release/fsm_diff" | grep -o "/[^ ]*libak_core.so")
CT=$(mktemp -d)
cat > "$CT/fsm.c" <<'C'
#include <stdio.h>
#include <string.h>
#include "ak_abi.h"
/* A ListResultsResponse with page = 3 and two results, each with a result_id string. */
int main(void) {
  struct ak_err e; struct ak_init_opts o; memset(&o, 0, sizeof o); o.abi_version = AK_ABI_VERSION;
  if (ak_init(&o, &e) < 0) { puts("ak_init failed"); return 1; }
  ak_dec_ctx *c = ak_dec_ctx_new_ListResultsResponse(NULL);
  const uint8_t m[] = { 0x0a, 0x03, 0x0a, 0x01, 'a', 0x0a, 0x03, 0x0a, 0x01, 'b', 0x10, 0x03 };
  /* The return value is the event's op (AK_BDR_*), < 0 an error; AK_BDR_APPLY is the last. */
  struct ak_fsm_ev ev; int32_t rc = ak_fsm_begin_ListResultsResponse(c, m, sizeof m, &ev); int n = 1;
  printf("  C host: event %d op %d slot %u token %lld n %u bytes %u\n", n, rc, ev.slot, (long long)ev.token, ev.n, ev.bytes);
  int ok = rc == (int32_t)AK_BDR_ADD && ev.slot == 1 && ev.n == 2;
  while (rc > 0 && rc != (int32_t)AK_BDR_APPLY) {
    rc = ak_fsm_next_ListResultsResponse(c, &ev); n++;
    printf("  C host: event %d op %d slot %u token %lld n %u bytes %u\n", n, rc, ev.slot, (long long)ev.token, ev.n, ev.bytes);
  }
  ok = ok && rc == (int32_t)AK_BDR_APPLY && n == 2 && ev.slot == 0 && ((const struct ak_dfix_ListResultsResponse *)ev.data)->page == 3;
  ok = ok && ak_fsm_next_ListResultsResponse(c, &ev) == AK_ERR_INVALID_STATE;
  ak_dec_ctx_free(c);
  puts(ok ? "  C HOST OK" : "  C HOST FAILED");
  return ok ? 0 : 1;
}
C
gcc -std=c11 -Wall -Werror -I../cpp/include "$CT/fsm.c" -o "$CT/fsm" "$SO" -Wl,-rpath,"$(dirname "$SO")" && "$CT/fsm"
rm -rf "$CT"

step "F7. the codec suite's pre-check WITH the FSM arm (AK_FSM=1), full and no-unknown builds"
( B=$(cargo bench -q -p campaign --bench codec_suite --no-run --message-format=json 2>/dev/null | exe_of)
  CRITERION_HOME="$(mktemp -d)" AK_FSM=1 AK_CAMPAIGN_GRID=full AK_PRECHECK_ONLY=1 "$B" 2>&1 | grep -E "^# precheck|PRECHECK" )
( export CARGO_TARGET_DIR="$PWD/target-nounk"
  B=$(cargo bench -q -p campaign --no-default-features --features init-guard --bench codec_suite --no-run --message-format=json 2>/dev/null | exe_of)
  CRITERION_HOME="$(mktemp -d)" AK_FSM=1 AK_CAMPAIGN_GRID=full AK_PRECHECK_ONLY=1 "$B" 2>&1 | grep -E "^# precheck|PRECHECK" )

step "3. byte identity against the validated manifest"
cargo run --release -q -p harness --bin conformance 2>/dev/null | tail -25

step "4. presence, oneof, unknown-field vectors"
cargo run --release -q -p harness --bin shapes 2>/dev/null | tail -25

step "5. crossing counts (counting build), push and pull, compared with BASE"
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p harness --features count --bin counts 2>/dev/null > /tmp/d23-counts.$$
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p harness --features count --bin pullbench 2>/dev/null > /tmp/d23-pull.$$
cat /tmp/d23-counts.$$ /tmp/d23-pull.$$
for x in counts:d23-counts pullbench:d23-pull; do
  n=${x%%:*}; f=/tmp/${x##*:}.$$
  if diff -q <(grep -v '^# commit\|^# built\|ns\b' "$BASEC/$n.txt") <(grep -v '^# commit\|^# built\|ns\b' "$f") >/dev/null; then
    echo "  $n: identical to BASE's ($(wc -l < "$f") lines)"
  else
    echo "  $n DIFFERS from BASE's:"; diff <(cat "$BASEC/$n.txt") "$f" | head -20; exit 1
  fi
done
rm -f /tmp/d23-counts.$$ /tmp/d23-pull.$$

step "9. R-D1 length-wrap reproductions"
gen/rd1_repro.sh gate

step "10. R-D6 sticky error slot"
cargo run --release -q -p harness --bin stickyerr 2>/dev/null
cargo run --release -q -p harness --bin fresh_enc 2>/dev/null

step "11. the conformance corpus, C ABI and core-native, drop and retain"
gen/corpus.sh

step "11b. the codec harness pre-check, default (no FSM arm)"
( B=$(cargo bench -q -p campaign --bench codec_suite --no-run --message-format=json 2>/dev/null | exe_of)
  CRITERION_HOME="$(mktemp -d)" AK_PRECHECK_ONLY=1 "$B" 2>&1 | grep -E "^# precheck|PRECHECK" )

step "11c. crossing counts of every timed core-ffi case vs gen/crossings.txt (and BASE's)"
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p campaign --features count --bin crossings 2>/dev/null > /tmp/d23-cr.$$
diff -q gen/crossings.txt /tmp/d23-cr.$$ >/dev/null && diff -q "$BASEC/crossings.txt" /tmp/d23-cr.$$ >/dev/null \
  && echo "  $(wc -l < gen/crossings.txt) lines identical to gen/crossings.txt and to BASE's" \
  || { echo "  crossing counts DIFFER"; exit 1; }
rm -f /tmp/d23-cr.$$

step "12. the NO-UNKNOWN variant: pre-check, byte identity, shape vectors, crossing counts"
( export CARGO_TARGET_DIR="$PWD/target-nounk"
  B=$(cargo bench -q -p campaign --no-default-features --features init-guard --bench codec_suite --no-run --message-format=json 2>/dev/null | exe_of)
  CRITERION_HOME="$(mktemp -d)" AK_PRECHECK_ONLY=1 "$B" 2>&1 | grep -E "^# precheck|PRECHECK" )
for bin in conformance shapes; do
  CARGO_TARGET_DIR="$PWD/target-nounk" cargo run --release -q -p harness --no-default-features --features guard,init-guard --bin $bin 2>/dev/null \
    | grep -E "^VERDICT" | sed "s/^/    $bin: /"
done
CARGO_TARGET_DIR="$PWD/target-count-nounk" cargo run --release -q -p campaign --no-default-features --features count,init-guard --bin crossings 2>/dev/null > /tmp/d23-crn.$$
diff -q gen/crossings-nounk.txt /tmp/d23-crn.$$ >/dev/null && diff -q "$BASEC/crossings-nounk.txt" /tmp/d23-crn.$$ >/dev/null \
  && echo "  no-unknown crossing counts: $(wc -l < gen/crossings-nounk.txt) lines identical to gen/crossings-nounk.txt and to BASE's" \
  || { echo "  no-unknown crossing counts DIFFER"; exit 1; }
rm -f /tmp/d23-crn.$$

echo
echo "D23 CHECKS PASSED"
