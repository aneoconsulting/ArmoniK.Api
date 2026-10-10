#!/usr/bin/env bash
# FIX-PLAN D27 (owner, 2026-10-10): reset on entry, the core's behaviour. Run by gen/gate.sh
# (step 11h); nothing timed. Stops at the first failure.
#
#   R1  bin roe_check, full build: ABANDON, ERR, ENC, HOSTERR, REARM (see the binary's header)
#   R2  the same, no-unknown build (ABANDON, ERR, ENC)
#   R3  plants in the generated core, in a SHADOW copy of poc/codec and poc/rust (as
#       gen/fsm_checks.sh, the same target-fsm-plant/ directories), each of which must make
#       roe_check FAIL: the decode entries' re-arm removed (push, pull, FSM), and the encode
#       entries' reset removed. The unplanted shadow build passes first.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
export AK_NO_TIMING=1
T="${TMPDIR:-/tmp}/ak-d27-checks.$$"
mkdir -p "$T"
trap 'rm -rf "$T"' EXIT

echo "  R1. roe_check, full build (drop, retain)"
cargo build --release -q -p campaign --bin roe_check 2>/dev/null
"$CARGO_TARGET_DIR/release/roe_check" > "$T/r1" 2>&1 || { grep -E "^FAIL|^#" "$T/r1" | head -20; echo "  D27 CHECKS FAILED (full build)"; exit 1; }
grep -E "^#|PASSED" "$T/r1" | sed 's/^/    /'

echo "  R2. roe_check, no-unknown build"
CARGO_TARGET_DIR="$PWD/target-nounk" cargo build --release -q -p campaign --no-default-features --features init-guard --bin roe_check 2>/dev/null
"$PWD/target-nounk/release/roe_check" > "$T/r2" 2>&1 || { grep -E "^FAIL|^#" "$T/r2" | head -20; echo "  D27 CHECKS FAILED (no-unknown build)"; exit 1; }
grep -E "^#|PASSED" "$T/r2" | sed 's/^/    /'

echo "  R3. planted defects in the generated core, in a shadow tree (each must FAIL roe_check)"
R=$(git rev-parse --show-toplevel)
SH="$PWD/target-fsm-plant/tree"
PT="$PWD/target-fsm-plant/build"
rm -rf "$SH/ffi/poc/codec" "$SH/ffi/poc/rust"
mkdir -p "$SH/ffi/poc/codec" "$SH/ffi/poc/rust"
tar -C "$R/ffi/poc/codec" --exclude='./target*' -cf - . | tar -C "$SH/ffi/poc/codec" -xf -
tar -C "$R/ffi/poc/rust" --exclude='./target*' --exclude='./corpus/target*' -cf - . | tar -C "$SH/ffi/poc/rust" -xf -
for d in ffi/schema ffi/corpus packages; do ln -sfn "$R/$d" "$SH/$d"; done
G="$SH/ffi/poc/codec/crates/ak-core/src/generated"
cp "$G/codec.rs" "$T/codec.rs.orig"; cp "$G/fsm.rs" "$T/fsm.rs.orig"
( cd "$SH/ffi/poc/rust" && CARGO_TARGET_DIR="$PT" cargo build --release -q -p campaign --bin roe_check 2>/dev/null )
"$PT/release/roe_check" > "$T/p" 2>&1 && echo "    unplanted shadow build: passes ($(grep -E '^# [0-9]+ checks' "$T/p"))" \
  || { tail -5 "$T/p"; echo "  the UNPLANTED shadow build fails roe_check"; exit 1; }
plant() {  # name, python expression over (c, f): the generated codec.rs and fsm.rs texts
  cp "$T/codec.rs.orig" "$G/codec.rs"; cp "$T/fsm.rs.orig" "$G/fsm.rs"
  python3 - "$G/codec.rs" "$G/fsm.rs" "$2" <<'PY'
import sys
pc, pf, expr = sys.argv[1], sys.argv[2], sys.argv[3]
c, f = open(pc).read(), open(pf).read()
c2, f2 = eval(expr)
assert (c2, f2) != (c, f), "the plant changed nothing"
open(pc, "w").write(c2); open(pf, "w").write(f2)
PY
  ( cd "$SH/ffi/poc/rust" && CARGO_TARGET_DIR="$PT" cargo build --release -q -p campaign --bin roe_check 2>/dev/null )
  if timeout 1200 "$PT/release/roe_check" > "$T/p" 2>&1; then
    echo "  PLANT NOT CAUGHT: $1"; exit 1
  fi
  echo "    planted $1: caught ($(grep -E '^# [0-9]+ checks' "$T/p"); first: $(grep -m1 '^FAIL' "$T/p" | cut -c1-150))"
}
plant "the decode entries' re-arm removed (push, pull, FSM)" \
  "(__import__('re').sub(r'    crate::rearm_on_entry\(dcx, &UNK_LAYOUT_\w+\);\n', '', c), __import__('re').sub(r'    crate::rearm_on_entry\(dcx, &crate::generated::codec::UNK_LAYOUT_\w+\);\n', '', f))"
plant "the encode entries' reset removed" "(c.replace('    crate::enc_reset_on_entry(cx);\n', ''), f)"
cp "$T/codec.rs.orig" "$G/codec.rs"; cp "$T/fsm.rs.orig" "$G/fsm.rs"
cmp -s "$R/ffi/poc/codec/crates/ak-core/src/generated/codec.rs" "$T/codec.rs.orig" && cmp -s "$R/ffi/poc/codec/crates/ak-core/src/generated/fsm.rs" "$T/fsm.rs.orig" \
  && echo "    this tree's generated core untouched (the plants were in the shadow copy only)" \
  || { echo "  this tree's generated core CHANGED during the plants"; exit 1; }
echo "  D27 CHECKS PASSED"
