#!/usr/bin/env bash
# README 5.1's hard stop, checked mechanically, WITH its positive control.
#
# "In C++ the divergence must not reach the layout of an installed header type": the
# consumer picks -std and we do not, so a facade type whose layout depends on the standard
# level is an ODR violation waiting for a consumer who compiles at a different level than
# the library was built at.
#
# Pass 1 is the check. Pass 2 rebuilds it with -DAK_ODR_BREAK, which makes ak::Optional
# carry one extra member at C++17 only, and the check MUST fail. A guard with no failing
# test is a guard nobody has seen work.
set -u
cd "$(dirname "$0")/.." || exit 2
echo "== pass 1: the facade as it is =="
build/odrcheck
rc1=$?
echo "  exit $rc1 (0 = no installed type's layout moved with -std)"
echo
echo "== pass 2: the POSITIVE CONTROL, one type made -std dependent on purpose =="
rm -rf build-odrbreak && mkdir -p build-odrbreak
(cd build-odrbreak && cmake .. -G Ninja -DAK_ODR_BREAK=ON >/dev/null && ninja odrcheck >/dev/null 2>&1)
build-odrbreak/odrcheck
rc2=$?
echo "  exit $rc2 (nonzero = the check CAN fail, so pass 1 means something)"
rm -rf build-odrbreak
echo
if [ $rc1 -eq 0 ] && [ $rc2 -ne 0 ]; then
  echo "odr_check: ok"
  exit 0
fi
echo "odr_check: FAILED (pass1=$rc1 pass2=$rc2)"
exit 1
