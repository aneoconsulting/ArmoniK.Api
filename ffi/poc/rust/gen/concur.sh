#!/usr/bin/env bash
# ABI v1 conformance obligation 12.5: the concurrency suite, end to end.
#
# FOUR builds of one source tree. ABI v1 section 6 carries two refusals and they are
# INDEPENDENT -- the cpp slice's suite separated them (ffi/logs/cpp/concurrency.log) and
# section 6 was rewritten on that evidence. This suite is built to the rewritten text:
#
#   shipped                        per-context table, prefix moved on a miss   must PASS
#   --features global-widths       one table for the whole process             must PASS
#                                  a data race and a THROUGHPUT defect, and
#                                  not a byte defect, because an unpadded
#                                  prefix is rewritten to the width the body
#                                  actually needs whatever the guess was
#   --features pad-widths          prefix PADDED to the learned width          must FAIL
#                                  the BYTE defect: the encoder's output now
#                                  depends on its own history
#   --features global,pad          the combination                             must FAIL
#                                  the worst case, and the one a naive suite
#                                  cannot see: the threads AGREE because they
#                                  share the pollution
#
# The oracle is prost, the incumbent's encoder, and not a re-encode with the code under
# test. Section 1b measures why: on the combination a "fresh" context reads the same
# polluted global table and agrees with the threads, so the old oracle is blind to it.
#
# Separate CARGO_TARGET_DIRs so no build is another's stale artifact, and each binary's
# loaded .so is confirmed from ldd rather than from the build log.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

echo "===== 0. configuration ====="
rustc --version
nproc
grep -m1 'model name' /proc/cpuinfo || true

bad=0
build() {  # name, target dir, features...
  local name=$1 dir=$2; shift 2
  if [ $# -gt 0 ]; then
    CARGO_TARGET_DIR="$dir" cargo build --release -q -p harness --features "$1" --bin concur
  else
    CARGO_TARGET_DIR="$dir" cargo build --release -q -p harness --bin concur
  fi
  echo "# $name loaded: $(ldd "$dir/release/concur" | grep -o '[^ ]*libak_core.so')"
}

echo
echo "===== 1. build all four arms, and confirm each one's .so from ldd ====="
build shipped     target
build global      target-gw            global-widths
build pad         target-pad           pad-widths
build both        target-gw-pad        global-widths,pad-widths

echo
echo "===== 1b. the shared-site pair really does flip a width (counting build) ====="
echo "# A contention arm that never contends measures nothing, so the flipping is counted."
CARGO_TARGET_DIR=target-count cargo run --release -q -p harness --features count --bin concur 2>/dev/null \
  | sed -n '/does the shared-site/,/^# visible to/p' | awk 'NR<=14'

# Each build knows what IT is required to do -- the binary reads its own feature flags and
# exits 0 when it behaved as required, which for a planted build means "the suite caught
# me". So the wrapper asks one question of every arm: did it exit 0. Inverting it here as
# well was the first version of this script, and it turned two correct catches into two
# reported failures.
run() {  # label, dir
  local label=$1 dir=$2 rc=0
  echo
  echo "----- $label -----"
  "$dir/release/concur" || rc=$?
  if [ "$rc" = 0 ]; then echo ">>> ok: $label behaved as its build requires"
  else echo ">>> FAIL: $label did not"; bad=$((bad+1)); fi
}

echo
echo "===== 2. the shipped build, three runs (must PASS) ====="
for i in 1 2 3; do run "shipped run $i" target; done

echo
echo "===== 3. the process-global table, three runs (a REFUSED design that is byte-CLEAN: must PASS) ====="
for i in 1 2 3; do run "global run $i" target-gw; done

echo
echo "===== 4. the padded prefix (a REFUSED design: must FAIL) ====="
run "pad" target-pad

echo
echo "===== 5. global AND padded, the combination (a REFUSED design: must FAIL) ====="
run "both" target-gw-pad

echo
echo "===== verdict ====="
if [ "$bad" = 0 ]; then echo "all four builds behaved as required"; else echo "$bad build(s) did not"; fi
exit $bad
