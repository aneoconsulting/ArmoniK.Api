#!/usr/bin/env bash
# ABI v1 obligation 12.5, run: the suite on the shipped design, and the planted builds that
# show the suite can fail.
#
# The plants are inverted here rather than in the binary. A fixture that exits 0 when it
# finds the defect it was built to find is a fixture nobody can run in a loop; a fixture
# that exits 0 when it does NOT find it is a fixture that silently stops working, which is
# how this slice's -flto control (C18) stopped working. So: shipped builds must pass, and
# each planted build must FAIL, and a planted build that passes fails this script.
#
# R-D7: failing is not enough. Until 2026-09-24 every planted build planted `ak::Enc` and
# linked the UNPLANTED core, so each one failed on the native arm alone and the ffi arm
# read 0 in all of them: the control never showed the ffi arm can fail. Each planted build
# now links the matching planted core, and this script reads the binary's per-encoder line
# ("whole run, distinct wrong operations: native-encoder N  core-encoder M  decoder D") and
# requires the encoder the plant is in to be the one that went wrong.
#
# B: the build directory (default ./build). AK_CONC_NO_T7=1: the suite's one timing is
# skipped, because in the setup phase a container timing is not a result (README 1.1).
set -u
cd "$(dirname "$0")/.."
B=${B:-./build}
export AK_CONC_NO_T7=${AK_CONC_NO_T7:-1}
bad=0
out=$(mktemp)
trap 'rm -f "$out"' EXIT

tot() {  # tot <native|core|dec>: the whole-run count from the last run's output
  local line
  line=$(grep '^whole run, distinct wrong operations:' "$out") || { echo -1; return; }
  case $1 in
    native) echo "$line" | sed -E 's/.*native-encoder ([0-9]+).*/\1/' ;;
    core)   echo "$line" | sed -E 's/.*core-encoder ([0-9]+).*/\1/' ;;
    dec)    echo "$line" | sed -E 's/.*decoder ([0-9]+).*/\1/' ;;
  esac
}

# expect <what> <actual> <op> <value>
expect() {
  if [ "$2" "$3" "$4" ]; then echo "  >>> ok: $1 = $2 ($3 $4)";
  else echo "  >>> FAIL: $1 = $2, required $3 $4"; bad=$((bad + 1)); fi
}

run() {  # run <exe> <threads> <rounds>; leaves output in $out and the exit code in $rc
  echo "$ ldd $B/$1 | grep ak_core"
  ldd "$B/$1" | grep ak_core | sed 's/ (0x[0-9a-f]*)//'
  "$B/$1" "$2" "$3" > "$out" 2>&1
  rc=$?
  cat "$out"
  echo "  exit $rc"
}

run_pass() {   # a build that must pass, with zero wrong operations on every encoder
  echo "=========== $1 $2 $3   (must PASS) ==========="
  run "$1" "$2" "$3"
  expect "$1 exit code" "$rc" -eq 0
  expect "$1 native-encoder wrong" "$(tot native)" -eq 0
  expect "$1 core-encoder wrong" "$(tot core)" -eq 0
  expect "$1 decoder wrong" "$(tot dec)" -eq 0
  echo
}

# run_plant <exe> <native: 0 or +> <core: 0 or +>: a planted build that must FAIL, and fail
# on the encoder(s) the plant is in and on no other.
run_plant() {
  echo "=========== $1 4 6   (a REFUSED design: must FAIL) ==========="
  run "$1" 4 6
  expect "$1 exit code (non-zero: the suite saw the plant)" "$rc" -ne 0
  if [ "$2" = + ]; then expect "$1 native-encoder wrong" "$(tot native)" -gt 0;
  else expect "$1 native-encoder wrong" "$(tot native)" -eq 0; fi
  if [ "$3" = + ]; then expect "$1 core-encoder wrong (the ffi arm CAN fail)" "$(tot core)" -gt 0;
  else expect "$1 core-encoder wrong" "$(tot core)" -eq 0; fi
  expect "$1 decoder wrong (no encode plant reaches the decoder)" "$(tot dec)" -eq 0
  echo
}

echo "== ABI v1 obligation 12.5 =="
echo "date: $(date -u +%FT%TZ)  nproc = $(nproc)  build dir: $B"
echo

run_pass  conc_a17_shared 4 6
run_pass  conc_c11_shared 4 6     # the floor is thread-clean too, or it is not a floor
run_pass  conc_a17_static 4 6     # and the second linkage
# More threads than cores, so the scheduler preempts inside an encode rather than between
# encodes. A suite whose threads never interleave is a suite that runs N sequential tests.
run_pass  conc_a17_shared 16 4

run_plant conc_a17_pad     + +    # ak::Enc AK_CONC_PAD, core --features pad-widths
run_plant conc_a17_both    + +    # both plants in both encoders
run_plant conc_a17_corepad 0 +    # the core planted ALONE: the ffi arm fails on its own

# The global table is NOT in the must-fail list, and that is the finding rather than an
# omission. A process-global width table is a data race and, per ABI v1 section 6, a
# throughput defect -- but it is not a BYTE defect on its own, because an unpadded prefix
# is rewritten to the width the body actually needs whatever the guess was. Section 6's two
# refusals are independent, and only the combination corrupts. So this build, planted in
# BOTH encoders now, must pass the byte suite.
echo "=========== conc_a17_global 4 6   (a REFUSED design that is byte-CLEAN: must PASS) ==========="
run conc_a17_global 4 6
expect "conc_a17_global exit code" "$rc" -eq 0
expect "conc_a17_global native-encoder wrong" "$(tot native)" -eq 0
expect "conc_a17_global core-encoder wrong" "$(tot core)" -eq 0
grep -q '^core plant: global-widths' "$out"
expect "conc_a17_global links the global-widths core (grep status)" "$?" -eq 0
echo

echo "concurrency: $bad expectation(s) not met"
exit $((bad ? 1 : 0))
