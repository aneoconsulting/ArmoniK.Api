#!/usr/bin/env bash
# ABI v1 obligation 12.5, run: the suite on the shipped design, and the three planted
# builds that show the suite can fail.
#
# The plants are inverted here rather than in the binary. A fixture that exits 0 when it
# finds the defect it was built to find is a fixture nobody can run in a loop; a fixture
# that exits 0 when it does NOT find it is a fixture that silently stops working, which is
# how this slice's -flto control (C18) stopped working. So: shipped builds must pass, and
# each planted build must FAIL, and a planted build that passes fails this script.
set -u
cd "$(dirname "$0")/.."
B=./build
bad=0

run_pass() {   # a build that must pass
  echo "=========== $1   (must PASS) ==========="
  "$B/$1" "${2:-4}" "${3:-6}"
  local rc=$?
  if [ $rc -ne 0 ]; then echo "  >>> FAIL: $1 exited $rc"; bad=$((bad + 1));
  else echo "  >>> ok: $1 passed"; fi
  echo
}

run_plant() {  # a planted build that must fail
  echo "=========== $1   (a REFUSED design: must FAIL) ==========="
  "$B/$1" "${2:-4}" "${3:-6}"
  local rc=$?
  if [ $rc -eq 0 ]; then
    echo "  >>> FAIL: $1 PASSED. The suite cannot see the defect class it exists for."
    bad=$((bad + 1))
  else
    echo "  >>> ok: $1 failed, which is what says the suite works"
  fi
  echo
}

echo "== ABI v1 obligation 12.5 =="
echo "nproc = $(nproc)"
echo

run_pass  conc_a17_shared 4 6
run_pass  conc_c11_shared 4 6     # the floor is thread-clean too, or it is not a floor
run_pass  conc_a17_static 4 6     # and the second linkage
# More threads than cores, so the scheduler preempts inside an encode rather than between
# encodes. A suite whose threads never interleave is a suite that runs N sequential tests.
run_pass  conc_a17_shared 16 4

run_plant conc_a17_pad    4 6
run_plant conc_a17_both   4 6

# AK_CONC_GLOBAL is NOT in the must-fail list, and that is the finding rather than an
# omission. A process-global width table is a data race and, per ABI v1 section 6, a
# throughput defect -- but it is not a BYTE defect on its own, because an unpadded prefix
# is rewritten to the width the body actually needs whatever the guess was. Section 6's two
# refusals are independent, and only the combination corrupts. So this build must PASS the
# byte suite, and what it costs shows up in T7's scaling instead.
echo "=========== conc_a17_global   (a REFUSED design that is byte-CLEAN: must PASS) ==========="
"$B/conc_a17_global" 4 6
rc=$?
if [ $rc -ne 0 ]; then echo "  >>> FAIL: conc_a17_global exited $rc"; bad=$((bad + 1));
else echo "  >>> ok: byte-clean, as predicted. Compare its T7 scaling with the shipped build's."; fi
echo

echo "concurrency: $bad build(s) did not do what they must"
exit $((bad ? 1 : 0))
