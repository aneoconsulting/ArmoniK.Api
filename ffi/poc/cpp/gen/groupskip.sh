#!/usr/bin/env bash
# C24: `ak::Dec::skip` over the deprecated GROUP form, run.
#
# The shipped configuration at C++17 target, C++17 floor, C++14 floor and C++11 floor --
# README 5.2 says the floor is a correctness gate and a recursive skip written the wrong
# way is exactly the construct that compiles at one level and not another. Then the two
# PLANTS, each of which must FAIL, because a test that has only ever passed has not been
# seen working and this slice has lost two fixtures that way (C18, C23).
set -u
cd "$(dirname "$0")/.."
B=./build
bad=0

run_pass() {
  echo "=========== $1   (must PASS) ==========="
  "$B/$1"
  local rc=$?
  if [ $rc -ne 0 ]; then echo "  >>> FAIL: $1 exited $rc"; bad=$((bad + 1));
  else echo "  >>> ok: $1 passed"; fi
  echo
}

run_plant() {
  echo "=========== $1   (a PLANTED defect: must FAIL) ==========="
  echo "  what it plants: $2"
  "$B/$1"
  local rc=$?
  if [ $rc -eq 0 ]; then
    echo "  >>> FAIL: $1 PASSED. The test cannot see the defect class it exists for."
    bad=$((bad + 1))
  else
    echo "  >>> ok: $1 failed, which is what says the test works"
  fi
  echo
}

echo "== C24: the GROUP skip, every standard level and both implementations =="
echo

run_pass groupskip_a17
run_pass groupskip_b17
run_pass groupskip_c14
run_pass groupskip_c11

run_plant groupskip_plant_depth \
  "count nesting depth instead of matching the field number (accepts X-group-mismatched-end)"
run_plant groupskip_plant_i32 \
  "the case 5: 32-bit arm dropped while case 3: was added"

echo "groupskip: $bad build(s) did not do what they must"
exit $((bad ? 1 : 0))
