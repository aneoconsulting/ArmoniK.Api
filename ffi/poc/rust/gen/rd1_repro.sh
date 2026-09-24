#!/usr/bin/env bash
# FIX-PLAN WP4 item 1 / R-D1: run each reproduction case in its own process under a
# timeout, so a hang (claim a, u32) or an abort (claim c) cannot hide the others.
#
# Usage: gen/rd1_repro.sh <label>    (label goes in the header, e.g. "unfixed" / "fixed")
# Builds with the rust slice's own CARGO_TARGET_DIR, never poc/codec/target.
set -u
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target-r-d1}"
LABEL="${1:-run}"

cargo build --release -p harness --bin rdrepro >/dev/null 2>&1 || {
    echo "BUILD FAILED"; exit 1; }
BIN="$CARGO_TARGET_DIR/release/rdrepro"

echo "# R-D1 length-varint-wrap reproduction -- $LABEL"
echo "#   rustc $(rustc --version | awk '{print $2}'), release (overflow-checks off), cdylib boundary"
echo "#   each case: timeout 5 ./rdrepro <case>"
echo "#   exit 124 = hang, 134 = SIGABRT (panic across extern \"C\"), 101 = rust panic, 139 = SIGSEGV"
echo "#   input for (a) is the finding's literal 11 bytes: 7A F5 FF FF FF FF FF FF FF FF 01"
echo

decode_exit() {
    case "$1" in
        124) echo "TIMEOUT/hang";; 134) echo "SIGABRT (extern-C abort)";;
        139) echo "SIGSEGV";; 101) echo "rust panic";; 0) echo "returned";; *) echo "exit $1";;
    esac
}

bad=0
for c in a-ffi a-native b-err-ffi b-err-native b-span-ffi b-span-native c-ffi c-native u32-ffi; do
    echo "## case $c"
    out=$(timeout 5 "$BIN" "$c" 2>&1); rc=$?
    # keep only the first two lines of any panic backtrace, they carry the message
    echo "$out" | grep -E '^\[|panicked at|slice index|cannot unwind|non-unwinding' | head -4 | sed 's/^/    /'
    echo "    -> exit=$rc ($(decode_exit $rc))"
    [ "$rc" -eq 0 ] || bad=$((bad+1))
    echo
done
echo "# cases that did not return promptly: $bad"
exit $([ $bad -eq 0 ] && echo 0 || echo 1)
