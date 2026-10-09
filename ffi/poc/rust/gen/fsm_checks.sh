#!/usr/bin/env bash
# FIX-PLAN D23's checks of the FSM decode family, which is the target's decode since D24
# (core-ffi's decode, the core-codec RPC cells' response decode). Run by gen/gate.sh (step
# 11g); nothing here is timed. Stops at the first failure.
#
#   F1  the differential on the codec suite's inputs (shapes, content sets, U-* rows), full
#       build (drop, retain), malformed variants under D20 masks 0 and all-ones: the FSM's
#       events equal the pull family's records, the Rust consumer's graph equals push's,
#       the API refusals (crates/campaign/src/bin/fsm_diff.rs)
#   F2  the same, no-unknown build
#   F3  the same in the counting build: events, calls and crossings per payload
#   F4  the corpus differential (every row the ABI carries), full and no-unknown builds
#   F5  planted defects in the generated FSM: each must FAIL the differential. Planted in a
#       SHADOW copy of poc/codec and poc/rust (target-fsm-plant/tree; schema, corpus and
#       packages linked), so nothing that builds the core in this tree meanwhile (another
#       slice's agent) can pick up a planted core, and this tree's files are never edited
#   F6  the C header: compiles as C11 and C++17 (full, no-unknown), and a C host decodes
#       through ak_fsm_begin/next against the core
#
# Needs: target/ and target-nounk/ (fsm_diff is built here), target-corpus/ and
# target-corpus-nounk/ (gen/corpus.sh builds them; built here when missing).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
export AK_NO_TIMING=1
T="${TMPDIR:-/tmp}/ak-fsm-checks.$$"
mkdir -p "$T"
trap 'rm -rf "$T"' EXIT

echo "  F1. differential, shapes core, full build (drop, retain)"
cargo build --release -q -p campaign --bin fsm_diff 2>/dev/null
"$CARGO_TARGET_DIR/release/fsm_diff" > "$T/f1" 2>&1 || { tail -40 "$T/f1"; echo "  FSM DIFFERENTIAL FAILED (full build)"; exit 1; }
grep -E "^#|PASSED|FAIL" "$T/f1" | head -20 | sed 's/^/    /'

echo "  F2. differential, shapes core, no-unknown build"
CARGO_TARGET_DIR="$PWD/target-nounk" cargo build --release -q -p campaign --no-default-features --features init-guard --bin fsm_diff 2>/dev/null
"$PWD/target-nounk/release/fsm_diff" > "$T/f2" 2>&1 || { tail -40 "$T/f2"; echo "  FSM DIFFERENTIAL FAILED (no-unknown build)"; exit 1; }
grep -E "^#|PASSED|FAIL" "$T/f2" | head -20 | sed 's/^/    /'

echo "  F3. differential in the counting builds: events, calls, crossings per payload (no malformed variants)"
CARGO_TARGET_DIR="$PWD/target-count" cargo run --release -q -p campaign --features count --bin fsm_diff -- --no-malformed 2>/dev/null | sed 's/^/    /'
CARGO_TARGET_DIR="$PWD/target-count-nounk" cargo run --release -q -p campaign --no-default-features --features count,init-guard --bin fsm_diff -- --no-malformed 2>/dev/null | sed 's/^/    /'

echo "  F4. corpus differential (every row the ABI carries), full and no-unknown builds"
( cd corpus && CARGO_TARGET_DIR="$HERE/../target-corpus" cargo build --release -q 2>/dev/null )
timeout 600 "$HERE/../target-corpus/release/corpus" --fsm-diff | sed 's/^/    /'
( cd corpus && CARGO_TARGET_DIR="$HERE/../target-corpus-nounk" cargo build --release -q --no-default-features --features init-guard 2>/dev/null )
timeout 600 "$HERE/../target-corpus-nounk/release/corpus" --fsm-diff | sed 's/^/    /'

echo "  F5. planted defects in the generated FSM, in a shadow tree (each must FAIL the differential)"
R=$(git rev-parse --show-toplevel)
SH="$PWD/target-fsm-plant/tree"
PT="$PWD/target-fsm-plant/build"
mkdir -p "$SH/ffi/poc/codec" "$SH/ffi/poc/rust"
# A fresh copy each run (the build dir is kept: only ak-core and what depends on it rebuild).
rm -rf "$SH/ffi/poc/codec" "$SH/ffi/poc/rust"
mkdir -p "$SH/ffi/poc/codec" "$SH/ffi/poc/rust"
tar -C "$R/ffi/poc/codec" --exclude='./target*' -cf - . | tar -C "$SH/ffi/poc/codec" -xf -
tar -C "$R/ffi/poc/rust" --exclude='./target*' --exclude='./corpus/target*' -cf - . | tar -C "$SH/ffi/poc/rust" -xf -
for d in ffi/schema ffi/corpus packages; do ln -sfn "$R/$d" "$SH/$d"; done
FSM="$SH/ffi/poc/codec/crates/ak-core/src/generated/fsm.rs"
cmp -s "$FSM" "$R/ffi/poc/codec/crates/ak-core/src/generated/fsm.rs" || { echo "  the shadow copy differs from the tree"; exit 1; }
cp "$FSM" "$T/fsm.rs.orig"
echo "    shadow tree $SH (codec and rust copied from this tree, the generated FSM identical)"
# The unplanted shadow build must PASS first, or a caught plant proves nothing.
( cd "$SH/ffi/poc/rust" && CARGO_TARGET_DIR="$PT" cargo build --release -q -p campaign --bin fsm_diff 2>/dev/null )
if timeout 600 "$PT/release/fsm_diff" > "$T/plant" 2>&1; then
  echo "    unplanted shadow build: passes ($(grep -E '^# [0-9]+ checks' "$T/plant" || true))"
