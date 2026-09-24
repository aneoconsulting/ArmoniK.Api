#!/usr/bin/env bash
# FIX-PLAN WP4 item 9 / R-D8: the concurrency suite (obligation 12.5) under ThreadSanitizer.
#
# The suite asserts OUTPUTS, so a race that does not change bytes at this thread count
# passes it. TSan is the race DETECTOR the suite is not. Both the harness and the shared
# core's cdylib are instrumented (RUSTFLAGS applies to every crate, and -Zbuild-std
# instruments std, without which TSan reports false positives in std's own atomics).
#
# Nightly only (-Zsanitizer is unstable). Own CARGO_TARGET_DIR. Sections 1-3 of `concur`
# only: section 4 is a planted shared-context data race (its own process, `--control`,
# not run here) and section 5 is a timing, meaningless under TSan.
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR="$PWD/target-tsan"
export RUSTFLAGS="-Zsanitizer=thread"
TRIPLE=x86_64-unknown-linux-gnu
B() { cargo +nightly build --release -q -Zbuild-std --target $TRIPLE -p harness --bin concur; }
echo "# rustc $(rustc +nightly --version)"
# First pass builds ak-core (and fails to link concur on this nightly's layout); the second
# links against the cdylib the first produced.
B 2>/dev/null || true
SO=$(find "$CARGO_TARGET_DIR/$TRIPLE/release" -name libak_core.so -newer Cargo.toml | head -1)
export AK_CORE_LIB_DIR="$(dirname "$SO")"
B
BIN=$(find "$CARGO_TARGET_DIR/$TRIPLE/release" -type f -name concur -perm -u+x | head -1)
echo "# binary  $BIN"
echo "# loads   $(ldd "$BIN" | grep -o 'libak_core.so => [^ ]*')"
echo "# core instrumented: $(nm -D "$SO" | grep -c __tsan_ || true) __tsan_ references in libak_core.so"
echo
echo "===== the suite (sections 1-3) under TSan: must report NO race ====="
TSAN_OPTIONS="halt_on_error=0 report_signal_unsafe=0" AK_CONCUR_NO_TIMING=1 "$BIN" > /tmp/tsan-suite.$$ 2>&1 && rc=0 || rc=$?
cat /tmp/tsan-suite.$$
echo "# suite exit $rc; TSan warnings in the suite: $(grep -c 'WARNING: ThreadSanitizer' /tmp/tsan-suite.$$ || true)"
rm -f /tmp/tsan-suite.$$
echo
echo "===== positive control: the planted shared-context race (concur --control) ====="
echo "# Four threads on ONE encode context, a contract violation (ABI v1 section 3). TSan"
echo "# must SEE it, or the run above says nothing. The process then aborts, as in stage 5."
TSAN_OPTIONS="halt_on_error=0 report_signal_unsafe=0" "$BIN" --control > /tmp/tsan-control.$$ 2>&1 || true
echo "# TSan warnings in the control: $(grep -c 'WARNING: ThreadSanitizer' /tmp/tsan-control.$$ || true)"
grep -m3 -A4 "WARNING: ThreadSanitizer" /tmp/tsan-control.$$ | sed 's/^/    /' || true
rm -f /tmp/tsan-control.$$