else
  tail -5 "$T/plant"; echo "  the UNPLANTED shadow build fails the differential"; exit 1
fi
plant() {  # name, python expression over s (the generated FSM's text)
  cp "$T/fsm.rs.orig" "$FSM"
  python3 - "$FSM" "$2" <<'PY'
import sys
f, expr = sys.argv[1], sys.argv[2]
s = open(f).read()
t = eval(expr)
assert t != s, "the plant changed nothing"
open(f, "w").write(t)
PY
  ( cd "$SH/ffi/poc/rust" && CARGO_TARGET_DIR="$PT" cargo build --release -q -p campaign --bin fsm_diff 2>/dev/null )
  if timeout 600 "$PT/release/fsm_diff" > "$T/plant" 2>&1; then
    echo "  PLANT NOT CAUGHT: $1"; tail -3 "$T/plant"; exit 1
  fi
  echo "    planted $1: caught ($(grep -E '^# [0-9]+ checks' "$T/plant" || true); first: $(grep -m1 -E 'FAIL' "$T/plant" | cut -c1-150))"
}
plant "a minted token off by one" "s.replace('let tok = f.mint();', 'let tok = f.mint() + 1;', 1)"
plant "a lost run (the root's last run never flushed: ListResultsResponse)" "s.replace('if f.n > 0 { ev_run_root!(); return FSM_ADD; }', '', 1)"
plant "a run split one element early (ListResultsResponse.results arena - 1)" "s.replace('const FSM_N_LISTRESULTSRESPONSE_ROOT_RESULTS: usize = fsm_arena_n(::core::mem::size_of::<ak_dfix_ResultRaw>());', 'const FSM_N_LISTRESULTSRESPONSE_ROOT_RESULTS: usize = fsm_arena_n(::core::mem::size_of::<ak_dfix_ResultRaw>()) - 1;', 1)"
plant "the rewind after a flush lands one byte late (element scope)" "s.replace('if f.n > 0 { ev_run_e0!(); f.cur = 0; f.pos = \$s0; return FSM_ADD; }', 'if f.n > 0 { ev_run_e0!(); f.cur = 0; f.pos = \$s0 + 1; return FSM_ADD; }')"
plant "the owed error of a truncated element dropped" "s.replace('if f.pend != 0 && f.depth <= f.pend_depth { fail!(f.pend); }', '')"
plant "the FSM ignores its D20 mask" "s.replace('f.sk = f.utf8_skip;', 'f.sk = 0;')"
cp "$T/fsm.rs.orig" "$FSM"
cmp -s "$R/ffi/poc/codec/crates/ak-core/src/generated/fsm.rs" "$T/fsm.rs.orig" \
  && echo "    this tree's generated FSM untouched (the plants were in the shadow copy only)" \
  || { echo "  this tree's generated FSM CHANGED during the plants"; exit 1; }

echo "  F6. the C header: C11 / C++17, full and no-unknown; a C host through ak_fsm_* against the core"
for h in ../cpp/include ../cpp/nounk/include; do
  echo '#include "ak_abi.h"' | gcc -std=c11 -Wall -Werror -fsyntax-only -I"$h" -x c - && echo "    $h: C11 ok"
  echo '#include "ak_abi.h"' | g++ -std=c++17 -Wall -Werror -fsyntax-only -I"$h" -x c++ - && echo "    $h: C++17 ok"
  echo "    $h: $(grep -c 'ak_fsm_\(begin\|next\|set_pvt\)_' "$h/ak_abi.h") FSM declarations"
done
SO=$(ldd "$CARGO_TARGET_DIR/release/fsm_diff" | grep -o "/[^ ]*libak_core.so")
cat > "$T/fsm.c" <<'C'
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
  printf("    C host: event %d op %d slot %u token %lld n %u bytes %u\n", n, rc, ev.slot, (long long)ev.token, ev.n, ev.bytes);
  int ok = rc == (int32_t)AK_BDR_ADD && ev.slot == 1 && ev.n == 2;
  while (rc > 0 && rc != (int32_t)AK_BDR_APPLY) {
    rc = ak_fsm_next_ListResultsResponse(c, &ev); n++;
    printf("    C host: event %d op %d slot %u token %lld n %u bytes %u\n", n, rc, ev.slot, (long long)ev.token, ev.n, ev.bytes);
  }
  ok = ok && rc == (int32_t)AK_BDR_APPLY && n == 2 && ev.slot == 0 && ((const struct ak_dfix_ListResultsResponse *)ev.data)->page == 3;
  ok = ok && ak_fsm_next_ListResultsResponse(c, &ev) == AK_ERR_INVALID_STATE;
  ak_dec_ctx_free(c);
  puts(ok ? "    C HOST OK" : "    C HOST FAILED");
  return ok ? 0 : 1;
}
C
gcc -std=c11 -Wall -Werror -I../cpp/include "$T/fsm.c" -o "$T/fsm" "$SO" -Wl,-rpath,"$(dirname "$SO")" && "$T/fsm"

echo "  FSM CHECKS PASSED"
